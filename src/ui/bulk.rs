use crate::app::App;
use crate::ui::theme::{self, PopupKind};
use ratatui::prelude::*;
use ratatui::widgets::{Clear, Paragraph};

pub fn render_bulk(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let key = Style::default().fg(t.accent_primary).add_modifier(Modifier::BOLD);
    let value = Style::default().fg(t.fg);

    let row = |k: &str, d: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {:4}", k), key),
            Span::styled(d.to_string(), value),
        ])
    };

    let text = vec![
        Line::from(Span::styled(" BULK COMMANDS", theme::header_label(t))),
        Line::from(""),
        row("s", "Stop all containers"),
        row("r", "Remove stopped containers"),
        row("c", "Prune containers"),
        row("i", "Prune dangling images"),
        row("v", "Prune unused volumes"),
        row("n", "Prune networks"),
        Line::from(""),
        row("Esc", "Close"),
    ];

    let popup_area = super::centered_rect(area, 44, text.len() as u16 + 2);
    f.render_widget(Clear, popup_area);
    f.render_widget(
        Paragraph::new(text)
            .block(theme::popup_block(t, " BULK ", PopupKind::Info))
            .style(Style::default().bg(t.bg_raised)),
        popup_area,
    );
}
