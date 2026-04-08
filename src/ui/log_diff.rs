use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};
use std::collections::HashSet;

pub fn render_log_diff(f: &mut Frame, area: Rect, app: &App) {
    let container_id = match app.selected_container_id() {
        Some(id) => id,
        None => return,
    };

    let current = app.logs.get(container_id).map(|l| l.as_slice()).unwrap_or(&[]);
    let snapshot = app.log_snapshot.as_deref().unwrap_or(&[]);

    if current.is_empty() && snapshot.is_empty() {
        f.render_widget(
            Paragraph::new("No logs to compare")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    let snapshot_set: HashSet<&str> = snapshot.iter().map(|s| s.as_str()).collect();

    let height = area.height as usize;

    let lines: Vec<Line> = current
        .iter()
        .map(|line| {
            if snapshot_set.contains(line.as_str()) {
                // Existed in snapshot -- show dim
                Line::from(Span::styled(line.clone(), Style::default().fg(Color::DarkGray)))
            } else {
                // New since snapshot -- show green with + prefix
                Line::from(vec![
                    Span::styled("+ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::styled(line.clone(), Style::default().fg(Color::Green)),
                ])
            }
        })
        .collect();

    let paragraph = Paragraph::new(lines.clone()).wrap(Wrap { trim: false });

    // Auto-scroll to bottom
    let w = area.width as usize;
    let total_visual: usize = lines
        .iter()
        .map(|line| {
            let line_width: usize = line.spans.iter().map(|s| s.content.len()).sum();
            if w == 0 {
                1
            } else {
                line_width.max(1).div_ceil(w)
            }
        })
        .sum();

    let scroll_y = total_visual.saturating_sub(height) as u16;
    f.render_widget(paragraph.scroll((scroll_y, 0)), area);
}
