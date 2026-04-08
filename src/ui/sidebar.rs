use crate::app::{App, Focus, SidebarSection};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

fn truncate_name(name: &str, max: usize) -> String {
    if name.chars().count() > max {
        format!("{}…", name.chars().take(max - 1).collect::<String>())
    } else {
        name.to_string()
    }
}

fn shorten_status(status: &str) -> String {
    let (prefix, body) = if let Some(rest) = status.strip_prefix("Up ") {
        ("", rest)
    } else if status.starts_with("Exited (") {
        if let Some(code_end) = status.find(") ") {
            let code = &status[7..code_end + 1]; // "(N)"
            let rest = &status[code_end + 2..];
            (code, rest)
        } else {
            return status.to_string();
        }
    } else {
        return status.to_string();
    };
    let short = body
        .replace("About an hour", "1h")
        .replace("About a minute", "1m")
        .replace(" minutes", "m")
        .replace(" minute", "m")
        .replace(" hours", "h")
        .replace(" hour", "h")
        .replace(" days", "d")
        .replace(" day", "d")
        .replace(" weeks", "w")
        .replace(" week", "w")
        .replace(" seconds", "s")
        .replace(" second", "s")
        .replace(" ago", "");
    if prefix.is_empty() {
        short
    } else {
        format!("{} {}", prefix, short)
    }
}

use super::format_size;

/// Render a scrollable list of lines into an area, with "↑ N more" / "↓ N more" indicators.
fn render_scrollable_lines(f: &mut Frame, area: Rect, lines: Vec<Line<'static>>, selected: Option<usize>) {
    let h = area.height as usize;
    let total = lines.len();

    if total == 0 || h == 0 {
        return;
    }

    // If everything fits, just render
    if total <= h {
        let text: Vec<Line> = lines;
        f.render_widget(Paragraph::new(text), area);
        return;
    }

    // Need scrolling — reserve lines for indicators
    let sel = selected.unwrap_or(0);

    // Calculate visible window
    let avail = h.saturating_sub(1); // at least 1 line for indicator
    let (start, need_top, need_bottom) = if sel < avail {
        // Selected is near top
        (0, false, true)
    } else if sel >= total.saturating_sub(h.saturating_sub(1)) {
        // Selected is near bottom
        let start = total.saturating_sub(h.saturating_sub(1)); // 1 for top indicator
        (start, true, false)
    } else {
        // Middle — need both indicators
        let start = sel.saturating_sub((h.saturating_sub(2)) / 2);
        (start, true, true)
    };

    let mut rendered: Vec<Line> = vec![];

    if need_top {
        rendered.push(Line::from(Span::styled(
            format!("  ↑ {} more", start),
            Style::default().fg(Color::DarkGray),
        )));
    }

    let slots = h - rendered.len() - if need_bottom { 1 } else { 0 };
    let end = (start + slots).min(total);

    for line in lines.into_iter().skip(start).take(end - start) {
        rendered.push(line);
    }

    let actual_end = start + slots;
    if need_bottom && actual_end < total {
        rendered.push(Line::from(Span::styled(
            format!("  ↓ {} more", total - actual_end),
            Style::default().fg(Color::DarkGray),
        )));
    }

    f.render_widget(Paragraph::new(rendered), area);
}

pub fn render_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let border_color = if app.focus == Focus::Sidebar {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let title_text = if let Some(ref host) = app.docker_host {
        format!(
            " rustydocker ({}) ",
            host.trim_start_matches("tcp://").trim_start_matches("http://")
        )
    } else {
        " rustydocker ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(vec![Span::styled(
            title_text,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )]));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split inner area into 4 equal sections
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(inner);

    render_containers_section(f, sections[0], app);
    render_images_section(f, sections[1], app);
    render_volumes_section(f, sections[2], app);
    render_networks_section(f, sections[3], app);
}

