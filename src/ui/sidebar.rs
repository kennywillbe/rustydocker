use crate::app::{App, Focus, SidebarSection};
use crate::ui::theme::{self, icons, Theme};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use std::borrow::Cow;

use super::format_size;

fn truncate_name(name: &str, max: usize) -> String {
    if name.chars().count() > max {
        format!("{}…", name.chars().take(max.saturating_sub(1)).collect::<String>())
    } else {
        name.to_string()
    }
}

fn shorten_status(status: &str) -> String {
    let (prefix, body) = if let Some(rest) = status.strip_prefix("Up ") {
        ("up", rest)
    } else if status.starts_with("Exited (") {
        if let Some(code_end) = status.find(") ") {
            let code = &status[7..code_end + 1];
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

/// Build a header line like "CONTAINERS ────────────" that spans `width`.
/// The rule string is served from a static pool, so only a dynamic label
/// (project names) allocates.
fn header_line(label: Cow<'static, str>, width: usize, focused: bool, t: &Theme) -> Line<'static> {
    let label_len = label.chars().count();
    let rule_len = width.saturating_sub(label_len + 3); // 1 lead + 1 gap + 1 trail
    Line::from(vec![
        Span::raw(" "),
        Span::styled(label, theme::section_header(t, focused)),
        Span::raw(" "),
        Span::styled(theme::rule(rule_len), theme::section_rule(t)),
        Span::raw(" "),
    ])
}

/// Render a scrollable list of lines into an area, with "↑ N more" / "↓ N more" indicators.
fn render_scrollable_lines(f: &mut Frame, area: Rect, lines: Vec<Line<'static>>, selected: Option<usize>, t: &Theme) {
    let h = area.height as usize;
    let total = lines.len();

    if total == 0 || h == 0 {
        return;
    }

    if total <= h {
        f.render_widget(Paragraph::new(lines), area);
        return;
    }

    let sel = selected.unwrap_or(0);
    let avail = h.saturating_sub(1);
    let (start, need_top, need_bottom) = if sel < avail {
        (0, false, true)
    } else if sel >= total.saturating_sub(h.saturating_sub(1)) {
        let start = total.saturating_sub(h.saturating_sub(1));
        (start, true, false)
    } else {
        let start = sel.saturating_sub((h.saturating_sub(2)) / 2);
        (start, true, true)
    };

    let mut rendered: Vec<Line> = vec![];
    let dim = theme::dim_label(t);

    if need_top {
        rendered.push(Line::from(Span::styled(
            format!("  {} {} more", icons::ARROW_UP, start),
            dim,
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
            format!("  {} {} more", icons::ARROW_DOWN, total - actual_end),
            dim,
        )));
    }

    f.render_widget(Paragraph::new(rendered), area);
}

pub fn render_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(area);

    render_containers_section(f, sections[0], app);
    render_images_section(f, sections[1], app);
    render_volumes_section(f, sections[2], app);
    render_networks_section(f, sections[3], app);
}

fn item_line(spans: Vec<Span<'static>>, is_selected: bool, t: &Theme) -> Line<'static> {
    if is_selected {
        Line::from(spans).style(theme::selected_row(t))
    } else {
        Line::from(spans)
    }
}

/// Render a section header + body into an area. Returns the area beneath
/// the header line where list items should be drawn.
fn render_section_header(f: &mut Frame, area: Rect, label: Cow<'static, str>, focused: bool, app: &App) -> Rect {
    let header = header_line(label, area.width as usize, focused, &app.theme);
    let header_area = Rect::new(area.x, area.y, area.width, 1);
    f.render_widget(Paragraph::new(header), header_area);
    Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1))
}

fn render_containers_section(f: &mut Frame, area: Rect, app: &App) {
    let active = app.sidebar_section == SidebarSection::Services;
    let focused = active && app.focus == Focus::Sidebar;

    let label: Cow<'static, str> = if app.projects.is_empty() {
        Cow::Borrowed("CONTAINERS")
    } else {
        Cow::Owned(
            app.projects
                .iter()
                .map(|p| p.name.to_ascii_uppercase())
                .collect::<Vec<_>>()
                .join(", "),
        )
    };

    let content_area = render_section_header(f, area, label, focused, app);

    let t = &app.theme;
    let dim = theme::dim_label(t);
    let filtered = app.filtered_containers();
    let max_name = (content_area.width as usize).saturating_sub(16).max(8);
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
            let (glyph, glyph_style) = theme::state_style(t, state);

            let status_raw = c.status.as_deref().unwrap_or(state);
            let status_text = shorten_status(status_raw);

            let id = c.id.as_deref().unwrap_or("");
            let is_multi_selected = app.selected_containers.contains(id);
            let is_pinned = app.pinned_containers.contains(id);
            let is_selected = active && app.selected_index == *orig_idx;
            let has_alert = app.container_has_alert(id);

            let display_name = truncate_name(&name, max_name);

            let select_prefix = if is_multi_selected {
                Span::styled(icons::SELECTED, Style::default().fg(t.accent_primary))
            } else {
                Span::raw(" ")
            };

            let mut spans = vec![
                Span::raw(" "),
                select_prefix,
                Span::raw(" "),
                Span::styled(glyph, glyph_style),
                Span::raw(" "),
                Span::styled(display_name, theme::name_style(t, is_selected, has_alert)),
                Span::raw(" "),
                Span::styled(status_text, dim),
            ];

            if is_pinned {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(icons::PIN, Style::default().fg(t.accent_primary)));
            }

            item_line(spans, is_selected, t)
        })
        .collect();

    let selected = if active {
        filtered
            .iter()
            .position(|(orig_idx, _)| *orig_idx == app.selected_index)
    } else {
        None
    };
    render_scrollable_lines(f, content_area, lines, selected, t);
}

