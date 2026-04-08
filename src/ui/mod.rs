pub mod bulk;
pub mod cleanup;
pub mod confirm;
pub mod custom_commands;
pub mod env;
pub mod graph;
pub mod help;
pub mod info;
pub mod layout;
pub mod log_diff;
pub mod logs;
pub mod sidebar;
pub mod stats_compare;
pub mod stats_panel;
pub mod top;

use crate::app::{self, App, Focus, InputMode, SidebarSection};
use ratatui::layout::Constraint;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

pub fn draw(f: &mut Frame, app: &App) {
    let app_layout = layout::build_layout(f.area(), app.sidebar_width, app.screen_mode);

    // Sidebar
    if app_layout.sidebar.width > 0 {
        sidebar::render_sidebar(f, app_layout.sidebar, app);
    }

    // Main panel title depends on sidebar section
    let title_line: Vec<Span> = match app.sidebar_section {
        SidebarSection::Services => {
            let mut line: Vec<Span> = vec![Span::raw(" ")];
            for (i, t) in app::Tab::all().iter().enumerate() {
                let label = if *t == app::Tab::Logs && app.show_log_diff {
                    " Diff ".to_string()
                } else if *t == app::Tab::Logs && app.show_all_logs {
                    " All Logs ".to_string()
                } else {
                    format!(" {} ", t.label())
                };
                if *t == app.active_tab {
                    line.push(Span::styled(
                        label,
                        Style::default()
                            .fg(Color::Rgb(30, 30, 40))
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    line.push(Span::styled(label, Style::default().fg(Color::Gray)));
                }
                if i < app::Tab::all().len() - 1 {
                    line.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                }
            }
            line.push(Span::raw(" "));
            line
        }
        SidebarSection::Images => {
            vec![Span::styled(
                " Image Detail ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )]
        }
        SidebarSection::Volumes => {
            vec![Span::styled(
                " Volume Detail ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )]
        }
        SidebarSection::Networks => {
            vec![Span::styled(
                " Network Detail ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )]
        }
    };

    let main_border_color = if app.focus == Focus::MainPanel {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(main_border_color))
        .title(Line::from(title_line));

    let content_inner = main_block.inner(app_layout.main_panel);
    f.render_widget(main_block, app_layout.main_panel);

    // Render content based on sidebar section
    match app.sidebar_section {
        SidebarSection::Services => {
            match app.active_tab {
                app::Tab::Logs => {
                    if app.show_log_diff {
                        log_diff::render_log_diff(f, content_inner, app)
                    } else if app.show_all_logs {
                        logs::render_all_logs(f, content_inner, app)
                    } else {
                        logs::render_logs(f, content_inner, app)
                    }
                }
                app::Tab::Stats => {
                    if app.compare_container_id.is_some() {
                        stats_compare::render_stats_compare(f, content_inner, app)
                    } else {
                        stats_panel::render_stats(f, content_inner, app)
                    }
                }
                app::Tab::Info => info::render_info(f, content_inner, app),
                app::Tab::Env => env::render_env(f, content_inner, app),
                app::Tab::Top => top::render_top(f, content_inner, app),
                app::Tab::Graph => graph::render_graph(f, content_inner, app),
            };
        }
        SidebarSection::Images => {
            render_image_detail(f, content_inner, app);
        }
        SidebarSection::Volumes => {
            render_volume_detail(f, content_inner, app);
        }
        SidebarSection::Networks => {
            render_network_detail(f, content_inner, app);
        }
    }

    // Status bar — search input replaces it when active
    let status_line = if app.input_mode == InputMode::Filter {
        let filter_text = app.sidebar_filter.as_deref().unwrap_or("");
        Line::from(vec![
            Span::styled(
                " filter: ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(filter_text),
            Span::styled("_", Style::default().fg(Color::Cyan)),
        ])
    } else if app.input_mode == InputMode::Search {
        let search_text = app.log_search.as_deref().unwrap_or("");
        let cursor_style = if search_text.is_empty() || regex::Regex::new(search_text).is_ok() {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Red)
        };
        Line::from(vec![
            Span::styled(" /", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(search_text),
            Span::styled("_", cursor_style),
        ])
    } else if let Some(ref msg) = app.status_message {
        let color = if msg.contains("pruned") || msg.contains("Started") {
            Color::Green
        } else if msg.contains("Error") || msg.contains("Failed") {
            Color::Rgb(255, 80, 80)
        } else {
            Color::Yellow
        };
        Line::from(vec![Span::styled(format!(" {}", msg), Style::default().fg(color))])
    } else {
        let running = app
            .containers
            .iter()
            .filter(|c| c.state.as_deref() == Some("running"))
            .count();
        let stopped = app.containers.len() - running;
        let status_left = format!(" \u{25cf} {} running  \u{25cb} {} stopped", running, stopped);
        let status_right = " ?:help  x:cleanup  b:bulk  q:quit ";
        Line::from(vec![
            Span::styled(status_left, Style::default().fg(Color::Green)),
            Span::styled(" \u{2502} ", Style::default().fg(Color::DarkGray)),
            Span::styled(status_right, Style::default().fg(Color::DarkGray)),
        ])
    };
    f.render_widget(
        Paragraph::new(status_line).style(Style::default().bg(Color::Rgb(30, 30, 40))),
        app_layout.status_bar,
    );

    // Popups (must be last - rendered on top)
    if app.show_help {
        help::render_help(f, f.area());
    }
    if app.show_cleanup {
        cleanup::render_cleanup(f, f.area(), app);
    }
    if app.show_bulk {
        bulk::render_bulk(f, f.area(), app);
    }
    if app.show_custom_commands {
        custom_commands::render_custom_commands(f, f.area(), app);
    }
    if app.pending_confirm.is_some() {
        confirm::render_confirm(f, f.area(), app);
    }
}

pub fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

pub fn format_size(bytes: i64) -> String {
    let mb = bytes as f64 / 1_048_576.0;
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{:.0} MB", mb)
    }
}

fn render_image_detail(f: &mut Frame, area: Rect, app: &App) {
    let image = match app.images.get(app.selected_index) {
        Some(img) => img,
        None => {
            f.render_widget(
                Paragraph::new("No images")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let tag = image.repo_tags.first().map(|t| t.as_str()).unwrap_or("<none>");
    let id = &image.id[..std::cmp::min(19, image.id.len())];
    let size = format_size(image.size);
    let created = chrono::DateTime::from_timestamp(image.created, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| image.created.to_string());

    let rows = vec![
        Row::new(vec![
            Cell::from(" Tag").style(Style::default().fg(Color::Cyan)),
            Cell::from(tag.to_string()).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Row::new(vec![
            Cell::from(" ID").style(Style::default().fg(Color::Cyan)),
            Cell::from(id.to_string()),
        ]),
        Row::new(vec![
            Cell::from(" Size").style(Style::default().fg(Color::Cyan)),
            Cell::from(size),
        ]),
        Row::new(vec![
            Cell::from(" Created").style(Style::default().fg(Color::Cyan)),
            Cell::from(created),
        ]),
    ];

    let table = Table::new(rows, [Constraint::Length(12), Constraint::Min(20)]);
    f.render_widget(table, area);
}

fn render_network_detail(f: &mut Frame, area: Rect, app: &App) {
    let network = match app.networks.get(app.selected_index) {
        Some(n) => n,
        None => {
            f.render_widget(
                Paragraph::new("No networks")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let name = network.name.as_deref().unwrap_or("unknown");
    let id = network
        .id
        .as_deref()
        .map(|id| &id[..std::cmp::min(12, id.len())])
        .unwrap_or("");
    let driver = network.driver.as_deref().unwrap_or("");
    let scope = network.scope.as_deref().unwrap_or("");

    let rows = vec![
        Row::new(vec![
            Cell::from(" Name").style(Style::default().fg(Color::Cyan)),
            Cell::from(name.to_string()).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Row::new(vec![
            Cell::from(" ID").style(Style::default().fg(Color::Cyan)),
            Cell::from(id.to_string()),
        ]),
        Row::new(vec![
            Cell::from(" Driver").style(Style::default().fg(Color::Cyan)),
            Cell::from(driver.to_string()),
        ]),
        Row::new(vec![
            Cell::from(" Scope").style(Style::default().fg(Color::Cyan)),
            Cell::from(scope.to_string()),
        ]),
    ];

    let table = Table::new(rows, [Constraint::Length(12), Constraint::Min(20)]);
    f.render_widget(table, area);
}

fn render_volume_detail(f: &mut Frame, area: Rect, app: &App) {
    let volume = match app.volumes.get(app.selected_index) {
        Some(v) => v,
        None => {
            f.render_widget(
                Paragraph::new("No volumes")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let driver = volume.driver.as_str();
    let mountpoint = volume.mountpoint.as_str();
    let scope = volume.scope.as_ref().map(|s| format!("{:?}", s)).unwrap_or_default();

    let rows = vec![
        Row::new(vec![
            Cell::from(" Name").style(Style::default().fg(Color::Cyan)),
            Cell::from(volume.name.clone()).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Row::new(vec![
            Cell::from(" Driver").style(Style::default().fg(Color::Cyan)),
            Cell::from(driver.to_string()),
        ]),
        Row::new(vec![
            Cell::from(" Mountpoint").style(Style::default().fg(Color::Cyan)),
            Cell::from(mountpoint.to_string()),
        ]),
        Row::new(vec![
            Cell::from(" Scope").style(Style::default().fg(Color::Cyan)),
            Cell::from(scope),
        ]),
    ];

    let table = Table::new(rows, [Constraint::Length(12), Constraint::Min(20)]);
    f.render_widget(table, area);
}
