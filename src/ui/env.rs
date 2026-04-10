use crate::app::App;
use crate::ui::theme;
use ratatui::layout::Constraint;
use ratatui::prelude::*;
use ratatui::widgets::{Cell, Paragraph, Row, Table};

pub fn render_env(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let env_vars = match &app.container_env {
        Some(vars) if !vars.is_empty() => vars,
        _ => {
            f.render_widget(
                Paragraph::new("No environment variables")
                    .style(theme::dim_label(t))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let key_cell = Style::default().fg(t.accent_primary);
    let value_cell = Style::default().fg(t.fg);
    let header_cell = theme::header_label(t);

    let rows: Vec<Row> = env_vars
        .iter()
        .map(|(key, value)| {
            Row::new(vec![
                Cell::from(format!(" {}", key)).style(key_cell),
                Cell::from(value.clone()).style(value_cell),
            ])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(25), Constraint::Min(20)]).header(Row::new(vec![
        Cell::from(" KEY").style(header_cell),
        Cell::from("VALUE").style(header_cell),
    ]));

    f.render_widget(table, area);
}
