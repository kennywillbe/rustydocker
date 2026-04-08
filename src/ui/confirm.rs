use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render_confirm(f: &mut Frame, area: Rect, app: &App) {
    let pending = match &app.pending_confirm {
        Some(p) => p,
        None => return,
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::raw(format!("  {}", pending.message))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" confirm    "),
            Span::styled("n/Esc", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" cancel"),
        ]),
    ];

    let popup_area = super::centered_rect(area, 42, text.len() as u16 + 2);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm ")
        .title_style(Style::default().fg(Color::Yellow))
        .border_style(Style::default().fg(Color::Yellow));

    f.render_widget(Clear, popup_area);
    f.render_widget(Paragraph::new(text).block(block), popup_area);
}
