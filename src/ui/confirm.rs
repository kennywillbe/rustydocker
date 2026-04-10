use crate::app::App;
use crate::ui::theme::{self, PopupKind};
use ratatui::prelude::*;
use ratatui::widgets::{Clear, Paragraph};

pub fn render_confirm(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let pending = match &app.pending_confirm {
        Some(p) => p,
        None => return,
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", pending.message),
            Style::default().fg(t.fg_bright),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y", Style::default().fg(t.ok).add_modifier(Modifier::BOLD)),
            Span::styled(" confirm    ", Style::default().fg(t.fg)),
            Span::styled("n/Esc", Style::default().fg(t.err).add_modifier(Modifier::BOLD)),
            Span::styled(" cancel", Style::default().fg(t.fg)),
        ]),
    ];

    let popup_area = super::centered_rect(area, 46, text.len() as u16 + 2);
    f.render_widget(Clear, popup_area);
    f.render_widget(
        Paragraph::new(text)
            .block(theme::popup_block(t, " CONFIRM ", PopupKind::Danger))
            .style(Style::default().bg(t.bg_raised)),
        popup_area,
    );
}
