use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "Keybindings",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("j/k", Style::default().fg(Color::Yellow)),
            Span::raw("     Navigate up/down"),
        ]),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw("     Switch panel tab"),
        ]),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw("   Select/expand"),
        ]),
        Line::from(vec![
            Span::styled("Space", Style::default().fg(Color::Yellow)),
            Span::raw("   Select/deselect container"),
        ]),
        Line::from(vec![
            Span::styled("*", Style::default().fg(Color::Yellow)),
            Span::raw("       Pin/unpin container"),
        ]),
        Line::from(vec![
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw("       Restart container"),
        ]),
        Line::from(vec![
            Span::styled("s", Style::default().fg(Color::Yellow)),
            Span::raw("       Stop container"),
        ]),
        Line::from(vec![
            Span::styled("u", Style::default().fg(Color::Yellow)),
            Span::raw("       Start container"),
        ]),
        Line::from(vec![
            Span::styled("d", Style::default().fg(Color::Yellow)),
            Span::raw("       Remove container"),
        ]),
        Line::from(vec![
            Span::styled("p", Style::default().fg(Color::Yellow)),
            Span::raw("       Pause/unpause container"),
        ]),
        Line::from(vec![
            Span::styled("e", Style::default().fg(Color::Yellow)),
            Span::raw("       Exec shell"),
        ]),
        Line::from(vec![
            Span::styled("a", Style::default().fg(Color::Yellow)),
            Span::raw("       Attach to container"),
        ]),
        Line::from(vec![
            Span::styled("w", Style::default().fg(Color::Yellow)),
            Span::raw("       Open in browser"),
        ]),
        Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw("       Search logs"),
        ]),
        Line::from(vec![
            Span::styled("m", Style::default().fg(Color::Yellow)),
            Span::raw("       Toggle log bookmark"),
        ]),
        Line::from(vec![
            Span::styled("n/N", Style::default().fg(Color::Yellow)),
            Span::raw("     Next/prev bookmark"),
        ]),
        Line::from(vec![
            Span::styled("T", Style::default().fg(Color::Yellow)),
            Span::raw("       Log snapshot/diff"),
        ]),
        Line::from(vec![
            Span::styled("x", Style::default().fg(Color::Yellow)),
            Span::raw("       Disk cleanup"),
        ]),
        Line::from(vec![
            Span::styled("b", Style::default().fg(Color::Yellow)),
            Span::raw("       Bulk commands menu"),
        ]),
        Line::from(vec![
            Span::styled("c", Style::default().fg(Color::Yellow)),
            Span::raw("       Custom commands menu"),
        ]),
        Line::from(vec![
            Span::styled("C", Style::default().fg(Color::Yellow)),
            Span::raw("       Compare stats with another container"),
        ]),
        Line::from(vec![
            Span::styled("+/_", Style::default().fg(Color::Yellow)),
            Span::raw("     Screen mode (normal/half/full)"),
        ]),
        Line::from(vec![
            Span::styled("?", Style::default().fg(Color::Yellow)),
            Span::raw("       This help"),
        ]),
        Line::from(vec![
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw("       Quit"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Compose",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("U", Style::default().fg(Color::Yellow)),
            Span::raw("       Compose up"),
        ]),
        Line::from(vec![
            Span::styled("D", Style::default().fg(Color::Yellow)),
            Span::raw("       Compose down"),
        ]),
        Line::from(vec![
            Span::styled("R", Style::default().fg(Color::Yellow)),
            Span::raw("       Compose restart"),
        ]),
        Line::from(vec![
            Span::styled("S", Style::default().fg(Color::Yellow)),
            Span::raw("       Export logs to file"),
        ]),
        Line::from(vec![
            Span::styled("L", Style::default().fg(Color::Yellow)),
            Span::raw("       Toggle all-logs view"),
        ]),
    ];

    let popup_area = super::centered_rect(area, 40, help_text.len() as u16 + 2);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .title_style(Style::default().fg(Color::Cyan))
        .border_style(Style::default().fg(Color::Yellow));

    f.render_widget(Clear, popup_area);
    f.render_widget(Paragraph::new(help_text).block(block), popup_area);
}
