use crate::app::App;
use crate::ui::theme;
use ratatui::layout::Constraint;
use ratatui::prelude::*;
use ratatui::widgets::{Cell, Paragraph, Row, Table};

pub fn render_top(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let top_data = match &app.container_top {
        Some(data) if data.len() > 1 => data,
        _ => {
            f.render_widget(
                Paragraph::new("No processes (container may not be running)")
                    .style(theme::dim_label(t))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let header_cell = theme::header_label(t);
    let headers = &top_data[0];
    let header_row = Row::new(headers.iter().map(|h| Cell::from(format!(" {}", h)).style(header_cell)));

    let rows: Vec<Row> = top_data[1..]
        .iter()
        .map(|process| {
            Row::new(
                process
                    .iter()
                    .map(|col| Cell::from(format!(" {}", col)).style(Style::default().fg(t.fg))),
            )
        })
        .collect();

    let widths: Vec<Constraint> = headers.iter().map(|_| Constraint::Min(8)).collect();

    let table = Table::new(rows, widths).header(header_row);
    f.render_widget(table, area);
}
