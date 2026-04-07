use crate::app::App;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::*;
use ratatui::symbols;
use ratatui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph};

fn format_bytes(mb: f64) -> String {
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{:.0} MB", mb)
    }
}

pub fn render_stats(f: &mut Frame, area: Rect, app: &App) {
    let container_id = match app.selected_container_id() {
        Some(id) => id,
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

    let history = match app.stats.get(container_id) {
        Some(h) => h,
        None => {
            f.render_widget(
                Paragraph::new("Waiting for stats...")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(area);

    render_cpu(f, sections[0], history);
    render_mem(f, sections[1], history);
    render_net(f, sections[2], history);
}

fn to_chart_data(values: &[f64]) -> Vec<(f64, f64)> {
    values.iter().enumerate().map(|(i, v)| (i as f64, *v)).collect()
}

fn render_cpu(f: &mut Frame, area: Rect, history: &crate::app::StatsHistory) {
    let current = history.cpu.last().unwrap_or(&0.0);
    let data = to_chart_data(&history.cpu);

    let title = Line::from(vec![
        Span::styled(" CPU ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{:.1}% ", current), Style::default().fg(Color::White)),
    ]);

    let dataset = Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Green))
        .data(&data);

    let chart = Chart::new(vec![dataset])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        )
        .x_axis(Axis::default().bounds([0.0, 60.0]))
        .y_axis(
            Axis::default()
                .bounds([0.0, 100.0])
                .labels(vec![
                    Span::styled("0", Style::default().fg(Color::DarkGray)),
                    Span::styled("100%", Style::default().fg(Color::DarkGray)),
                ]),
        );

    f.render_widget(chart, area);
}

fn render_mem(f: &mut Frame, area: Rect, history: &crate::app::StatsHistory) {
    let current = history.memory.last().unwrap_or(&0.0);
    let limit = history.memory_limit_mb;
    let pct = if limit > 0.0 { current / limit * 100.0 } else { 0.0 };

    let title = Line::from(vec![
        Span::styled(" MEM ", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{}/{} ({:.1}%) ", format_bytes(*current), format_bytes(limit), pct),
            Style::default().fg(Color::White),
        ),
    ]);

    // Scale to percentage of limit
    let pct_data: Vec<f64> = if limit > 0.0 {
        history.memory.iter().map(|v| v / limit * 100.0).collect()
    } else {
        history.memory.clone()
    };
    let data = to_chart_data(&pct_data);

    let y_max = if limit > 0.0 { 100.0 } else { pct_data.iter().cloned().fold(1.0_f64, f64::max) };

    let dataset = Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Blue))
        .data(&data);

    let y_label = if limit > 0.0 {
        format_bytes(limit)
    } else {
        format!("{:.0}", y_max)
    };

    let chart = Chart::new(vec![dataset])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        )
        .x_axis(Axis::default().bounds([0.0, 60.0]))
        .y_axis(
            Axis::default()
                .bounds([0.0, y_max])
                .labels(vec![
                    Span::styled("0", Style::default().fg(Color::DarkGray)),
                    Span::styled(y_label, Style::default().fg(Color::DarkGray)),
                ]),
        );

    f.render_widget(chart, area);
}

fn render_net(f: &mut Frame, area: Rect, history: &crate::app::StatsHistory) {
    let rx_current = history.net_rx.last().unwrap_or(&0.0) / 1024.0;
    let tx_current = history.net_tx.last().unwrap_or(&0.0) / 1024.0;

    let title = Line::from(vec![
        Span::styled(" NET ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::styled(format!("RX {:.1} KB/s ", rx_current), Style::default().fg(Color::Cyan)),
        Span::styled(format!("TX {:.1} KB/s ", tx_current), Style::default().fg(Color::Yellow)),
    ]);

    let rx_kb: Vec<f64> = history.net_rx.iter().map(|v| v / 1024.0).collect();
    let tx_kb: Vec<f64> = history.net_tx.iter().map(|v| v / 1024.0).collect();
    let rx_data = to_chart_data(&rx_kb);
    let tx_data = to_chart_data(&tx_kb);

    let y_max = rx_kb.iter().chain(tx_kb.iter()).cloned().fold(1.0_f64, f64::max);

    let datasets = vec![
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&rx_data),
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Yellow))
            .data(&tx_data),
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        )
        .x_axis(Axis::default().bounds([0.0, 60.0]))
        .y_axis(
            Axis::default()
                .bounds([0.0, y_max])
                .labels(vec![
                    Span::styled("0", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{:.0} KB/s", y_max), Style::default().fg(Color::DarkGray)),
                ]),
        );

    f.render_widget(chart, area);
}
