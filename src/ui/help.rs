use crate::app::App;
use crate::ui::theme::{self, PopupKind};
use ratatui::prelude::*;
use ratatui::widgets::{Clear, Paragraph};

pub fn render_help(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let key = Style::default().fg(t.accent_primary).add_modifier(Modifier::BOLD);
    let desc = Style::default().fg(t.fg);
    let section = theme::header_label(t);

    let row = |k: &str, d: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!(" {:6} ", k), key),
            Span::styled(d.to_string(), desc),
        ])
    };

    let help_text: Vec<Line> = vec![
        Line::from(vec![Span::styled(" NAVIGATION", section)]),
        Line::from(""),
        row("j/k", "Move cursor up / down"),
        row("h/l", "Focus sidebar / main"),
        row("Tab", "Next detail tab"),
        row("1–6", "Jump to tab"),
        row("/", "Regex search in logs"),
        row("+/_", "Screen mode (normal/half/full)"),
        row("?", "This help"),
        row("q", "Quit"),
        row("Ctrl+U", "Check for / install update"),
        Line::from(""),
        Line::from(vec![Span::styled(" CONTAINER", section)]),
        Line::from(""),
        row("u", "Start"),
        row("s", "Stop"),
        row("r", "Restart"),
        row("p", "Pause / unpause"),
        row("d", "Remove (with confirm)"),
        row("e", "Exec shell"),
        row("a", "Attach"),
        row("w", "Open port in browser"),
        row("Space", "Multi-select"),
        row("*", "Pin to top"),
        row("S", "Export logs to file"),
        Line::from(""),
        Line::from(vec![Span::styled(" LOGS", section)]),
        Line::from(""),
        row("m", "Toggle log bookmark"),
        row("n/N", "Next / prev bookmark"),
        row("T", "Log snapshot / diff"),
        row("L", "All-logs view"),
        Line::from(""),
        Line::from(vec![Span::styled(" COMPOSE & BULK", section)]),
        Line::from(""),
        row("U", "compose up -d"),
        row("D", "compose down"),
        row("R", "compose restart"),
        row("b", "Bulk commands menu"),
        row("x", "Disk cleanup menu"),
        row("c", "Custom commands menu"),
        row("C", "Compare stats"),
    ];

    let popup_w: u16 = 44;
    let popup_h: u16 = help_text.len() as u16 + 2;
    let popup_area = super::centered_rect(area, popup_w, popup_h.min(area.height.saturating_sub(2)));

    f.render_widget(Clear, popup_area);
    f.render_widget(
        Paragraph::new(help_text)
            .block(theme::popup_block(t, " HELP ", PopupKind::Info))
            .style(Style::default().bg(t.bg_raised)),
        popup_area,
    );
}
