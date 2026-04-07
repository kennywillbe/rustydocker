use crate::app::{App, Focus, SidebarSection};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

fn format_size(bytes: i64) -> String {
    let mb = bytes as f64 / 1_048_576.0;
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{:.0} MB", mb)
    }
}

struct SidebarItem {
    line: Line<'static>,
    highlight: bool,
}

pub fn render_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let mut sidebar_items: Vec<SidebarItem> = vec![];
    let mut selected_visual_index: Option<usize> = None;

    // === Containers Header ===
    let svc_active = app.sidebar_section == SidebarSection::Services;
    let svc_header_style = if svc_active {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let section_title = if app.projects.is_empty() {
        " Containers ".to_string()
    } else {
        let names: Vec<&str> = app.projects.iter().map(|p| p.name.as_str()).collect();
        format!(" {} ", names.join(", "))
    };
    sidebar_items.push(SidebarItem {
        line: Line::from(Span::styled(section_title, svc_header_style)),
        highlight: false,
    });

    for (i, container) in app.containers.iter().enumerate() {
        let name = container.names.as_ref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let state = container.state.as_deref().unwrap_or("unknown");
        let (indicator, color) = match state {
            "running" => ("●", Color::Green),
            "exited" => ("●", Color::Rgb(255, 80, 80)),
            "restarting" => ("◉", Color::Yellow),
            "paused" => ("●", Color::Blue),
            _ => ("○", Color::DarkGray),
        };

        let is_selected = svc_active && app.selected_index == i;
        if is_selected { selected_visual_index = Some(sidebar_items.len()); }

        sidebar_items.push(SidebarItem {
            line: Line::from(vec![
                Span::styled(format!("  {} ", indicator), Style::default().fg(color)),
                Span::styled(name, if is_selected {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                }),
            ]),
            highlight: is_selected,
        });
    }

    // === Separator ===
    sidebar_items.push(SidebarItem {
        line: Line::from(Span::styled("─".repeat(area.width.saturating_sub(2) as usize), Style::default().fg(Color::DarkGray))),
        highlight: false,
    });

    // === Images Header ===
    let img_active = app.sidebar_section == SidebarSection::Images;
    let img_header_style = if img_active {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    sidebar_items.push(SidebarItem {
        line: Line::from(Span::styled(format!(" Images ({})", app.images.len()), img_header_style)),
        highlight: false,
    });

    for (i, image) in app.images.iter().enumerate() {
        let tag = image.repo_tags.first().map(|t| t.as_str()).unwrap_or("<none>");
        let size = format_size(image.size);
        let is_selected = img_active && app.selected_index == i;
        if is_selected { selected_visual_index = Some(sidebar_items.len()); }

        sidebar_items.push(SidebarItem {
            line: Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(tag.to_string(), if is_selected {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                }),
                Span::styled(format!(" {}", size), Style::default().fg(Color::DarkGray)),
            ]),
            highlight: is_selected,
        });
    }

    // === Separator ===
    sidebar_items.push(SidebarItem {
        line: Line::from(Span::styled("─".repeat(area.width.saturating_sub(2) as usize), Style::default().fg(Color::DarkGray))),
        highlight: false,
    });

    // === Volumes Header ===
    let vol_active = app.sidebar_section == SidebarSection::Volumes;
    let vol_header_style = if vol_active {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    sidebar_items.push(SidebarItem {
        line: Line::from(Span::styled(format!(" Volumes ({})", app.volumes.len()), vol_header_style)),
        highlight: false,
    });

    for (i, volume) in app.volumes.iter().enumerate() {
        let display_name = if volume.name.len() > 20 {
            format!("{}…", &volume.name[..19])
        } else {
            volume.name.clone()
        };
        let is_selected = vol_active && app.selected_index == i;
        if is_selected { selected_visual_index = Some(sidebar_items.len()); }

        sidebar_items.push(SidebarItem {
            line: Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(display_name, if is_selected {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                }),
            ]),
            highlight: is_selected,
        });
    }

    // Build ListItems with highlight
    let list_items: Vec<ListItem> = sidebar_items.iter().map(|item| {
        let li = ListItem::new(item.line.clone());
        if item.highlight {
            li.style(Style::default().bg(Color::Rgb(50, 50, 70)))
        } else {
            li
        }
    }).collect();

    let border_color = if app.focus == Focus::Sidebar { Color::Cyan } else { Color::DarkGray };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(vec![Span::styled(
            " rustydocker ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )]));

    // Use ListState to auto-scroll to selected item
    let mut state = ListState::default();
    state.select(selected_visual_index);

    let list = List::new(list_items)
        .block(block)
        .highlight_symbol("");

    f.render_stateful_widget(list, area, &mut state);
}
