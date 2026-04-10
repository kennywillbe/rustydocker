use crate::app::App;
use crate::ui::theme::{self, Theme};
use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};
use regex::RegexBuilder;
use std::sync::OnceLock;
use unicode_width::UnicodeWidthStr;

fn log_level_style(line: &str, t: &Theme) -> Style {
    let lower = line.to_lowercase();

    if lower.contains("error")
        || lower.contains("err]")
        || lower.contains("fatal")
        || lower.contains("panic")
        || lower.contains("critical")
    {
        Style::default().fg(t.err)
    } else if lower.contains("warn") || lower.contains("wrn]") {
        Style::default().fg(t.warn)
    } else if lower.contains("debug") || lower.contains("dbg]") || lower.contains("trace") {
        Style::default().fg(t.fg_muted)
    } else if lower.contains("info") || lower.contains("inf]") {
        Style::default().fg(t.info)
    } else {
        Style::default().fg(t.fg)
    }
}

/// Shared regex for HTTP status codes. Matches a 3-digit code that is not
/// adjacent to other digits (avoids matching IDs, timestamps, etc).
fn http_status_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(?:^|\s)([1-5]\d{2})(?:\s|$)").unwrap())
}

/// Style a single 3-digit HTTP status code.
fn http_status_style(code: &str, t: &Theme) -> Option<Style> {
    let first = code.chars().next()?;
    Some(match first {
        '2' => Style::default().fg(t.ok).add_modifier(Modifier::BOLD),
        '3' => Style::default().fg(t.warn),
        '4' | '5' => Style::default().fg(t.err).add_modifier(Modifier::BOLD),
        _ => return None,
    })
}

/// Split a line into spans so embedded HTTP status codes get their own
/// colour on top of the base level style. Returns borrowed spans — no
/// allocation per line in the common case.
fn colorize_http_statuses<'a>(text: &'a str, base: Style, t: &Theme) -> Vec<Span<'a>> {
    let re = http_status_regex();
    let mut spans: Vec<Span<'a>> = Vec::new();
    let mut last = 0usize;

    for cap in re.captures_iter(text) {
        let whole = cap.get(0).unwrap();
        let code = cap.get(1).unwrap();
        if whole.start() > last {
            spans.push(Span::styled(&text[last..whole.start()], base));
        }
        // The regex captures a leading space before the digits — keep it
        // on the base style so only the three digits get the status colour.
        let leading = &text[whole.start()..code.start()];
        if !leading.is_empty() {
            spans.push(Span::styled(leading, base));
        }
        let style = http_status_style(code.as_str(), t).unwrap_or(base);
        spans.push(Span::styled(&text[code.start()..code.end()], style));
        last = code.end();
    }
    if last < text.len() {
        spans.push(Span::styled(&text[last..], base));
    }
    if spans.is_empty() {
        spans.push(Span::styled(text, base));
    }
    spans
}

fn style_log_line<'a>(line: &'a str, search_re: Option<&regex::Regex>, t: &Theme) -> Line<'a> {
    let base_style = log_level_style(line, t);
    // `on_accent` contrasts with the accent across both themes.
    let highlight_style = Style::default().bg(t.accent_primary).fg(t.on_accent);

    // Search match highlight wins over HTTP colouring.
    if let Some(re) = search_re {
        if re.is_match(line) {
            let mut spans = Vec::new();
            let mut last_end = 0;
            for m in re.find_iter(line) {
                if m.start() > last_end {
                    spans.push(Span::styled(&line[last_end..m.start()], base_style));
                }
                spans.push(Span::styled(&line[m.start()..m.end()], highlight_style));
                last_end = m.end();
            }
            if last_end < line.len() {
                spans.push(Span::styled(&line[last_end..], base_style));
            }
            return Line::from(spans);
        }
    }

    let (timestamp, body) = match find_timestamp_end(line) {
        Some(end) => (Some(&line[..end]), &line[end..]),
        None => (None, line),
    };

    let mut spans: Vec<Span> = Vec::new();
    if let Some(ts) = timestamp {
        spans.push(Span::styled(ts, theme::dim_label(t)));
    }
    spans.extend(colorize_http_statuses(body, base_style, t));
    Line::from(spans)
}

