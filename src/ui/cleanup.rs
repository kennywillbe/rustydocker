use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render_cleanup(f: &mut Frame, area: Rect, app: &App) {
    let dangling_images = app
        .images
        .iter()
        .filter(|img| img.repo_tags.is_empty() || img.repo_tags.iter().all(|t| t == "<none>:<none>"))
        .count();

    let total_image_size: f64 = app.images.iter().map(|img| img.size as f64 / 1_048_576.0).sum();

    let text = vec![
        Line::from(Span::styled(
            "Disk Cleanup",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!(
            "  Images: {} total ({:.0} MB)",
            app.images.len(),
            total_image_size
        )),
        Line::from(format!("  Dangling images: {}", dangling_images)),
        Line::from(format!("  Volumes: {}", app.volumes.len())),
        Line::from(""),
        Line::from(Span::styled("Actions:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("  i", Style::default().fg(Color::Yellow)),
            Span::raw(" — Prune dangling images"),
        ]),
        Line::from(vec![
            Span::styled("  v", Style::default().fg(Color::Yellow)),
            Span::raw(" — Prune unused volumes"),
        ]),
        Line::from(vec![
            Span::styled("  Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" — Close"),
        ]),
    ];

    let popup_area = super::centered_rect(area, 45, text.len() as u16 + 2);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Cleanup ")
        .title_style(Style::default().fg(Color::Red))
        .border_style(Style::default().fg(Color::Red));

    f.render_widget(Clear, popup_area);
    f.render_widget(Paragraph::new(text).block(block), popup_area);
}