fn render_images_section(f: &mut Frame, area: Rect, app: &App) {
    let active = app.sidebar_section == SidebarSection::Images;
    let focused = active && app.focus == Focus::Sidebar;
    let content_area = render_section_header(f, area, Cow::Borrowed("IMAGES"), focused, app);

    let t = &app.theme;
    let dim = theme::dim_label(t);
    let filtered = app.filtered_images();
    let lines: Vec<Line> = filtered
        .iter()
        .map(|(orig_idx, image)| {
            let tag = image.repo_tags.first().map(|t| t.as_str()).unwrap_or("<none>");
            let size = format_size(image.size);
            let is_selected = active && app.selected_index == *orig_idx;
            item_line(
                vec![
                    Span::raw("  "),
                    Span::styled(tag.to_string(), theme::name_style(t, is_selected, false)),
                    Span::raw("  "),
                    Span::styled(size, dim),
                ],
                is_selected,
                t,
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
    render_scrollable_lines(f, content_area, lines, selected, t);
}

fn render_volumes_section(f: &mut Frame, area: Rect, app: &App) {
    let active = app.sidebar_section == SidebarSection::Volumes;
    let focused = active && app.focus == Focus::Sidebar;
    let content_area = render_section_header(f, area, Cow::Borrowed("VOLUMES"), focused, app);

    let t = &app.theme;
    let max_name = (content_area.width as usize).saturating_sub(4).max(6);
    let filtered = app.filtered_volumes();
    let lines: Vec<Line> = filtered
        .iter()
        .map(|(orig_idx, volume)| {
            let display_name = truncate_name(&volume.name, max_name);
            let is_selected = active && app.selected_index == *orig_idx;
            item_line(
                vec![
                    Span::raw("  "),
                    Span::styled(display_name, theme::name_style(t, is_selected, false)),
                ],
                is_selected,
                t,
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
    render_scrollable_lines(f, content_area, lines, selected, t);
}

fn render_networks_section(f: &mut Frame, area: Rect, app: &App) {
    let active = app.sidebar_section == SidebarSection::Networks;
    let focused = active && app.focus == Focus::Sidebar;
    let content_area = render_section_header(f, area, Cow::Borrowed("NETWORKS"), focused, app);

    let t = &app.theme;
    let dim = theme::dim_label(t);
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
                    Span::raw("  "),
                    Span::styled(display_name, theme::name_style(t, is_selected, false)),
                    Span::raw("  "),
                    Span::styled(driver.to_string(), dim),
                ],
                is_selected,
                t,
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
    render_scrollable_lines(f, content_area, lines, selected, t);
}
