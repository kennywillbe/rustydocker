use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ratatui::prelude::*;
use rustydocker::app::{self, AppAction};
use rustydocker::config;
use rustydocker::docker::client::DockerClient;
use rustydocker::docker::compose::{
    find_compose_files, load_compose_project, load_effective_project, ComposeInvocation,
};
use rustydocker::docker::connection;
use rustydocker::docker::model::container_state;
use rustydocker::docker::stats::parse_stats;
use rustydocker::event::{AppEvent, EventHandler};
use rustydocker::runtime::actions::ActionExecutor;
use rustydocker::runtime::details::refresh_selected;
use rustydocker::runtime::images::refresh_selected_image;
use rustydocker::runtime::logs::{LogEvent, LogManager};
use rustydocker::{ui, update};
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

    /// Docker context (overrides DOCKER_CONTEXT and DOCKER_HOST)
    #[arg(long = "context")]
    docker_context: Option<String>,

    /// Enable a Docker Compose profile (repeatable)
    #[arg(long = "profile")]
    compose_profiles: Vec<String>,
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
    let resolved_connection = connection::resolve(cli.docker_context.as_deref(), &cfg)?;
    let docker_host = resolved_connection.spec.host().map(str::to_owned);
    let docker = DockerClient::new(&resolved_connection.spec)?;
    let compose_profiles = if cli.compose_profiles.is_empty() {
        cfg.compose_profiles.clone()
    } else {
        cli.compose_profiles.clone()
    };
    let compose_invocation = ComposeInvocation::new(
        cli.compose_file.clone().unwrap_or_default(),
        cli.project_name.clone(),
        docker_host.clone(),
    )
    .with_context(resolved_connection.context.clone())
    .with_profiles(compose_profiles)
    .with_tls_cert_path(match &resolved_connection.spec {
        connection::ConnectionSpec::Tls { cert_path, .. } => Some(cert_path.clone()),
        _ => None,
    });
    let mut app = app::App::new(cfg);
    app.docker_host = docker_host;
    let mut events = EventHandler::new(tick_rate_ms);

    // Update check channels. update_tx is consumed by the background
    // check spawned below; progress_tx is cloned into the spawn_blocking
    // task when the user actually triggers a self-update.
    let (update_tx, mut update_rx) = tokio::sync::mpsc::unbounded_channel::<update::UpdateCheckOutcome>();
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel::<update::UpdateProgress>();

    let check_enabled = app.check_updates && std::env::var("RUSTYDOCKER_NO_UPDATE_CHECK").is_err();
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
    match load_effective_project(&compose_invocation).await {
        Ok(project) if !project.services.is_empty() => app.projects.push(project),
        Ok(_) => {}
        Err(error) => {
            // Keep the TUI useful when Compose is not installed or the current
            // directory has no project, while making the degraded model visible.
            for path in &compose_files {
                if let Ok(mut project) = load_compose_project(path) {
                    if let Some(ref name) = cli.project_name {
                        project.name = name.clone();
                    }
                    app.projects.push(project);
                }
            }
            if !compose_files.is_empty() {
                app.set_status(&format!("Compose config fallback: {error}"));
            }
        }
    }

    // Initial data load
    app.containers = docker.list_containers().await.unwrap_or_default();
    app.sort_containers();
    app.images = docker.list_images().await.unwrap_or_default();
    app.volumes = docker.list_volumes().await.unwrap_or_default();
    app.networks = docker.list_networks().await.unwrap_or_default();

    let mut log_manager = LogManager::default();
    log_manager.sync(&docker, &app.containers, &app.log_tail_lines);
    refresh_selected(&mut app, &docker, false).await;
    let mut action_executor = ActionExecutor::new(compose_invocation.clone());

    // Fetch initial stats for all running containers
    for container in &app.containers {
        if container_state(container) == Some("running") {
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
                                    let mut command = std::process::Command::new("docker");
                                    resolved_connection.spec.configure_cli(&mut command, resolved_connection.context.as_deref());
                                    let _ = command.args(["exec", "-it", &id, "/bin/sh"]).status();
                                    crossterm::terminal::enable_raw_mode()?;
                                    execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
                                    terminal.clear()?;
                                }
                            }
                            AppAction::AttachContainer => {
                                if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
                                    crossterm::terminal::disable_raw_mode()?;
                                    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                                    let mut command = std::process::Command::new("docker");
                                    resolved_connection.spec.configure_cli(&mut command, resolved_connection.context.as_deref());
                                    let _ = command.args(["attach", &id]).status();
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
                            AppAction::PullImage(reference) => {
                                app.set_status(&format!("Pulling {reference}..."));
                                action_executor.spawn_image_pull(docker.clone(), reference);
                            }
                            AppAction::RemoveImage => {
                                if let Some(reference) = app.selected_image_reference().map(str::to_owned) {
                                    match docker.remove_image(&reference, false).await {
                                        Ok(()) => app.set_status(&format!("Removed {reference}")),
                                        Err(error) => app.set_status(&format!("Error: {error}")),
                                    }
                                    app.images = docker.list_images().await.unwrap_or_default();
                                    app.clamp_selected_index();
                                    refresh_selected_image(&mut app, &docker).await;
                                }
                            }
                            AppAction::ComposeUp => {
                                app.set_status("Starting Compose project...");
                                action_executor.spawn_compose(vec!["up", "-d"], "Compose up complete", false);
                            }
                            AppAction::ComposeDown => {
                                app.set_status("Stopping Compose project...");
                                action_executor.spawn_compose(vec!["down"], "Compose down complete", false);
                            }
                            AppAction::ComposeRestart => {
                                app.set_status("Restarting Compose project...");
                                action_executor.spawn_compose(vec!["restart"], "Compose restart complete", false);
                            }
                            AppAction::ComposePull => {
                                app.set_status("Pulling Compose images...");
                                action_executor.spawn_compose(vec!["pull"], "Compose images pulled", true);
                            }
                            AppAction::ComposeRebuild => {
                                app.set_status("Rebuilding Compose project...");
                                action_executor.spawn_compose(
                                    vec!["up", "-d", "--build"],
                                    "Compose rebuild complete",
                                    true,
                                );
                            }
                            AppAction::ComposeWatchToggle => {
                                match action_executor.toggle_compose_watch().await {
                                    Ok(true) => app.set_status("Compose watch started"),
                                    Ok(false) => app.set_status("Compose watch stopped"),
                                    Err(error) => app.set_status(&format!("Error: {error}")),
                                }
                            }
                            AppAction::StopAllContainers => {
                                for c in &app.containers {
                                    if container_state(c) == Some("running") {
                                        if let Some(id) = &c.id {
                                            let _ = docker.stop_container(id).await;
                                        }
                                    }
                                }
                                app.set_status("Stopping all containers...");
                            }
                            AppAction::RemoveStoppedContainers => {
                                for c in &app.containers {
                                    if container_state(c) == Some("exited") {
                                        if let Some(id) = &c.id {
                                            let _ = docker.remove_container(id).await;
                                        }
                                    }
                                }
                                app.set_status("Removed stopped containers");
                            }
                            AppAction::PruneContainers => {
                                let mut command = std::process::Command::new("docker");
                                resolved_connection.spec.configure_cli(&mut command, resolved_connection.context.as_deref());
                                let _ = command.args(["container", "prune", "-f"]).output();
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
                            AppAction::RequestUpdateCheck => {
                                match (&app.update_available, &app.update_flow) {
                                    (Some(info), app::UpdateFlow::Idle) if info.self_updatable => {
                                        app.update_flow = app::UpdateFlow::Confirming;
                                    }
                                    (Some(_), app::UpdateFlow::Idle) => {
                                        app.set_status("Update available — install via your package manager");
                                    }
                                    (None, app::UpdateFlow::Idle) => {
                                        app.set_status(&format!(
                                            "You're on the latest version ({})",
                                            env!("CARGO_PKG_VERSION")
                                        ));
                                    }
                                    (_, app::UpdateFlow::InstalledPendingRestart) => {
                                        app.set_status("Update already installed — restart rustydocker to apply");
                                    }
                                    _ => {} // already in a transient flow state — ignore
                                }
                            }
                            AppAction::ConfirmUpdate => {
                                if let Some(info) = app.update_available.clone() {
                                    app.update_flow = app::UpdateFlow::Downloading(0);
                                    let ptx = progress_tx.clone();
                                    let version = info.version.clone();
                                    tokio::task::spawn_blocking(move || {
                                        update::run_self_update(&version, ptx);
                                    });
                                }
                            }
                            AppAction::CancelUpdate => {
                                app.update_flow = app::UpdateFlow::Idle;
                            }
                            AppAction::DismissAfterUpdate => {
                                app.update_flow = app::UpdateFlow::InstalledPendingRestart;
                            }
                            AppAction::RestartAfterUpdate => {
                                use std::os::unix::process::CommandExt;
                                restore_terminal();
                                let exe = std::env::current_exe()?;
                                let err = std::process::Command::new(exe)
                                    .args(std::env::args().skip(1))
                                    .exec();
                                // exec() only returns on failure.
                                return Err(err.into());
                            }
                            _ => {}
                        }

                        // Refresh details through the runtime adapter when selection changes.
                        if prev_selected != app.selected_index || prev_section != app.sidebar_section {
                            if app.sidebar_section == app::SidebarSection::Images {
                                refresh_selected_image(&mut app, &docker).await;
                            } else {
                                refresh_selected(&mut app, &docker, true).await;
                            }
                        }
                    }
                    AppEvent::Mouse(mouse) => {
                        let prev_selected = app.selected_index;
                        let prev_section = app.sidebar_section;
                        let size = terminal.size()?;
                        let rect = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                        app.handle_mouse(mouse, rect);

                        // If container selection changed, refresh its details.
                        if prev_selected != app.selected_index || prev_section != app.sidebar_section {
                            if app.sidebar_section == app::SidebarSection::Images {
                                refresh_selected_image(&mut app, &docker).await;
                            } else {
                                refresh_selected(&mut app, &docker, true).await;
                            }
                        }
                    }
                    AppEvent::Tick => {
                        app.clear_expired_status();
                        tick_count += 1;
                        if matches!(action_executor.poll_compose_watch(), Ok(Some(false))) {
                            app.set_status("Compose watch exited");
                        }
                        if tick_count.is_multiple_of(40) {
                            let prev_id = app.selected_container_id().map(|s| s.to_string());
                            app.containers = docker.list_containers().await.unwrap_or_default();
                            app.sort_containers();
                            app.clamp_selected_index();
                            app.prune_stale_selections();
                            app.networks = docker.list_networks().await.unwrap_or_default();
                            log_manager.sync(&docker, &app.containers, &app.log_tail_lines);
                            // If selected container changed after refresh, update details.
                            let new_id = app.selected_container_id().map(|s| s.to_string());
                            if new_id != prev_id {
                                refresh_selected(&mut app, &docker, true).await;
                            }
                        }
                        if tick_count.is_multiple_of(8) {
                            // Collect stats for all running containers
                            for container in &app.containers {
                                if container_state(container) == Some("running") {
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
                            log_manager.sync(&docker, &app.containers, &app.log_tail_lines);
                        }
                        Some(EventMessageTypeEnum::IMAGE) => {
                            app.images = docker.list_images().await.unwrap_or_default();
                            if app.sidebar_section == app::SidebarSection::Images {
                                app.clamp_selected_index();
                                refresh_selected_image(&mut app, &docker).await;
                            }
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
            Some(log_event) = log_manager.recv() => {
                match log_event {
                    LogEvent::Line { container_id, container_name, line } => {
                        app.push_log(container_id, container_name, line);
                    }
                    LogEvent::StreamError { container_id, message } => {
                        let short_id: String = container_id.chars().take(12).collect();
                        app.set_status(&format!("Log stream {short_id}: {message}"));
                    }
                }
            }
            Some(outcome) = action_executor.recv() => {
                match outcome {
                    Ok(outcome) => {
                        app.set_status(&outcome.message);
                        if outcome.refresh_images {
                            app.images = docker.list_images().await.unwrap_or_default();
                            app.clamp_selected_index();
                            if app.sidebar_section == app::SidebarSection::Images {
                                refresh_selected_image(&mut app, &docker).await;
                            }
                        }
                    }
                    Err(error) => app.set_status(&format!("Error: {error}")),
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
    action_executor.shutdown().await;
    Ok(())
}
