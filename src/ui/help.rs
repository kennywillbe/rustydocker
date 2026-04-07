use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled("Keybindings", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled("j/k", Style::default().fg(Color::Yellow)), Span::raw("     Navigate up/down")]),
        Line::from(vec![Span::styled("Tab", Style::default().fg(Color::Yellow)), Span::raw("     Switch panel tab")]),
        Line::from(vec![Span::styled("Enter", Style::default().fg(Color::Yellow)), Span::raw("   Select/expand")]),
        Line::from(vec![Span::styled("r", Style::default().fg(Color::Yellow)), Span::raw("       Restart container")]),
        Line::from(vec![Span::styled("s", Style::default().fg(Color::Yellow)), Span::raw("       Stop container")]),
        Line::from(vec![Span::styled("u", Style::default().fg(Color::Yellow)), Span::raw("       Start container")]),
        Line::from(vec![Span::styled("d", Style::default().fg(Color::Yellow)), Span::raw("       Remove container")]),
        Line::from(vec![Span::styled("e", Style::default().fg(Color::Yellow)), Span::raw("       Exec shell")]),
        Line::from(vec![Span::styled("/", Style::default().fg(Color::Yellow)), Span::raw("       Search logs")]),
        Line::from(vec![Span::styled("x", Style::default().fg(Color::Yellow)), Span::raw("       Disk cleanup")]),
        Line::from(vec![Span::styled("?", Style::default().fg(Color::Yellow)), Span::raw("       This help")]),
        Line::from(vec![Span::styled("q", Style::default().fg(Color::Yellow)), Span::raw("       Quit")]),
    ];

    let popup_width = 40;
    let popup_height = help_text.len() as u16 + 2;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .title_style(Style::default().fg(Color::Cyan))
        .border_style(Style::default().fg(Color::Yellow));

    f.render_widget(Clear, popup_area);
    f.render_widget(Paragraph::new(help_text).block(block), popup_area);
}
