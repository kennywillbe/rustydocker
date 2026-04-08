use crate::app::App;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render_stats_compare(f: &mut Frame, area: Rect, app: &App) {
    let selected_id = match app.selected_container_id() {
        Some(id) => id.to_string(),
        None => return,
    };
    let compare_id = match &app.compare_container_id {
        Some(id) => id.clone(),
        None => return,
    };

    // Split horizontally
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(area);

    // Left: selected container
    render_half(f, halves[0], app, &selected_id, true);
    // Right: compare container
    render_half(f, halves[1], app, &compare_id, false);
}

fn render_half(f: &mut Frame, area: Rect, app: &App, container_id: &str, is_left: bool) {
    let name = app
        .containers
        .iter()
        .find(|c| c.id.as_deref() == Some(container_id))
        .and_then(|c| c.names.as_ref())
        .and_then(|n| n.first())
        .map(|n| n.trim_start_matches('/').to_string())
        .unwrap_or_else(|| container_id[..12.min(container_id.len())].to_string());

    let history = match app.stats.get(container_id) {
        Some(h) => h,
        None => {
            f.render_widget(
                Paragraph::new(format!("{}: No stats", name))
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let border_color = if is_left { Color::Cyan } else { Color::Yellow };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {} ", name),
            Style::default().fg(border_color).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split vertically for CPU, MEM, NET
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(inner);

    super::stats_panel::render_cpu(f, sections[0], history);
    super::stats_panel::render_mem(f, sections[1], history);
    super::stats_panel::render_net(f, sections[2], history);
}
