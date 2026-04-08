use crate::app::App;
use ratatui::layout::Constraint;
use ratatui::prelude::*;
use ratatui::widgets::{Cell, Paragraph, Row, Table};

pub fn render_env(f: &mut Frame, area: Rect, app: &App) {
    let env_vars = match &app.container_env {
        Some(vars) if !vars.is_empty() => vars,
        _ => {
            f.render_widget(
                Paragraph::new("No environment variables")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let rows: Vec<Row> = env_vars
        .iter()
        .map(|(key, value)| {
            Row::new(vec![
                Cell::from(format!(" {}", key)).style(Style::default().fg(Color::Cyan)),
                Cell::from(value.clone()).style(Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(25), Constraint::Min(20)]).header(Row::new(vec![
        Cell::from(" KEY").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("VALUE").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]));

    f.render_widget(table, area);
}
