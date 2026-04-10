use crate::app::App;
use crate::ui::theme::{self, PopupKind};
use ratatui::prelude::*;
use ratatui::widgets::{Clear, Paragraph};

pub fn render_custom_commands(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let key = Style::default().fg(t.accent_primary).add_modifier(Modifier::BOLD);
    let value = Style::default().fg(t.fg);

    let mut text = vec![
        Line::from(Span::styled(" CUSTOM COMMANDS", theme::header_label(t))),
        Line::from(""),
    ];

    if app.custom_commands.is_empty() {
        text.push(Line::from(Span::styled(
            "  (none configured — add [[custom_commands]] to config.toml)",
            theme::dim_label(t),
        )));
    } else {
        for (i, cmd) in app.custom_commands.iter().enumerate() {
            text.push(Line::from(vec![
                Span::styled(format!("  {}  ", i + 1), key),
                Span::styled(cmd.name.as_str().to_string(), value),
            ]));
        }
    }

    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("  Esc  ", key),
        Span::styled("Close", value),
    ]));

    let popup_area = super::centered_rect(area, 48, text.len() as u16 + 2);
    f.render_widget(Clear, popup_area);
    f.render_widget(
        Paragraph::new(text)
            .block(theme::popup_block(t, " COMMANDS ", PopupKind::Info))
            .style(Style::default().bg(t.bg_raised)),
        popup_area,
    );
}
