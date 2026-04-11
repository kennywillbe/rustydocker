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
pub mod theme;
pub mod top;
pub mod update_modal;

use crate::app::{self, App, Focus, InputMode, SidebarSection};
use crate::ui::theme::{self as thm, icons};
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::prelude::*;
use ratatui::widgets::{Cell, Paragraph, Row, Table};

pub fn draw(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let app_layout = layout::build_layout(f.area(), app.sidebar_width, app.screen_mode);

    if app_layout.sidebar.width > 0 {
        sidebar::render_sidebar(f, app_layout.sidebar, app);
    }

    if app_layout.divider.width > 0 && app_layout.divider.height > 0 {
        let rule_style = Style::default().fg(t.rule);
        let lines: Vec<Line> = (0..app_layout.divider.height)
            .map(|_| Line::from(Span::styled(icons::VRULE, rule_style)))
            .collect();
        f.render_widget(Paragraph::new(lines), app_layout.divider);
    }

    let main_area = app_layout.main_panel;
    let header_area = Rect::new(main_area.x, main_area.y, main_area.width, 1);
    let content_inner = Rect::new(
        main_area.x,
        main_area.y + 1,
        main_area.width,
        main_area.height.saturating_sub(1),
    );

    let header_line: Line = match app.sidebar_section {
        SidebarSection::Services => build_tab_line(app),
        SidebarSection::Images => detail_header("IMAGE DETAIL", app),
        SidebarSection::Volumes => detail_header("VOLUME DETAIL", app),
        SidebarSection::Networks => detail_header("NETWORK DETAIL", app),
    };
    f.render_widget(Paragraph::new(header_line), header_area);

    // Render content beneath the header.
    match app.sidebar_section {
        SidebarSection::Services => match app.active_tab {
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
        },
        SidebarSection::Images => render_image_detail(f, content_inner, app),
        SidebarSection::Volumes => render_volume_detail(f, content_inner, app),
        SidebarSection::Networks => render_network_detail(f, content_inner, app),
    }

    // Status bar split: left = normal hints, right = update indicator.
    let right_width = update_indicator_width(app);
    let status_areas =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(right_width)]).split(app_layout.status_bar);
    let left_area = status_areas[0];
    let right_area = status_areas[1];

    f.render_widget(Paragraph::new(build_status_line(app)), left_area);
    if right_width > 0 {
        f.render_widget(
            Paragraph::new(build_update_indicator(app)).alignment(Alignment::Right),
            right_area,
        );
    }

    // Popups render on top of everything else.
    if app.show_help {
        help::render_help(f, f.area(), app);
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
    update_modal::render(f, f.area(), app);
}

fn update_indicator_width(app: &App) -> u16 {
    use crate::app::UpdateFlow;
    match (&app.update_flow, &app.update_available) {
        (UpdateFlow::Idle, None) => 0,
        (UpdateFlow::Idle, Some(info)) if info.self_updatable => {
            // e.g. "↑ 0.3.1 available · Ctrl+U " — keep a couple of spare cells.
            (info.version.chars().count() as u16) + 24
        }
        (UpdateFlow::Idle, Some(info)) => (info.version.chars().count() as u16) + 22,
        (UpdateFlow::Downloading(_), _) => {
            let version_len = app
                .update_available
                .as_ref()
                .map(|i| i.version.chars().count())
                .unwrap_or(6);
            (version_len as u16) + 22
        }
        (UpdateFlow::Installing, _) => 18,
        (UpdateFlow::InstalledPendingRestart, _) => {
            let version_len = app
                .update_available
                .as_ref()
                .map(|i| i.version.chars().count())
                .unwrap_or(6);
            (version_len as u16) + 24
        }
        // Modal owns the screen in these states — hide the right segment.
        (UpdateFlow::Confirming, _) | (UpdateFlow::Complete, _) | (UpdateFlow::Failed(_), _) => 0,
    }
}

fn build_update_indicator(app: &App) -> Line<'static> {
    use crate::app::UpdateFlow;
    let t = &app.theme;
    let accent = Style::default().fg(t.accent_primary).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(t.fg_muted);

    match (&app.update_flow, &app.update_available) {
        (UpdateFlow::Idle, Some(info)) if info.self_updatable => {
            Line::from(Span::styled(format!("↑ {} available · Ctrl+U ", info.version), accent))
        }
        (UpdateFlow::Idle, Some(info)) => {
            Line::from(Span::styled(format!("↑ {} · package manager ", info.version), dim))
        }
        (UpdateFlow::Downloading(p), _) => {
            let version = app
                .update_available
                .as_ref()
                .map(|i| i.version.clone())
                .unwrap_or_else(|| "update".to_string());
            Line::from(Span::styled(format!("↓ downloading {} {}% ", version, p), accent))
        }
        (UpdateFlow::Installing, _) => Line::from(Span::styled("↓ installing… ".to_string(), accent)),
        (UpdateFlow::InstalledPendingRestart, _) => {
            let version = app
                .update_available
                .as_ref()
                .map(|i| i.version.clone())
                .unwrap_or_default();
            Line::from(Span::styled(format!("✓ {} installed · restart ", version), accent))
        }
        _ => Line::from(""),
    }
}

/// Tab bar: `Logs · Stats · Info · Env · Top · Graph`. Active tab underlined.
/// `Logs` is replaced by `Diff` / `All Logs` when those modes are on.
fn build_tab_line(app: &App) -> Line<'static> {
    let t = &app.theme;
    let separator = thm::tab_separator(t);
    let active = thm::tab_active(t);
    let inactive = thm::tab_inactive(t);

    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];
    for (i, tab) in app::Tab::all().iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" · ", separator));
        }
        let label: &'static str = if *tab == app::Tab::Logs && app.show_log_diff {
            "Diff"
        } else if *tab == app::Tab::Logs && app.show_all_logs {
            "All Logs"
        } else {
            tab.label()
        };
        let style = if *tab == app.active_tab { active } else { inactive };
        spans.push(Span::styled(label, style));
    }
    Line::from(spans)
}