fn header_style(active: bool) -> Style {
    if active {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn item_style(is_selected: bool) -> Style {
    if is_selected {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    }
}

fn item_line(spans: Vec<Span<'static>>, is_selected: bool) -> Line<'static> {
    if is_selected {
        Line::from(spans).style(Style::default().bg(Color::Rgb(50, 50, 70)))
    } else {
        Line::from(spans)
    }
}

fn render_containers_section(f: &mut Frame, area: Rect, app: &App) {
    let active = app.sidebar_section == SidebarSection::Services;

    let title = if app.projects.is_empty() {
        "Containers".to_string()
    } else {
        let names: Vec<&str> = app.projects.iter().map(|p| p.name.as_str()).collect();
        names.join(", ")
    };

    let block = Block::default()
        .title(Span::styled(format!(" {} ", title), header_style(active)))
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let content_area = block.inner(area);
    f.render_widget(block, area);

    let filtered = app.filtered_containers();
    let lines: Vec<Line> = filtered
        .iter()
        .map(|(orig_idx, c)| {
            let name = c
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| n.trim_start_matches('/').to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let state = c.state.as_deref().unwrap_or("unknown");
            let (indicator, color) = match state {
                "running" => ("●", Color::Green),
                "exited" => ("●", Color::Rgb(255, 80, 80)),
                "restarting" => ("◉", Color::Yellow),
                "paused" => ("●", Color::Blue),
                _ => ("○", Color::DarkGray),
            };
            let status_raw = c.status.as_deref().unwrap_or(state);
            let status_text = shorten_status(status_raw);
            let display_name = truncate_name(&name, 18);
            let id = c.id.as_deref().unwrap_or("");
            let is_multi_selected = app.selected_containers.contains(id);
            let is_pinned = app.pinned_containers.contains(id);
            let pin_marker = if is_pinned { "★" } else { "" };
            let prefix = if is_multi_selected {
                format!("◆{}{} ", pin_marker, indicator)
            } else if is_pinned {
                format!(" {}{} ", pin_marker, indicator)
            } else {
                format!(" {} ", indicator)
            };
            let is_selected = active && app.selected_index == *orig_idx;
            let has_alert = app.container_has_alert(id);
            let name_style = if has_alert {
                Style::default()
                    .fg(Color::Rgb(255, 80, 80))
                    .add_modifier(Modifier::BOLD)
            } else {
                item_style(is_selected)
            };
            item_line(
                vec![
                    Span::styled(prefix, Style::default().fg(color)),
                    Span::styled(display_name, name_style),
                    Span::styled(format!(" {}", status_text), Style::default().fg(Color::DarkGray)),
                ],
                is_selected,
            )
        })
        .collect();

    let selected = if active {
        filtered
            .iter()
            .position(|(orig_idx, _)| *orig_idx == app.selected_index)
    } else {
        None
    };
    render_scrollable_lines(f, content_area, lines, selected);
}

fn render_images_section(f: &mut Frame, area: Rect, app: &App) {
    let active = app.sidebar_section == SidebarSection::Images;

    let block = Block::default()
        .title(Span::styled(
            format!(" Images ({}) ", app.images.len()),
            header_style(active),
        ))
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let content_area = block.inner(area);
    f.render_widget(block, area);

    let filtered = app.filtered_images();
    let lines: Vec<Line> = filtered
        .iter()
        .map(|(orig_idx, image)| {
            let tag = image.repo_tags.first().map(|t| t.as_str()).unwrap_or("<none>");
            let size = format_size(image.size);
            let is_selected = active && app.selected_index == *orig_idx;
            item_line(
                vec![
                    Span::styled(" ", Style::default()),
                    Span::styled(tag.to_string(), item_style(is_selected)),
                    Span::styled(format!(" {}", size), Style::default().fg(Color::DarkGray)),
                ],
                is_selected,
            )
        })
        .collect();

    let selected = if active {
        filtered
            .iter()
            .position(|(orig_idx, _)| *orig_idx == app.selected_index)
    } else {
        None
    };
    render_scrollable_lines(f, content_area, lines, selected);
}

fn render_volumes_section(f: &mut Frame, area: Rect, app: &App) {
    let active = app.sidebar_section == SidebarSection::Volumes;

    let block = Block::default()
        .title(Span::styled(
            format!(" Volumes ({}) ", app.volumes.len()),
            header_style(active),
        ))
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let content_area = block.inner(area);
    f.render_widget(block, area);

    let max_name = content_area.width.saturating_sub(2) as usize;
    let filtered = app.filtered_volumes();
    let lines: Vec<Line> = filtered
        .iter()
        .map(|(orig_idx, volume)| {
            let display_name = truncate_name(&volume.name, max_name);
            let is_selected = active && app.selected_index == *orig_idx;
            item_line(
                vec![
                    Span::styled(" ", Style::default()),
                    Span::styled(display_name, item_style(is_selected)),
                ],
                is_selected,
            )
        })
        .collect();

    let selected = if active {
        filtered
            .iter()
            .position(|(orig_idx, _)| *orig_idx == app.selected_index)
    } else {
        None
    };
    render_scrollable_lines(f, content_area, lines, selected);
}

fn render_networks_section(f: &mut Frame, area: Rect, app: &App) {
    let active = app.sidebar_section == SidebarSection::Networks;

    let block = Block::default().title(Span::styled(
        format!(" Networks ({}) ", app.networks.len()),
        header_style(active),
    ));

    let content_area = block.inner(area);
    f.render_widget(block, area);

    let filtered = app.filtered_networks();
    let lines: Vec<Line> = filtered
        .iter()
        .map(|(orig_idx, network)| {
            let name = network.name.as_deref().unwrap_or("unknown");
            let driver = network.driver.as_deref().unwrap_or("");
            let display_name = truncate_name(name, 18);
            let is_selected = active && app.selected_index == *orig_idx;
            item_line(
                vec![
                    Span::styled(" ", Style::default()),
                    Span::styled(display_name, item_style(is_selected)),
                    Span::styled(format!(" {}", driver), Style::default().fg(Color::DarkGray)),
                ],
                is_selected,
            )
        })
        .collect();

    let selected = if active {
        filtered
            .iter()
            .position(|(orig_idx, _)| *orig_idx == app.selected_index)
    } else {
        None
    };
    render_scrollable_lines(f, content_area, lines, selected);
}
