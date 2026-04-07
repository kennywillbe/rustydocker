pub mod cleanup;
pub mod graph;
pub mod help;
pub mod info;
pub mod layout;
pub mod logs;
pub mod sidebar;
pub mod stats_panel;

use crate::app::{self, App, Focus, SidebarSection};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::layout::Constraint;

pub fn draw(f: &mut Frame, app: &App) {
    let app_layout = layout::build_layout(f.area());

    // Sidebar
    sidebar::render_sidebar(f, app_layout.sidebar, app);

    // Main panel title depends on sidebar section
    let title_line: Vec<Span> = match app.sidebar_section {
        SidebarSection::Services => {
            let tab_titles: Vec<Span> = app::Tab::all()
                .iter()
                .map(|t| {
                    if *t == app.active_tab {
                        Span::styled(
                            format!(" {} ", t.label()),
                            Style::default()
                                .fg(Color::Yellow)
                                .bg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::styled(
                            format!(" {} ", t.label()),
                            Style::default().fg(Color::DarkGray),
                        )
                    }
                })
                .collect();

            let mut line = vec![Span::raw(" ")];
            for (i, span) in tab_titles.into_iter().enumerate() {
                line.push(span);
                if i < app::Tab::all().len() - 1 {
                    line.push(Span::styled(" \u{2502} ", Style::default().fg(Color::DarkGray)));
                }
            }
            line.push(Span::raw(" "));
            line
        }
        SidebarSection::Images => {
            vec![Span::styled(" Image Detail ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]
        }
        SidebarSection::Volumes => {
            vec![Span::styled(" Volume Detail ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]
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
                app::Tab::Logs => logs::render_logs(f, content_inner, app),
                app::Tab::Stats => stats_panel::render_stats(f, content_inner, app),
                app::Tab::Info => info::render_info(f, content_inner, app),
                app::Tab::Graph => graph::render_graph(f, content_inner, app),
            };
        }
        SidebarSection::Images => {
            render_image_detail(f, content_inner, app);
        }
        SidebarSection::Volumes => {
            render_volume_detail(f, content_inner, app);
        }
    }

    // Status bar with more info
    let running = app
        .containers
        .iter()
        .filter(|c| c.state.as_deref() == Some("running"))
        .count();
    let stopped = app.containers.len() - running;
    let status_left = format!(" \u{25cf} {} running  \u{25cb} {} stopped", running, stopped);
    let status_right = " ?:help  x:cleanup  q:quit ";

    let status_line = Line::from(vec![
        Span::styled(status_left, Style::default().fg(Color::Green)),
        Span::styled(
            " \u{2502} ",
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(status_right, Style::default().fg(Color::DarkGray)),
    ]);
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
}

fn format_size(bytes: i64) -> String {
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
                Paragraph::new("No images").style(Style::default().fg(Color::DarkGray)).alignment(Alignment::Center),
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

fn render_volume_detail(f: &mut Frame, area: Rect, app: &App) {
    let volume = match app.volumes.get(app.selected_index) {
        Some(v) => v,
        None => {
            f.render_widget(
                Paragraph::new("No volumes").style(Style::default().fg(Color::DarkGray)).alignment(Alignment::Center),
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
