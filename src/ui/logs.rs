use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

fn log_level_style(line: &str) -> Style {
    let lower = line.to_lowercase();

    if lower.contains("error") || lower.contains("err]") || lower.contains("fatal")
        || lower.contains("panic") || lower.contains("critical")
    {
        Style::default().fg(Color::Rgb(255, 80, 80)) // bright red
    } else if lower.contains("warn") || lower.contains("wrn]") {
        Style::default().fg(Color::Rgb(255, 200, 50)) // bright yellow
    } else if lower.contains("debug") || lower.contains("dbg]") || lower.contains("trace") {
        Style::default().fg(Color::DarkGray)
    } else if lower.contains("info") || lower.contains("inf]") {
        Style::default().fg(Color::Rgb(80, 220, 120)) // bright green
    } else {
        Style::default().fg(Color::White)
    }
}

fn style_log_line<'a>(line: &'a str, search: &Option<String>) -> Line<'a> {
    // Search highlight takes priority
    if let Some(ref query) = search {
        if !query.is_empty() && line.to_lowercase().contains(&query.to_lowercase()) {
            return Line::from(Span::styled(
                line.to_string(),
                Style::default().bg(Color::Yellow).fg(Color::Black),
            ));
        }
    }

    // Try to parse structured logs (JSON or key=value)
    let style = log_level_style(line);

    // Colorize timestamp prefix if present (common formats: 2025-01-01T00:00:00, [2025-01-01])
    if let Some(ts_end) = find_timestamp_end(line) {
        let (timestamp, rest) = line.split_at(ts_end);
        Line::from(vec![
            Span::styled(timestamp.to_string(), Style::default().fg(Color::DarkGray)),
            Span::styled(rest.to_string(), style),
        ])
    } else {
        Line::from(Span::styled(line.to_string(), style))
    }
}

fn find_timestamp_end(line: &str) -> Option<usize> {
    // Match common timestamp patterns at the start of a line
    let trimmed = line.trim_start();
    let offset = line.len() - trimmed.len();

    // ISO format: 2025-01-01T00:00:00 or 2025-01-01 00:00:00
    if trimmed.len() >= 19 {
        let bytes = trimmed.as_bytes();
        if bytes.len() >= 4
            && bytes[0].is_ascii_digit()
            && bytes[1].is_ascii_digit()
            && bytes[2].is_ascii_digit()
            && bytes[3].is_ascii_digit()
            && (bytes[4] == b'-' || bytes[4] == b'/')
        {
            // Find end of timestamp (up to timezone or space after seconds)
            for (i, &b) in bytes.iter().enumerate().skip(19) {
                if b == b' ' || b == b'|' || b == b']' {
                    return Some(offset + i + 1);
                }
            }
            // If line is exactly a timestamp
            if trimmed.len() <= 35 {
                return None; // Whole line is timestamp, don't split
            }
            return Some(offset + std::cmp::min(30, trimmed.len()));
        }
    }

    // Bracketed: [2025-01-01 00:00:00] or [INFO]
    if trimmed.starts_with('[') {
        if let Some(end) = trimmed.find(']') {
            // Check if it looks like a timestamp inside brackets
            let inside = &trimmed[1..end];
            if inside.len() >= 10 && inside.as_bytes()[0].is_ascii_digit() {
                return Some(offset + end + 1);
            }
        }
    }

    None
}

pub fn render_logs(f: &mut Frame, area: Rect, app: &App) {
    let container_id = match app.selected_container_id() {
        Some(id) => id,
        None => {
            let msg = Paragraph::new("No container selected")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            f.render_widget(msg, area);
            return;
        }
    };

    let logs = app.logs.get(container_id).cloned().unwrap_or_default();

    if logs.is_empty() {
        let msg = Paragraph::new("Waiting for logs...")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(msg, area);
        return;
    }

    let search = &app.log_search;
    let height = area.height as usize;

    let lines: Vec<Line> = logs
        .iter()
        .map(|line| style_log_line(line, search))
        .collect();

    let paragraph = Paragraph::new(lines.clone()).wrap(Wrap { trim: false });

    // Calculate total visual lines (after wrapping) to auto-scroll to bottom
    let w = area.width as usize;
    let total_visual: usize = lines.iter().map(|line| {
        let line_width: usize = line.spans.iter().map(|s| UnicodeWidthStr::width(s.content.as_ref())).sum();
        if w == 0 { 1 } else { line_width.max(1).div_ceil(w) }
    }).sum();

    let scroll_y = if app.log_scroll_offset == 0 {
        // Auto-scroll: jump to bottom
        total_visual.saturating_sub(height) as u16
    } else {
        // Manual scroll: offset from bottom
        total_visual
            .saturating_sub(height)
            .saturating_sub(app.log_scroll_offset as usize) as u16
    };

    f.render_widget(paragraph.scroll((scroll_y, 0)), area);
}
