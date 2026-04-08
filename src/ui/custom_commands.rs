use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render_custom_commands(f: &mut Frame, area: Rect, app: &App) {
    let mut text = vec![
        Line::from(Span::styled(
            "Custom Commands",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (i, cmd) in app.custom_commands.iter().enumerate() {
        text.push(Line::from(vec![
            Span::styled(format!("  {}", i + 1), Style::default().fg(Color::Yellow)),
            Span::raw(format!(" — {}", cmd.name)),
        ]));
    }

    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("  Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" — Close"),
    ]));

    let popup_area = super::centered_rect(area, 45, text.len() as u16 + 2);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Commands ")
        .title_style(Style::default().fg(Color::Cyan))
        .border_style(Style::default().fg(Color::Cyan));

    f.render_widget(Clear, popup_area);
    f.render_widget(Paragraph::new(text).block(block), popup_area);
}
