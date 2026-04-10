//! Side-by-side statistics comparison between two containers.

use crate::app::App;
use crate::ui::theme::{self as thm, sparkline_iter};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use super::stats_panel::format_bytes;

const SPARK_WIDTH: usize = 14;

pub fn render_stats_compare(f: &mut Frame, area: Rect, app: &App) {
    let selected_id = match app.selected_container_id() {
        Some(id) => id.to_string(),
        None => return,
    };
    let compare_id = match &app.compare_container_id {
        Some(id) => id.clone(),
        None => return,
    };

    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(area);

    render_half(f, halves[0], app, &selected_id, true);
    render_half(f, halves[1], app, &compare_id, false);
}

fn render_half(f: &mut Frame, area: Rect, app: &App, container_id: &str, is_primary: bool) {
    let t = &app.theme;
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
                Paragraph::new(format!("{}: waiting for stats…", name))
                    .style(thm::dim_label(t))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // heading
            Constraint::Length(1), // rule
            Constraint::Length(1), // blank
            Constraint::Length(1), // CPU
            Constraint::Length(1), // MEM
            Constraint::Length(1), // NET
            Constraint::Min(0),
        ])
        .split(area);

    let heading_color = if is_primary { t.accent_primary } else { t.accent_header };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                name.to_uppercase(),
                Style::default().fg(heading_color).add_modifier(Modifier::BOLD),
            ),
        ])),
        rows[0],
    );

    let rule_len = (area.width as usize).saturating_sub(2);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled(thm::rule(rule_len), thm::section_rule(t)),
        ])),
        rows[1],
    );

    let label = Style::default().fg(t.accent_header);
    let spark_style = Style::default().fg(t.accent_primary);
    let value_style = Style::default().fg(t.fg_bright);

    let cpu_now = history.cpu.back().copied().unwrap_or(0.0);
    let cpu_spark = sparkline_iter(history.cpu.iter().copied(), SPARK_WIDTH, 0.0, 100.0);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled("CPU ", label),
            Span::styled(cpu_spark, spark_style),
            Span::raw("  "),
            Span::styled(format!("{:5.1}%", cpu_now), value_style),
        ])),
        rows[3],
    );

    let mem_now = history.memory.back().copied().unwrap_or(0.0);
    let limit = history.memory_limit_mb;
    let mem_pct = if limit > 0.0 { mem_now / limit * 100.0 } else { 0.0 };
    let mem_spark = if limit > 0.0 {
        sparkline_iter(
            history.memory.iter().map(|v| v / limit * 100.0),
            SPARK_WIDTH,
            0.0,
            100.0,
        )
    } else {
        sparkline_iter(history.memory.iter().copied(), SPARK_WIDTH, 0.0, 100.0)
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled("MEM ", label),
            Span::styled(mem_spark, spark_style),
            Span::raw("  "),
            Span::styled(format!("{:5.1}%  {}", mem_pct, format_bytes(mem_now)), value_style),
        ])),
        rows[4],
    );

    let rx = history.net_rx.back().copied().unwrap_or(0.0) / 1024.0;
    let tx = history.net_tx.back().copied().unwrap_or(0.0) / 1024.0;
    let combined_iter = || {
        history
            .net_rx
            .iter()
            .zip(history.net_tx.iter())
            .map(|(rxv, txv)| (rxv + txv) / 1024.0)
    };
    let net_max = combined_iter().fold(1.0_f64, f64::max);
    let net_spark = sparkline_iter(combined_iter(), SPARK_WIDTH, 0.0, net_max.max(1.0));
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled("NET ", label),
            Span::styled(net_spark, spark_style),
            Span::raw("  "),
            Span::styled(format!("{:6.1} KB/s", rx + tx), value_style),
        ])),
        rows[5],
    );
}
