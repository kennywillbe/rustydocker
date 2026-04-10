use crate::app::App;
use crate::ui::theme::{self, PopupKind};
use ratatui::prelude::*;
use ratatui::widgets::{Clear, Paragraph};

pub fn render_cleanup(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let dangling_images = app
        .images
        .iter()
        .filter(|img| img.repo_tags.is_empty() || img.repo_tags.iter().all(|t| t == "<none>:<none>"))
        .count();

    let total_image_size: f64 = app.images.iter().map(|img| img.size as f64 / 1_048_576.0).sum();

    let key_style = Style::default().fg(t.accent_primary).add_modifier(Modifier::BOLD);
    let label = theme::header_label(t);
    let value = Style::default().fg(t.fg);
    let label_col = theme::label_cell(t);

    let text = vec![
        Line::from(Span::styled(" DISK CLEANUP", label)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Images           ", label_col),
            Span::styled(
                format!("{} total ({:.0} MB)", app.images.len(), total_image_size),
                value,
            ),
        ]),
        Line::from(vec![
            Span::styled("  Dangling images  ", label_col),
            Span::styled(dangling_images.to_string(), value),
        ]),
        Line::from(vec![
            Span::styled("  Volumes          ", label_col),
            Span::styled(app.volumes.len().to_string(), value),
        ]),
        Line::from(""),
        Line::from(Span::styled(" ACTIONS", label)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  i   ", key_style),
            Span::styled("Prune dangling images", value),
        ]),
        Line::from(vec![
            Span::styled("  v   ", key_style),
            Span::styled("Prune unused volumes", value),
        ]),
        Line::from(vec![Span::styled("  Esc ", key_style), Span::styled("Close", value)]),
    ];

    let popup_area = super::centered_rect(area, 46, text.len() as u16 + 2);
    f.render_widget(Clear, popup_area);
    f.render_widget(
        Paragraph::new(text)
            .block(theme::popup_block(t, " CLEANUP ", PopupKind::Info))
            .style(Style::default().bg(t.bg_raised)),
        popup_area,
    );
}
