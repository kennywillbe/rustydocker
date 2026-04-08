use crate::app::App;
use ratatui::layout::Constraint;
use ratatui::prelude::*;
use ratatui::widgets::{Cell, Paragraph, Row, Table};

pub fn render_info(f: &mut Frame, area: Rect, app: &App) {
    let container = match app.selected_container() {
        Some(c) => c,
        None => {
            f.render_widget(
                Paragraph::new("No container selected")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let key_style = Style::default().fg(Color::Cyan);
    let val_style = Style::default().fg(Color::White);
    let secondary_style = Style::default().fg(Color::DarkGray);

    let mut rows: Vec<Row> = Vec::new();

    // Name
    let names = container
        .names
        .as_ref()
        .map(|n| {
            n.iter()
                .map(|s| s.trim_start_matches('/'))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "N/A".to_string());
    rows.push(Row::new(vec![
        Cell::from(" Name").style(key_style),
        Cell::from(names).style(val_style.add_modifier(Modifier::BOLD)),
    ]));

    // ID
    let id = container
        .id
        .as_deref()
        .unwrap_or("N/A")
        .chars()
        .take(12)
        .collect::<String>();
    rows.push(Row::new(vec![
        Cell::from(" ID").style(key_style),
        Cell::from(id).style(val_style),
    ]));

    // Image
    let image = container.image.as_deref().unwrap_or("N/A");
    rows.push(Row::new(vec![
        Cell::from(" Image").style(key_style),
        Cell::from(image.to_string()).style(val_style),
    ]));

    // State
    let state = container.state.as_deref().unwrap_or("N/A");
    let state_color = match state {
        "running" => Color::Green,
        "exited" => Color::Red,
        "restarting" => Color::Yellow,
        "paused" => Color::Yellow,
        _ => Color::DarkGray,
    };
    rows.push(Row::new(vec![
        Cell::from(" State").style(key_style),
        Cell::from(Span::styled(
            state.to_string(),
            Style::default().fg(state_color).add_modifier(Modifier::BOLD),
        )),
    ]));

    // Status
    let status = container.status.as_deref().unwrap_or("N/A");
    rows.push(Row::new(vec![
        Cell::from(" Status").style(key_style),
        Cell::from(status.to_string()).style(val_style),
    ]));

    // Fields from inspect data
    if let Some(ref inspect) = app.container_inspect {
        // Command
        if let Some(ref config) = inspect.config {
            let cmd = config.cmd.as_ref().map(|c| c.join(" ")).unwrap_or_default();
            if !cmd.is_empty() {
                rows.push(Row::new(vec![
                    Cell::from(" Command").style(key_style),
                    Cell::from(cmd).style(val_style),
                ]));
            }

            // Entrypoint
            let entrypoint = config.entrypoint.as_ref().map(|e| e.join(" ")).unwrap_or_default();
            if !entrypoint.is_empty() {
                rows.push(Row::new(vec![
                    Cell::from(" Entrypoint").style(key_style),
                    Cell::from(entrypoint).style(val_style),
                ]));
            }

            // Working Dir
            if let Some(ref wd) = config.working_dir {
                if !wd.is_empty() {
                    rows.push(Row::new(vec![
                        Cell::from(" WorkingDir").style(key_style),
                        Cell::from(wd.clone()).style(val_style),
                    ]));
                }
            }

            // Hostname
            if let Some(ref hostname) = config.hostname {
                if !hostname.is_empty() {
                    rows.push(Row::new(vec![
                        Cell::from(" Hostname").style(key_style),
                        Cell::from(hostname.clone()).style(val_style),
                    ]));
                }
            }
        }

        // Platform
        if let Some(ref platform) = inspect.platform {
            if !platform.is_empty() {
                rows.push(Row::new(vec![
                    Cell::from(" Platform").style(key_style),
                    Cell::from(platform.clone()).style(val_style),
                ]));
            }
        }

        // Restart policy
        if let Some(ref host_config) = inspect.host_config {
            if let Some(ref restart) = host_config.restart_policy {
                if let Some(ref name) = restart.name {
                    let policy = format!("{:?}", name);
                    rows.push(Row::new(vec![
                        Cell::from(" Restart").style(key_style),
                        Cell::from(policy).style(val_style),
                    ]));
                }
            }
        }
    }

    // Ports (from container summary)
    let ports = container
        .ports
        .as_ref()
        .map(|ports| {
            ports
                .iter()
                .map(|p| {
                    let private = p.private_port;
                    let typ = p.typ.as_ref().map(|t| format!("/{}", t)).unwrap_or_default();
                    if let Some(public) = p.public_port {
                        let ip = p.ip.as_deref().unwrap_or("0.0.0.0");
                        format!("{}:{}->{}{}", ip, public, private, typ)
                    } else {
                        format!("{}{}", private, typ)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "None".to_string());
    rows.push(Row::new(vec![
        Cell::from(" Ports").style(key_style),
        Cell::from(ports).style(val_style),
    ]));

    // Mounts from inspect
    if let Some(ref inspect) = app.container_inspect {
        if let Some(ref mounts) = inspect.mounts {
            if !mounts.is_empty() {
                // Section header
                rows.push(Row::new(vec![Cell::from(""), Cell::from("")]));
                rows.push(Row::new(vec![
                    Cell::from(" Mounts").style(key_style.add_modifier(Modifier::BOLD)),
                    Cell::from("").style(secondary_style),
                ]));
                for mount in mounts {
                    let typ = mount
                        .typ
                        .as_ref()
                        .map(|t| format!("{:?}", t))
                        .unwrap_or_else(|| "?".to_string());
                    let src = mount.source.as_deref().unwrap_or("?");
                    let dst = mount.destination.as_deref().unwrap_or("?");
                    let rw = if mount.rw.unwrap_or(true) { "rw" } else { "ro" };
                    rows.push(Row::new(vec![
                        Cell::from(format!("  {}", typ)).style(secondary_style),
                        Cell::from(format!("{} -> {} ({})", src, dst, rw)).style(val_style),
                    ]));
                }
            }
        }
    }

    // Labels from inspect
    if let Some(ref inspect) = app.container_inspect {
        if let Some(ref config) = inspect.config {
            if let Some(ref labels) = config.labels {
                if !labels.is_empty() {
                    rows.push(Row::new(vec![Cell::from(""), Cell::from("")]));
                    rows.push(Row::new(vec![
                        Cell::from(" Labels").style(key_style.add_modifier(Modifier::BOLD)),
                        Cell::from("").style(secondary_style),
                    ]));
                    let mut sorted_labels: Vec<_> = labels.iter().collect();
                    sorted_labels.sort_by_key(|(k, _)| k.as_str());
                    for (k, v) in sorted_labels {
                        rows.push(Row::new(vec![
                            Cell::from(format!("  {}", k)).style(secondary_style),
                            Cell::from(v.clone()).style(val_style),
                        ]));
                    }
                }
            }
        }
    }

    // Health check section
    if let Some(ref inspect) = app.container_inspect {
        if let Some(ref state) = inspect.state {
            if let Some(ref health) = state.health {
                rows.push(Row::new(vec![Cell::from(""), Cell::from("")]));

                // Health status
                let health_status = health
                    .status
                    .as_ref()
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|| "unknown".to_string());
                let health_color = match health_status.as_str() {
                    "HEALTHY" => Color::Green,
                    "UNHEALTHY" => Color::Rgb(255, 80, 80),
                    "STARTING" => Color::Yellow,
                    _ => Color::White,
                };
                rows.push(Row::new(vec![
                    Cell::from(" Health").style(Style::default().fg(Color::Cyan)),
                    Cell::from(health_status).style(Style::default().fg(health_color).add_modifier(Modifier::BOLD)),
                ]));

                // Recent health check logs
                if let Some(ref log) = health.log {
                    rows.push(Row::new(vec![
                        Cell::from(" Checks").style(Style::default().fg(Color::Cyan)),
                        Cell::from(format!("{} recent", log.len())),
                    ]));
                    for entry in log.iter().rev().take(5) {
                        let exit_code = entry.exit_code.unwrap_or(-1);
                        let icon = if exit_code == 0 { "✓" } else { "✗" };
                        let color = if exit_code == 0 {
                            Color::Green
                        } else {
                            Color::Rgb(255, 80, 80)
                        };
                        let started = entry.start.as_deref().unwrap_or("");
                        // Shorten timestamp
                        let time = if started.len() > 19 { &started[11..19] } else { started };
                        let output = entry.output.as_deref().unwrap_or("").trim();
                        let output_short = if output.len() > 40 {
                            format!("{}…", &output[..39])
                        } else {
                            output.to_string()
                        };
                        rows.push(Row::new(vec![
                            Cell::from(format!("  {} {}", icon, time)).style(Style::default().fg(color)),
                            Cell::from(output_short).style(Style::default().fg(Color::DarkGray)),
                        ]));
                    }
                }
            }
        }
    }

    // Created
    let created = container
        .created
        .map(|ts| {
            chrono::DateTime::from_timestamp(ts, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| ts.to_string())
        })
        .unwrap_or_else(|| "N/A".to_string());
    rows.push(Row::new(vec![Cell::from(""), Cell::from("")]));
    rows.push(Row::new(vec![
        Cell::from(" Created").style(key_style),
        Cell::from(created).style(val_style),
    ]));

    let rows: Vec<Row> = rows.into_iter().map(|r| r.height(1)).collect();

    let table = Table::new(rows, [Constraint::Length(14), Constraint::Min(20)])
        .row_highlight_style(Style::default().bg(Color::Rgb(40, 40, 55)));

    f.render_widget(table, area);
}