fn find_timestamp_end(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let offset = line.len() - trimmed.len();

    // ISO: 2025-01-01T00:00:00 or 2025-01-01 00:00:00
    if trimmed.len() >= 19 {
        let bytes = trimmed.as_bytes();
        if bytes.len() >= 4
            && bytes[0].is_ascii_digit()
            && bytes[1].is_ascii_digit()
            && bytes[2].is_ascii_digit()
            && bytes[3].is_ascii_digit()
            && (bytes[4] == b'-' || bytes[4] == b'/')
        {
            for (i, &b) in bytes.iter().enumerate().skip(19) {
                if b == b' ' || b == b'|' || b == b']' {
                    return Some(offset + i + 1);
                }
            }
            if trimmed.len() <= 35 {
                return None;
            }
            return Some(offset + std::cmp::min(30, trimmed.len()));
        }
    }

    // Bracketed: [2025-01-01 00:00:00] or [12:34:56]
    if trimmed.starts_with('[') {
        if let Some(end) = trimmed.find(']') {
            let inside = &trimmed[1..end];
            if inside.len() >= 5 && inside.as_bytes()[0].is_ascii_digit() {
                return Some(offset + end + 1);
            }
        }
    }

    None
}

pub fn render_all_logs(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let height = area.height as usize;
    const MAX_LINES: usize = 500;

    // Walk containers, capturing each container's display name + a slice of
    // its recent log lines. No per-line tuple clones.
    let mut sources: Vec<(String, &[String])> = Vec::new();
    let mut total: usize = 0;
    for container in &app.containers {
        let id = container.id.as_deref().unwrap_or("");
        let Some(logs) = app.logs.get(id) else { continue };
        if logs.is_empty() {
            continue;
        }
        let name = container
            .names
            .as_ref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| "unknown".to_string());
        total += logs.len();
        sources.push((name, logs.as_slice()));
    }

    if total == 0 {
        let msg = Paragraph::new("No logs from any container")
            .style(theme::dim_label(t))
            .alignment(Alignment::Center);
        f.render_widget(msg, area);
        return;
    }

    let prefix_style = Style::default().fg(t.accent_header);

    // Produce owned lines — each row borrows from `sources` via String.
    // Trim from the oldest side if we exceed MAX_LINES.
    let skip = total.saturating_sub(MAX_LINES);
    let mut lines: Vec<Line> = Vec::with_capacity(total.min(MAX_LINES));
    let mut skipped = 0usize;
    for (name, logs) in &sources {
        let short = if name.chars().count() > 12 {
            let cut: String = name.chars().take(11).collect();
            format!("[{}…]", cut)
        } else {
            format!("[{}]", name)
        };
        for line in *logs {
            if skipped < skip {
                skipped += 1;
                continue;
            }
            let base = log_level_style(line, t);
            let mut spans: Vec<Span> = Vec::with_capacity(4);
            spans.push(Span::styled(short.clone(), prefix_style));
            spans.push(Span::raw(" "));
            spans.extend(colorize_http_statuses(line, base, t));
            lines.push(Line::from(spans));
        }
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let visible = total.min(MAX_LINES);
    let scroll_y = visible.saturating_sub(height) as u16;
    f.render_widget(paragraph.scroll((scroll_y, 0)), area);
}

pub fn render_logs(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let container_id = match app.selected_container_id() {
        Some(id) => id,
        None => {
            let msg = Paragraph::new("No container selected")
                .style(theme::dim_label(t))
                .alignment(Alignment::Center);
            f.render_widget(msg, area);
            return;
        }
    };

    let empty: Vec<String> = vec![];
    let logs = app.logs.get(container_id).unwrap_or(&empty);

    if logs.is_empty() {
        let msg = Paragraph::new("Waiting for logs…")
            .style(theme::dim_label(t))
            .alignment(Alignment::Center);
        f.render_widget(msg, area);
        return;
    }

    let height = area.height as usize;

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
            let mut styled = style_log_line(line, search_re.as_ref(), t);
            if app.log_bookmarks.contains(&idx) {
                let mut spans = vec![Span::styled("▶ ", Style::default().fg(t.accent_primary))];
                spans.extend(styled.spans);
                styled = Line::from(spans);
            }
            styled
        })
        .collect();

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
        total_visual.saturating_sub(height) as u16
    } else {
        total_visual
            .saturating_sub(height)
            .saturating_sub(app.log_scroll_offset as usize) as u16
    };

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph.scroll((scroll_y, 0)), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_status_regex_extracts_codes() {
        let re = http_status_regex();
        let line = "GET /users 200 12ms";
        let codes: Vec<&str> = re.captures_iter(line).map(|c| c.get(1).unwrap().as_str()).collect();
        assert_eq!(codes, vec!["200"]);
    }

    #[test]
    fn http_status_regex_ignores_embedded_digits() {
        let re = http_status_regex();
        // "v1200" should not produce a match because 200 is not preceded by whitespace
        let line = "build v1200 complete";
        let count = re.captures_iter(line).count();
        assert_eq!(count, 0);
    }

    #[test]
    fn http_status_style_pick() {
        let t = Theme::EMBER;
        assert_eq!(http_status_style("200", &t).unwrap().fg, Some(t.ok));
        assert_eq!(http_status_style("404", &t).unwrap().fg, Some(t.err));
        assert_eq!(http_status_style("301", &t).unwrap().fg, Some(t.warn));
    }
}
