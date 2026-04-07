use crate::app::App;
use ratatui::layout::Constraint;
use ratatui::prelude::*;
use ratatui::widgets::{Cell, Paragraph, Row, Table};

pub fn render_info(f: &mut Frame, area: Rect, app: &App) {
    let container = match app.selected_container() {
        Some(c) => c,
        None => {
            f.render_widget(
                Paragraph::new("No container selected")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let id = container
        .id
        .as_deref()
        .unwrap_or("N/A")
        .chars()
        .take(12)
        .collect::<String>();
    let image = container.image.as_deref().unwrap_or("N/A");
    let state = container.state.as_deref().unwrap_or("N/A");
    let status = container.status.as_deref().unwrap_or("N/A");

    let state_color = match state {
        "running" => Color::Green,
        "exited" => Color::Red,
        "restarting" => Color::Yellow,
        _ => Color::DarkGray,
    };

    let ports = container
        .ports
        .as_ref()
        .map(|ports| {
            ports
                .iter()
                .map(|p| {
                    let private = p.private_port;
                    let public = p.public_port.map(|pp| pp.to_string()).unwrap_or_default();
                    if public.is_empty() {
                        format!("{}", private)
                    } else {
                        format!("{}:{}", public, private)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "None".to_string());

    let created = container
        .created
        .map(|ts| {
            chrono::DateTime::from_timestamp(ts, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| ts.to_string())
        })
        .unwrap_or_else(|| "N/A".to_string());

    let names = container
        .names
        .as_ref()
        .map(|n| {
            n.iter()
                .map(|s| s.trim_start_matches('/'))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "N/A".to_string());

    let rows = vec![
        Row::new(vec![
            Cell::from(" Name").style(Style::default().fg(Color::Cyan)),
            Cell::from(names).style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Row::new(vec![
            Cell::from(" ID").style(Style::default().fg(Color::Cyan)),
            Cell::from(id),
        ]),
        Row::new(vec![
            Cell::from(" Image").style(Style::default().fg(Color::Cyan)),
            Cell::from(image.to_string()),
        ]),
        Row::new(vec![
            Cell::from(" State").style(Style::default().fg(Color::Cyan)),
            Cell::from(Span::styled(
                state.to_string(),
                Style::default()
                    .fg(state_color)
                    .add_modifier(Modifier::BOLD),
            )),
        ]),
        Row::new(vec![
            Cell::from(" Status").style(Style::default().fg(Color::Cyan)),
            Cell::from(status.to_string()),
        ]),
        Row::new(vec![
            Cell::from(" Ports").style(Style::default().fg(Color::Cyan)),
            Cell::from(ports),
        ]),
        Row::new(vec![
            Cell::from(" Created").style(Style::default().fg(Color::Cyan)),
            Cell::from(created),
        ]),
    ]
    .into_iter()
    .map(|r| r.height(1))
    .collect::<Vec<_>>();

    let table = Table::new(rows, [Constraint::Length(12), Constraint::Min(20)])
        .row_highlight_style(Style::default().bg(Color::Rgb(40, 40, 55)));

    f.render_widget(table, area);
}
