use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render_bulk(f: &mut Frame, area: Rect, _app: &App) {
    let text = vec![
        Line::from(Span::styled(
            "Bulk Commands",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  s", Style::default().fg(Color::Yellow)),
            Span::raw(" — Stop all containers"),
        ]),
        Line::from(vec![
            Span::styled("  r", Style::default().fg(Color::Yellow)),
            Span::raw(" — Remove stopped containers"),
        ]),
        Line::from(vec![
            Span::styled("  c", Style::default().fg(Color::Yellow)),
            Span::raw(" — Prune containers"),
        ]),
        Line::from(vec![
            Span::styled("  i", Style::default().fg(Color::Yellow)),
            Span::raw(" — Prune dangling images"),
        ]),
        Line::from(vec![
            Span::styled("  v", Style::default().fg(Color::Yellow)),
            Span::raw(" — Prune unused volumes"),
        ]),
        Line::from(vec![
            Span::styled("  n", Style::default().fg(Color::Yellow)),
            Span::raw(" — Prune networks"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" — Close"),
        ]),
    ];

    let popup_area = super::centered_rect(area, 40, text.len() as u16 + 2);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Bulk ")
        .title_style(Style::default().fg(Color::Cyan))
        .border_style(Style::default().fg(Color::Cyan));

    f.render_widget(Clear, popup_area);
    f.render_widget(Paragraph::new(text).block(block), popup_area);
}