fn detail_header(label: &'static str, app: &App) -> Line<'static> {
    Line::from(vec![
        Span::raw(" "),
        Span::styled(label, thm::section_header(&app.theme, app.focus == Focus::MainPanel)),
    ])
}

fn build_status_line(app: &App) -> Line<'static> {
    let t = &app.theme;
    let dim = thm::dim_label(t);
    let prompt_style = Style::default().fg(t.accent_primary).add_modifier(Modifier::BOLD);
    let input_style = Style::default().fg(t.fg_bright);

    if app.input_mode == InputMode::Filter {
        let filter_text = app.sidebar_filter.as_deref().unwrap_or("").to_string();
        return Line::from(vec![
            Span::styled(" filter: ", prompt_style),
            Span::styled(filter_text, input_style),
            Span::styled("_", Style::default().fg(t.accent_primary)),
        ]);
    }

    if app.input_mode == InputMode::Search {
        let raw = app.log_search.as_deref().unwrap_or("");
        let cursor_style = if raw.is_empty() || regex::Regex::new(raw).is_ok() {
            Style::default().fg(t.accent_primary)
        } else {
            Style::default().fg(t.err)
        };
        return Line::from(vec![
            Span::styled(" /", prompt_style),
            Span::styled(raw.to_string(), input_style),
            Span::styled("_", cursor_style),
        ]);
    }

    if let Some(ref msg) = app.status_message {
        let color = if msg.contains("pruned") || msg.contains("Started") {
            t.ok
        } else if msg.contains("Error") || msg.contains("Failed") {
            t.err
        } else {
            t.accent_header
        };
        return Line::from(vec![Span::styled(format!(" {}", msg), Style::default().fg(color))]);
    }

    let running = app
        .containers
        .iter()
        .filter(|c| c.state.as_deref() == Some("running"))
        .count();
    let stopped = app.containers.len() - running;

    Line::from(vec![
        Span::raw(" "),
        Span::styled(icons::RUN, Style::default().fg(t.ok)),
        Span::styled(format!(" {} running  ", running), dim),
        Span::styled(icons::EXITED, Style::default().fg(t.fg_muted)),
        Span::styled(format!(" {} stopped", stopped), dim),
        Span::styled(
            "     j/k nav    tab switch    / search    r restart    s stop    ? help    q quit",
            dim,
        ),
    ])
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
    let t = &app.theme;
    let image = match app.images.get(app.selected_index) {
        Some(img) => img,
        None => {
            f.render_widget(
                Paragraph::new("No images")
                    .style(Style::default().fg(t.fg_dim))
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
            Cell::from(" Tag").style(thm::label_cell(t)),
            Cell::from(tag.to_string()).style(thm::value_cell(t)),
        ]),
        Row::new(vec![
            Cell::from(" ID").style(thm::label_cell(t)),
            Cell::from(id.to_string()).style(Style::default().fg(t.fg)),
        ]),
        Row::new(vec![
            Cell::from(" Size").style(thm::label_cell(t)),
            Cell::from(size).style(Style::default().fg(t.fg)),
        ]),
        Row::new(vec![
            Cell::from(" Created").style(thm::label_cell(t)),
            Cell::from(created).style(Style::default().fg(t.fg)),
        ]),
    ];

    let table = Table::new(rows, [Constraint::Length(12), Constraint::Min(20)]);
    f.render_widget(table, area);
}

fn render_network_detail(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let network = match app.networks.get(app.selected_index) {
        Some(n) => n,
        None => {
            f.render_widget(
                Paragraph::new("No networks")
                    .style(Style::default().fg(t.fg_dim))
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
            Cell::from(" Name").style(thm::label_cell(t)),
            Cell::from(name.to_string()).style(thm::value_cell(t)),
        ]),
        Row::new(vec![
            Cell::from(" ID").style(thm::label_cell(t)),
            Cell::from(id.to_string()).style(Style::default().fg(t.fg)),
        ]),
        Row::new(vec![
            Cell::from(" Driver").style(thm::label_cell(t)),
            Cell::from(driver.to_string()).style(Style::default().fg(t.fg)),
        ]),
        Row::new(vec![
            Cell::from(" Scope").style(thm::label_cell(t)),
            Cell::from(scope.to_string()).style(Style::default().fg(t.fg)),
        ]),
    ];

    let table = Table::new(rows, [Constraint::Length(12), Constraint::Min(20)]);
    f.render_widget(table, area);
}

fn render_volume_detail(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let volume = match app.volumes.get(app.selected_index) {
        Some(v) => v,
        None => {
            f.render_widget(
                Paragraph::new("No volumes")
                    .style(Style::default().fg(t.fg_dim))
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
            Cell::from(" Name").style(thm::label_cell(t)),
            Cell::from(volume.name.clone()).style(thm::value_cell(t)),
        ]),
        Row::new(vec![
            Cell::from(" Driver").style(thm::label_cell(t)),
            Cell::from(driver.to_string()).style(Style::default().fg(t.fg)),
        ]),
        Row::new(vec![
            Cell::from(" Mountpoint").style(thm::label_cell(t)),
            Cell::from(mountpoint.to_string()).style(Style::default().fg(t.fg)),
        ]),
        Row::new(vec![
            Cell::from(" Scope").style(thm::label_cell(t)),
            Cell::from(scope).style(Style::default().fg(t.fg)),
        ]),
    ];

    let table = Table::new(rows, [Constraint::Length(12), Constraint::Min(20)]);
    f.render_widget(table, area);
}
