mod app;
mod config;
mod docker;
mod event;
mod ui;
mod update;

use anyhow::Result;
use app::AppAction;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use docker::client::DockerClient;
use docker::compose::{find_compose_files, load_compose_project};
use docker::stats::parse_stats;
use event::{AppEvent, EventHandler};
use futures_util::StreamExt;
use ratatui::prelude::*;
use std::io;
use std::path::Path;

#[derive(Parser)]
#[command(name = "rustydocker", version, about = "A fast, compose-first Docker TUI")]
struct Cli {
    /// Docker compose file path(s)
    #[arg(short = 'f', long = "file")]
    compose_file: Option<Vec<String>>,

    /// Docker compose project name
    #[arg(short = 'p', long = "project")]
    project_name: Option<String>,
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    let _ = execute!(io::stdout(), crossterm::cursor::Show);
}

#[tokio::main]
async fn main() -> Result<()> {
    // clap handles --version / --help before we touch the terminal.
    // This MUST be the first statement in main — otherwise terminal
    // setup runs on non-tty stdio and fails with ENXIO (os error 6),
    // and clap never gets to short-circuit on --version / --help.
    let cli = Cli::parse();

    // Restore terminal on panic so the shell stays usable
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, cli).await;

    restore_terminal();

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }
    Ok(())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, cli: Cli) -> Result<()> {
    let cfg = config::AppConfig::load();
    let tick_rate_ms = cfg.tick_rate_ms;
    let docker_host = std::env::var("DOCKER_HOST").ok().or_else(|| cfg.docker_host.clone());
    let docker = DockerClient::new(docker_host.as_deref())?;
    let mut app = app::App::new(cfg);
    app.docker_host = docker_host;
    let mut events = EventHandler::new(tick_rate_ms);

    // Update check channels. update_tx is consumed by the background
    // check spawned below; progress_tx is cloned into the spawn_blocking
    // task when the user actually triggers a self-update.
    let (update_tx, mut update_rx) =
        tokio::sync::mpsc::unbounded_channel::<update::UpdateCheckOutcome>();
    let (progress_tx, mut progress_rx) =
        tokio::sync::mpsc::unbounded_channel::<update::UpdateProgress>();

    let check_enabled = app.check_updates
        && std::env::var("RUSTYDOCKER_NO_UPDATE_CHECK").is_err();
    update::spawn_check(env!("CARGO_PKG_VERSION"), check_enabled, update_tx);

    // Load compose projects from CLI flag or current directory
    let compose_files = if let Some(ref files) = cli.compose_file {
        files
            .iter()
            .map(std::path::PathBuf::from)
            .collect::<Vec<_>>()
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    } else {
        find_compose_files(Path::new("."))
    };
    for path in &compose_files {
        if let Ok(mut project) = load_compose_project(path) {
            if let Some(ref name) = cli.project_name {
                project.name = name.clone();
            }
            app.projects.push(project);
        }
    }

    // Initial data load
    app.containers = docker.list_containers().await.unwrap_or_default();
    app.sort_containers();
    app.images = docker.list_images().await.unwrap_or_default();
    app.volumes = docker.list_volumes().await.unwrap_or_default();
    app.networks = docker.list_networks().await.unwrap_or_default();

    // Track the container ID we're streaming for
    let mut streaming_id: Option<String> = None;

    // Start log/stats streams for first container
    type LogStream<'a> = std::pin::Pin<
        Box<dyn futures_util::Stream<Item = Result<bollard::container::LogOutput, bollard::errors::Error>> + Send + 'a>,
    >;
    let mut log_stream: Option<LogStream<'_>> = None;

    if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
        // Load initial logs in one batch, then start follow stream
        let initial_logs = docker
            .container_logs_batch(&id, &app.log_tail_lines)
            .await
            .unwrap_or_default();
        app.logs.insert(id.clone(), initial_logs);
        app.log_bookmarks.clear();
        log_stream = Some(Box::pin(docker.container_logs_follow(&id)));
        if let Ok(inspect) = docker.inspect_container(&id).await {
            let env_vars: Vec<(String, String)> = inspect
                .config
                .as_ref()
                .and_then(|c| c.env.as_ref())
                .map(|envs| {
                    envs.iter()
                        .filter_map(|e| e.split_once('=').map(|(k, v)| (k.to_string(), v.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            app.container_env = Some(env_vars);
            app.container_inspect = Some(inspect);
        } else {
            app.container_env = None;
            app.container_inspect = None;
        }
        if let Ok(top) = docker.container_top(&id).await {
            app.container_top = Some(top);
        } else {
            app.container_top = None;
        }
        streaming_id = Some(id);
    }

    // Fetch initial stats for all running containers
    for container in &app.containers {
        if container.state.as_deref() == Some("running") {
            if let Some(id) = container.id.as_deref() {
                if let Ok(stats) = docker.container_stats_oneshot(id).await {
                    let snapshot = parse_stats(&stats);
                    let history = app.stats.entry(id.to_string()).or_default();
                    history.push(
                        snapshot.cpu_percent,
                        snapshot.memory_mb,
                        snapshot.memory_limit_mb,
                        snapshot.net_rx_bytes,
                        snapshot.net_tx_bytes,
                    );
                }
            }
        }
    }

    let mut docker_event_stream = Box::pin(docker.docker_events());

    let mut tick_count: u64 = 0;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        tokio::select! {
            event = events.next() => {
                match event? {
                    AppEvent::Key(key) => {
                        let prev_selected = app.selected_index;
                        let prev_section = app.sidebar_section;
                        let action = app.handle_key(key);
                        match action {
                            AppAction::Quit => break,
                            AppAction::RestartContainer => {
                                let targets = app.target_container_ids();
                                for id in &targets { let _ = docker.restart_container(id).await.map_err(|e| app.set_status(&format!("Error: {}", e))); }
                                if !targets.is_empty() { app.set_status(&format!("Restarting {} container(s)...", targets.len())); app.selected_containers.clear(); }
                            }
                            AppAction::StopContainer => {
                                let targets = app.target_container_ids();
                                for id in &targets { let _ = docker.stop_container(id).await.map_err(|e| app.set_status(&format!("Error: {}", e))); }
                                if !targets.is_empty() { app.set_status(&format!("Stopping {} container(s)...", targets.len())); app.selected_containers.clear(); }
                            }
                            AppAction::StartContainer => {
                                let targets = app.target_container_ids();
                                for id in &targets { let _ = docker.start_container(id).await.map_err(|e| app.set_status(&format!("Error: {}", e))); }
                                if !targets.is_empty() { app.set_status(&format!("Starting {} container(s)...", targets.len())); app.selected_containers.clear(); }
                            }
                            AppAction::PauseContainer => {
                                let targets = app.target_container_ids();
                                for id in &targets { let _ = docker.pause_container(id).await.map_err(|e| app.set_status(&format!("Error: {}", e))); }
                                if !targets.is_empty() { app.set_status(&format!("Pausing {} container(s)...", targets.len())); app.selected_containers.clear(); }
                            }
                            AppAction::UnpauseContainer => {
                                let targets = app.target_container_ids();
                                for id in &targets { let _ = docker.unpause_container(id).await.map_err(|e| app.set_status(&format!("Error: {}", e))); }
                                if !targets.is_empty() { app.set_status(&format!("Unpausing {} container(s)...", targets.len())); app.selected_containers.clear(); }
                            }
                            AppAction::RemoveContainer => {
                                let targets = app.target_container_ids();
                                for id in &targets { let _ = docker.remove_container(id).await.map_err(|e| app.set_status(&format!("Error: {}", e))); }
                                if !targets.is_empty() { app.set_status(&format!("Removing {} container(s)...", targets.len())); app.selected_containers.clear(); }
                            }
                            AppAction::ExecShell => {
                                if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
                                    crossterm::terminal::disable_raw_mode()?;
                                    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                                    let _ = std::process::Command::new("docker")
                                        .args(["exec", "-it", &id, "/bin/sh"])
                                        .status();
                                    crossterm::terminal::enable_raw_mode()?;
                                    execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
                                    terminal.clear()?;
                                }
                            }
                            AppAction::AttachContainer => {
                                if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
                                    crossterm::terminal::disable_raw_mode()?;
                                    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                                    let _ = std::process::Command::new("docker")
                                        .args(["attach", &id])
                                        .status();
                                    crossterm::terminal::enable_raw_mode()?;
                                    execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
                                    terminal.clear()?;
                                }
                            }
                            AppAction::PruneImages => {
                                match docker.prune_images().await {
                                    Ok(_) => app.set_status("Images pruned"),
                                    Err(e) => app.set_status(&format!("Error: {}", e)),
                                }
                                app.images = docker.list_images().await.unwrap_or_default();
                            }
                            AppAction::PruneVolumes => {
                                match docker.prune_volumes().await {
                                    Ok(_) => app.set_status("Volumes pruned"),
                                    Err(e) => app.set_status(&format!("Error: {}", e)),
                                }
                                app.volumes = docker.list_volumes().await.unwrap_or_default();
                            }
                            AppAction::ComposeUp => {
                                let status = std::process::Command::new("docker")
                                    .args(["compose", "up", "-d"])
                                    .output();
                                match status {
                                    Ok(output) if output.status.success() => app.set_status("Compose up started"),
                                    Ok(output) => app.set_status(&format!("Error: {}", String::from_utf8_lossy(&output.stderr).trim())),
                                    Err(e) => app.set_status(&format!("Error: {}", e)),
                                }
                            }
                            AppAction::ComposeDown => {
                                let status = std::process::Command::new("docker")
                                    .args(["compose", "down"])
                                    .output();
                                match status {
                                    Ok(output) if output.status.success() => app.set_status("Compose down complete"),
                                    Ok(output) => app.set_status(&format!("Error: {}", String::from_utf8_lossy(&output.stderr).trim())),
                                    Err(e) => app.set_status(&format!("Error: {}", e)),
                                }
                            }
                            AppAction::ComposeRestart => {
                                let status = std::process::Command::new("docker")
                                    .args(["compose", "restart"])
                                    .output();
                                match status {
                                    Ok(output) if output.status.success() => app.set_status("Compose restart complete"),
                                    Ok(output) => app.set_status(&format!("Error: {}", String::from_utf8_lossy(&output.stderr).trim())),
                                    Err(e) => app.set_status(&format!("Error: {}", e)),
                                }
                            }
                            AppAction::StopAllContainers => {
                                for c in &app.containers {
                                    if c.state.as_deref() == Some("running") {
                                        if let Some(id) = &c.id {
                                            let _ = docker.stop_container(id).await;
                                        }
                                    }
                                }
                                app.set_status("Stopping all containers...");
                            }
                            AppAction::RemoveStoppedContainers => {
                                for c in &app.containers {
                                    if c.state.as_deref() == Some("exited") {
                                        if let Some(id) = &c.id {
                                            let _ = docker.remove_container(id).await;
                                        }
                                    }
                                }
                                app.set_status("Removed stopped containers");
                            }
                            AppAction::PruneContainers => {
                                let _ = std::process::Command::new("docker")
                                    .args(["container", "prune", "-f"])
                                    .output();
                                app.set_status("Containers pruned");
                            }
                            AppAction::PruneNetworks => {
                                match docker.prune_networks().await {
                                    Ok(_) => app.set_status("Networks pruned"),
                                    Err(e) => app.set_status(&format!("Error: {}", e)),
                                }
                                app.networks = docker.list_networks().await.unwrap_or_default();
                            }
                            AppAction::ExportLogs => {
                                if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
                                    let name = app.selected_container()
                                        .and_then(|c| c.names.as_ref())
                                        .and_then(|n| n.first())
                                        .map(|n| n.trim_start_matches('/').to_string())
                                        .unwrap_or_else(|| id[..12.min(id.len())].to_string());
                                    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                                    let filename = format!("{}_{}.log", name, timestamp);
                                    if let Some(logs) = app.logs.get(&id) {
                                        match std::fs::write(&filename, logs.join("\n")) {
                                            Ok(_) => app.set_status(&format!("Logs saved to {}", filename)),
                                            Err(e) => app.set_status(&format!("Error: {}", e)),
                                        }
                                    } else {
                                        app.set_status("No logs to export");
                                    }
                                }
                            }
                            AppAction::OpenInBrowser => {
                                if let Some(container) = app.selected_container() {
                                    if let Some(ports) = &container.ports {
                                        if let Some(port) = ports.iter().find(|p| p.public_port.is_some()) {
                                            let public_port = port.public_port.unwrap();
                                            let url = format!("http://localhost:{}", public_port);
                                            #[cfg(target_os = "linux")]
                                            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                                            #[cfg(target_os = "macos")]
                                            let _ = std::process::Command::new("open").arg(&url).spawn();
                                            app.set_status(&format!("Opening {}...", url));
                                        } else {
                                            app.set_status("No public ports found");
                                        }
                                    } else {
                                        app.set_status("No ports exposed");
                                    }
                                }
                            }
                            AppAction::RunCustomCommand(idx) => {
                                if let Some(cmd) = app.custom_commands.get(idx) {
                                    let command_str = if let Some(id) = app.selected_container_id() {
                                        cmd.command.replace("{container_id}", id)
                                            .replace("{container_name}",
                                                app.selected_container()
                                                    .and_then(|c| c.names.as_ref())
                                                    .and_then(|n| n.first())
                                                    .map(|n| n.trim_start_matches('/'))
                                                    .unwrap_or(""))
                                    } else {
                                        cmd.command.clone()
                                    };

                                    if cmd.attach {
                                        crossterm::terminal::disable_raw_mode()?;
                                        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                                        let _ = std::process::Command::new("sh")
                                            .args(["-c", &command_str])
                                            .status();
                                        crossterm::terminal::enable_raw_mode()?;
                                        execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
                                        terminal.clear()?;
                                    } else {
                                        match std::process::Command::new("sh")
                                            .args(["-c", &command_str])
                                            .output()
                                        {
                                            Ok(output) if output.status.success() => {
                                                app.set_status(&format!("Command '{}' completed", cmd.name));
                                            }
                                            Ok(output) => {
                                                app.set_status(&format!("Error: {}", String::from_utf8_lossy(&output.stderr).trim()));
                                            }
                                            Err(e) => app.set_status(&format!("Error: {}", e)),
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }

                        // If selection changed, update streams
                        if prev_selected != app.selected_index || prev_section != app.sidebar_section {
                            if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
                                let initial = docker.container_logs_batch(&id, &app.log_tail_lines).await.unwrap_or_default();
                                app.logs.insert(id.clone(), initial);
                                app.log_bookmarks.clear();
                                log_stream = Some(Box::pin(docker.container_logs_follow(&id)));
                                if let Ok(inspect) = docker.inspect_container(&id).await {
                                    let env_vars: Vec<(String, String)> = inspect.config.as_ref()
                                        .and_then(|c| c.env.as_ref())
                                        .map(|envs| envs.iter().filter_map(|e| e.split_once('=').map(|(k, v)| (k.to_string(), v.to_string()))).collect())
                                        .unwrap_or_default();
                                    app.container_env = Some(env_vars);
                                    app.container_inspect = Some(inspect);
                                } else {
                                    app.container_env = None;
                                    app.container_inspect = None;
                                }
                                if let Ok(top) = docker.container_top(&id).await {
                                    app.container_top = Some(top);
                                } else {
                                    app.container_top = None;
                                }
                                streaming_id = Some(id);
                            }
                        }
                    }
                    AppEvent::Mouse(mouse) => {
                        let prev_selected = app.selected_index;
                        let prev_section = app.sidebar_section;
                        let size = terminal.size()?;
                        let rect = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                        app.handle_mouse(mouse, rect);

                        // If container selection changed, update streams
                        if (prev_selected != app.selected_index || prev_section != app.sidebar_section)
                            && app.sidebar_section == app::SidebarSection::Services
                        {
                            if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
                                let initial = docker.container_logs_batch(&id, &app.log_tail_lines).await.unwrap_or_default();
                                app.logs.insert(id.clone(), initial);
                                app.log_bookmarks.clear();
                                log_stream = Some(Box::pin(docker.container_logs_follow(&id)));
                                if let Ok(inspect) = docker.inspect_container(&id).await {
                                    let env_vars: Vec<(String, String)> = inspect.config.as_ref()
                                        .and_then(|c| c.env.as_ref())
                                        .map(|envs| envs.iter().filter_map(|e| e.split_once('=').map(|(k, v)| (k.to_string(), v.to_string()))).collect())
                                        .unwrap_or_default();
                                    app.container_env = Some(env_vars);
                                    app.container_inspect = Some(inspect);
                                } else {
                                    app.container_env = None;
                                    app.container_inspect = None;
                                }
                                if let Ok(top) = docker.container_top(&id).await {
                                    app.container_top = Some(top);
                                } else {
                                    app.container_top = None;
                                }
                                streaming_id = Some(id);
                            }
                        }
                    }
                    AppEvent::Tick => {
                        app.clear_expired_status();
                        tick_count += 1;
                        if tick_count % 40 == 0 {
                            let prev_id = app.selected_container_id().map(|s| s.to_string());
                            app.containers = docker.list_containers().await.unwrap_or_default();
                            app.sort_containers();
                            app.clamp_selected_index();
                            app.prune_stale_selections();
                            app.networks = docker.list_networks().await.unwrap_or_default();
                            // If selected container changed after refresh, update streams
                            let new_id = app.selected_container_id().map(|s| s.to_string());
                            if new_id != prev_id {
                                if let Some(ref id) = new_id {
                                    let initial = docker.container_logs_batch(id, &app.log_tail_lines).await.unwrap_or_default();
                                    app.logs.insert(id.clone(), initial);
                                    app.log_bookmarks.clear();
                                    log_stream = Some(Box::pin(docker.container_logs_follow(id)));
                                    streaming_id = new_id;
                                }
                            }
                        }
                        if tick_count % 8 == 0 {
                            // Collect stats for all running containers
                            for container in &app.containers {
                                if container.state.as_deref() == Some("running") {
                                    if let Some(id) = container.id.as_deref() {
                                        if let Ok(stats) = docker.container_stats_oneshot(id).await {
                                            let snapshot = parse_stats(&stats);
                                            let history = app.stats.entry(id.to_string()).or_default();
                                            history.push(snapshot.cpu_percent, snapshot.memory_mb, snapshot.memory_limit_mb, snapshot.net_rx_bytes, snapshot.net_tx_bytes);
                                        }
                                    }
                                }
                            }
                            // Also refresh top data for selected container
                            if app.active_tab == app::Tab::Top {
                                if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
                                    if let Ok(top) = docker.container_top(&id).await {
                                        app.container_top = Some(top);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Some(event_result) = docker_event_stream.next() => {
                if let Ok(event) = event_result {
                    use bollard::models::EventMessageTypeEnum;

                    // Get event details for hook substitution
                    let event_action = event.action.as_deref().unwrap_or("");
                    let actor_id = event.actor.as_ref()
                        .and_then(|a| a.id.as_deref())
                        .unwrap_or("");
                    let actor_name = event.actor.as_ref()
                        .and_then(|a| a.attributes.as_ref())
                        .and_then(|attrs| attrs.get("name"))
                        .map(|s| s.as_str())
                        .unwrap_or("");

                    // Map Docker event action to hook event name
                    let hook_event = match (&event.typ, event_action) {
                        (Some(EventMessageTypeEnum::CONTAINER), "start") => Some("container_start"),
                        (Some(EventMessageTypeEnum::CONTAINER), "stop") => Some("container_stop"),
                        (Some(EventMessageTypeEnum::CONTAINER), "die") => Some("container_die"),
                        (Some(EventMessageTypeEnum::CONTAINER), "restart") => Some("container_restart"),
                        (Some(EventMessageTypeEnum::IMAGE), "pull") => Some("image_pull"),
                        _ => None,
                    };

                    // Run matching hooks in background
                    if let Some(hook_name) = hook_event {
                        for hook in &app.hooks {
                            if hook.event == hook_name {
                                let cmd = hook.command
                                    .replace("{container_id}", actor_id)
                                    .replace("{container_name}", actor_name);
                                tokio::spawn(async move {
                                    let _ = tokio::process::Command::new("sh")
                                        .args(["-c", &cmd])
                                        .output()
                                        .await;
                                });
                            }
                        }
                    }

                    match event.typ {
                        Some(EventMessageTypeEnum::CONTAINER) => {
                            app.containers = docker.list_containers().await.unwrap_or_default();
                            app.sort_containers();
                            app.clamp_selected_index();
                            app.prune_stale_selections();
                        }
                        Some(EventMessageTypeEnum::IMAGE) => {
                            app.images = docker.list_images().await.unwrap_or_default();
                        }
                        Some(EventMessageTypeEnum::VOLUME) => {
                            app.volumes = docker.list_volumes().await.unwrap_or_default();
                        }
                        Some(EventMessageTypeEnum::NETWORK) => {
                            app.networks = docker.list_networks().await.unwrap_or_default();
                        }
                        _ => {}
                    }
                }
            }
            Some(log_result) = async {
                if let Some(ref mut stream) = log_stream { stream.next().await } else { None }
            } => {
                if let Ok(output) = log_result {
                    if let Some(ref id) = streaming_id {
                        let entry = app.logs.entry(id.clone()).or_default();
                        entry.push(output.to_string());
                        // Drain all buffered log lines in one go (no re-render per line)
                        while let Some(more) = {
                            use futures_util::FutureExt;
                            if let Some(ref mut s) = log_stream {
                                s.next().now_or_never().flatten()
                            } else {
                                None
                            }
                        } {
                            if let Ok(out) = more {
                                entry.push(out.to_string());
                            }
                        }
                        if entry.len() > 1000 { entry.drain(..entry.len() - 1000); }
                    }
                }
            }
            Some(outcome) = update_rx.recv() => {
                if let update::UpdateCheckOutcome::Available { version, self_updatable } = outcome {
                    app.update_available = Some(app::UpdateInfo { version, self_updatable });
                }
            }
            Some(progress) = progress_rx.recv() => {
                use update::UpdateProgress::*;
                app.update_flow = match progress {
                    Downloading(p) => app::UpdateFlow::Downloading(p),
                    Installing     => app::UpdateFlow::Installing,
                    Done           => app::UpdateFlow::Complete,
                    Failed(msg)    => app::UpdateFlow::Failed(msg),
                };
            }
        }

        if !app.running {
            break;
        }
    }
    Ok(())
}
