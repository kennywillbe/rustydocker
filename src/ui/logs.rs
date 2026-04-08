use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};
use regex::RegexBuilder;
use unicode_width::UnicodeWidthStr;

fn log_level_style(line: &str) -> Style {
    let lower = line.to_lowercase();

    if lower.contains("error")
        || lower.contains("err]")
        || lower.contains("fatal")
        || lower.contains("panic")
        || lower.contains("critical")
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

fn style_log_line<'a>(line: &'a str, search_re: Option<&regex::Regex>) -> Line<'a> {
    let base_style = log_level_style(line);
    let highlight_style = Style::default().bg(Color::Yellow).fg(Color::Black);

    if let Some(re) = search_re {
        if re.is_match(line) {
            let mut spans = Vec::new();
            let mut last_end = 0;
            for m in re.find_iter(line) {
                if m.start() > last_end {
                    spans.push(Span::styled(line[last_end..m.start()].to_string(), base_style));
                }
                spans.push(Span::styled(line[m.start()..m.end()].to_string(), highlight_style));
                last_end = m.end();
            }
            if last_end < line.len() {
                spans.push(Span::styled(line[last_end..].to_string(), base_style));
            }
            return Line::from(spans);
        }
    }

    // Colorize timestamp prefix if present
    if let Some(ts_end) = find_timestamp_end(line) {
        let (timestamp, rest) = line.split_at(ts_end);
        Line::from(vec![
            Span::styled(timestamp.to_string(), Style::default().fg(Color::DarkGray)),
            Span::styled(rest.to_string(), base_style),
        ])
    } else {
        Line::from(Span::styled(line.to_string(), base_style))
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

pub fn render_all_logs(f: &mut Frame, area: Rect, app: &App) {
    let height = area.height as usize;

    // Collect all log lines from all containers with name prefix
    let mut all_lines: Vec<(String, String)> = Vec::new();
    for container in &app.containers {
        let name = container
            .names
            .as_ref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let id = container.id.as_deref().unwrap_or("");
        if let Some(logs) = app.logs.get(id) {
            for line in logs {
                all_lines.push((name.clone(), line.clone()));
            }
        }
    }

    if all_lines.is_empty() {
        let msg = Paragraph::new("No logs from any container")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(msg, area);
        return;
    }

    // Keep last N lines
    let max_lines = 500;
    if all_lines.len() > max_lines {
        all_lines.drain(..all_lines.len() - max_lines);
    }

    // Generate colors for container names (cycle through a palette)
    let colors = [
        Color::Cyan,
        Color::Green,
        Color::Yellow,
        Color::Magenta,
        Color::Blue,
        Color::Red,
    ];
    let mut name_colors: std::collections::HashMap<String, Color> = std::collections::HashMap::new();
    let mut color_idx = 0;
    for (name, _) in &all_lines {
        if !name_colors.contains_key(name) {
            name_colors.insert(name.clone(), colors[color_idx % colors.len()]);
            color_idx += 1;
        }
    }

    let lines: Vec<Line> = all_lines
        .iter()
        .map(|(name, line)| {
            let color = name_colors.get(name).copied().unwrap_or(Color::White);
            let short_name = if name.len() > 12 {
                format!("{}…", &name[..11])
            } else {
                format!("{:>12}", name)
            };
            Line::from(vec![
                Span::styled(format!("{} │ ", short_name), Style::default().fg(color)),
                Span::styled(line.to_string(), Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });

    // Auto-scroll to bottom
    let total_visual: usize = all_lines.len();
    let scroll_y = total_visual.saturating_sub(height) as u16;

    f.render_widget(paragraph.scroll((scroll_y, 0)), area);
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

    let empty: Vec<String> = vec![];
    let logs = app.logs.get(container_id).unwrap_or(&empty);

    if logs.is_empty() {
        let msg = Paragraph::new("Waiting for logs...")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(msg, area);
        return;
    }

    let height = area.height as usize;

    // Compile regex once, reuse for all lines
    let search_re = app.log_search.as_ref().and_then(|q| {
        if q.is_empty() {
            return None;
        }
        RegexBuilder::new(q)
            .case_insensitive(true)
            .build()
            .or_else(|_| RegexBuilder::new(&regex::escape(q)).case_insensitive(true).build())
            .ok()
    });

    let lines: Vec<Line> = logs
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            let mut styled = style_log_line(line, search_re.as_ref());
            if app.log_bookmarks.contains(&idx) {
                let mut spans = vec![Span::styled("▶ ", Style::default().fg(Color::Yellow))];
                spans.extend(styled.spans);
                styled = Line::from(spans);
            }
            styled
        })
        .collect();

    // Calculate total visual lines (after wrapping) to auto-scroll to bottom
    let w = area.width as usize;
    let total_visual: usize = lines
        .iter()
        .map(|line| {
            let line_width: usize = line
                .spans
                .iter()
                .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
                .sum();
            if w == 0 {
                1
            } else {
                line_width.max(1).div_ceil(w)
            }
        })
        .sum();

    let scroll_y = if app.log_scroll_offset == 0 {
        // Auto-scroll: jump to bottom
        total_visual.saturating_sub(height) as u16
    } else {
        // Manual scroll: offset from bottom
        total_visual
            .saturating_sub(height)
            .saturating_sub(app.log_scroll_offset as usize) as u16
    };

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph.scroll((scroll_y, 0)), area);
}
