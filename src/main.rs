mod app;
mod config;
mod docker;
mod event;
mod ui;

use anyhow::Result;
use app::{App, AppAction};
use crossterm::{
    event::{EnableMouseCapture, DisableMouseCapture},
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

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }
    Ok(())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let docker = DockerClient::new()?;
    let mut app = App::new();
    let mut events = EventHandler::new(config::TICK_RATE_MS);

    // Load compose projects from current directory
    let compose_files = find_compose_files(Path::new("."));
    for path in &compose_files {
        if let Ok(project) = load_compose_project(path) {
            app.projects.push(project);
        }
    }

    // Initial data load
    app.containers = docker.list_containers().await.unwrap_or_default();
    app.images = docker.list_images().await.unwrap_or_default();
    app.volumes = docker.list_volumes().await.unwrap_or_default();

    // Track the container ID we're streaming for
    let mut streaming_id: Option<String> = None;

    // Start log/stats streams for first container
    type LogStream<'a> = std::pin::Pin<
        Box<
            dyn futures_util::Stream<
                    Item = Result<bollard::container::LogOutput, bollard::errors::Error>,
                > + Send
                + 'a,
        >,
    >;
    type StatsStream<'a> = std::pin::Pin<
        Box<
            dyn futures_util::Stream<
                    Item = Result<bollard::container::Stats, bollard::errors::Error>,
                > + Send
                + 'a,
        >,
    >;
    let mut log_stream: Option<LogStream<'_>> = None;
    let mut stats_stream: Option<StatsStream<'_>> = None;

    if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
        // Load initial logs in one batch, then start follow stream
        let initial_logs = docker.container_logs_batch(&id).await.unwrap_or_default();
        app.logs.insert(id.clone(), initial_logs);
        log_stream = Some(Box::pin(docker.container_logs_follow(&id)));
        stats_stream = Some(Box::pin(docker.container_stats(&id)));
        streaming_id = Some(id);
    }

    let mut tick_count: u64 = 0;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        tokio::select! {
            event = events.next() => {
                match event? {
                    AppEvent::Key(key) => {
                        let prev_selected = app.selected_index;
                        let action = app.handle_key(key);
                        match action {
                            AppAction::Quit => break,
                            AppAction::RestartContainer => {
                                if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
                                    let _ = docker.restart_container(&id).await;
                                    app.status_message = Some("Restarting...".to_string());
                                }
                            }
                            AppAction::StopContainer => {
                                if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
                                    let _ = docker.stop_container(&id).await;
                                    app.status_message = Some("Stopping...".to_string());
                                }
                            }
                            AppAction::StartContainer => {
                                if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
                                    let _ = docker.start_container(&id).await;
                                    app.status_message = Some("Starting...".to_string());
                                }
                            }
                            AppAction::RemoveContainer => {
                                if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
                                    let _ = docker.remove_container(&id).await;
                                    app.status_message = Some("Removing...".to_string());
                                }
                            }
                            AppAction::ExecShell => {
                                if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
                                    crossterm::terminal::disable_raw_mode()?;
                                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                                    let _ = std::process::Command::new("docker")
                                        .args(["exec", "-it", &id, "/bin/sh"])
                                        .status();
                                    crossterm::terminal::enable_raw_mode()?;
                                    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                                    terminal.clear()?;
                                }
                            }
                            AppAction::PruneImages => {
                                let _ = docker.prune_images().await;
                                app.images = docker.list_images().await.unwrap_or_default();
                                app.status_message = Some("Images pruned".to_string());
                            }
                            AppAction::PruneVolumes => {
                                let _ = docker.prune_volumes().await;
                                app.volumes = docker.list_volumes().await.unwrap_or_default();
                                app.status_message = Some("Volumes pruned".to_string());
                            }
                            _ => {}
                        }

                        // If selection changed, update streams
                        if prev_selected != app.selected_index {
                            if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
                                let initial = docker.container_logs_batch(&id).await.unwrap_or_default();
                                app.logs.insert(id.clone(), initial);
                                log_stream = Some(Box::pin(docker.container_logs_follow(&id)));
                                stats_stream = Some(Box::pin(docker.container_stats(&id)));
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
                                let initial = docker.container_logs_batch(&id).await.unwrap_or_default();
                                app.logs.insert(id.clone(), initial);
                                log_stream = Some(Box::pin(docker.container_logs_follow(&id)));
                                stats_stream = Some(Box::pin(docker.container_stats(&id)));
                                streaming_id = Some(id);
                            }
                        }
                    }
                    AppEvent::Tick => {
                        tick_count += 1;
                        if tick_count.is_multiple_of(10) {
                            app.containers = docker.list_containers().await.unwrap_or_default();
                        }
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
            Some(stats_result) = async {
                if let Some(ref mut stream) = stats_stream { stream.next().await } else { None }
            } => {
                if let Ok(stats) = stats_result {
                    let snapshot = parse_stats(&stats);
                    if let Some(ref id) = streaming_id {
                        let history = app.stats.entry(id.clone()).or_default();
                        history.push(snapshot.cpu_percent, snapshot.memory_mb, snapshot.memory_limit_mb, snapshot.net_rx_bytes, snapshot.net_tx_bytes);
                    }
                }
            }
        }

        if !app.running {
            break;
        }
    }
    Ok(())
}
