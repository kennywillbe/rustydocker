use crate::app::App;
use ratatui::layout::Constraint;
use ratatui::prelude::*;
use ratatui::widgets::{Cell, Paragraph, Row, Table};

pub fn render_top(f: &mut Frame, area: Rect, app: &App) {
    let top_data = match &app.container_top {
        Some(data) if data.len() > 1 => data,
        _ => {
            f.render_widget(
                Paragraph::new("No processes (container may not be running)")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let headers = &top_data[0];
    let header_row =
        Row::new(headers.iter().map(|h| {
            Cell::from(format!(" {}", h)).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        }));

    let rows: Vec<Row> = top_data[1..]
        .iter()
        .map(|process| {
            Row::new(
                process
                    .iter()
                    .map(|col| Cell::from(format!(" {}", col)).style(Style::default().fg(Color::White))),
            )
        })
        .collect();

    let widths: Vec<Constraint> = headers.iter().map(|_| Constraint::Min(8)).collect();

    let table = Table::new(rows, widths).header(header_row);
    f.render_widget(table, area);
}
