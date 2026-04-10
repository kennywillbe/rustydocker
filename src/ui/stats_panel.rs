//! Compact, sparkline-based statistics view.
//!
//! ```text
//! CPU ▁▂▄█▆▃▂▁▂▃▅▇█▆▃▁▂▄▆▇   42.1%
//! MEM ▁▁▂▃▄▄▄▅▅▅▆▆▆▆▆▇▇▇▇▇   68.3%  !
//! NET ▂▃▂▄▃▅▆▄▃▂▃▄▅▆▇▅▄▃▂▂   128 KB/s ↑
//! ```

use crate::app::App;
use crate::ui::theme::{self as thm, icons, sparkline_iter, Theme};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

pub(super) fn format_bytes(mb: f64) -> String {
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{:.0} MB", mb)
    }
}

/// Width used for the embedded sparkline. Chosen to match the landing-site
/// mock (20 characters).
const SPARK_WIDTH: usize = 20;

pub fn render_stats(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;

    let container_id = match app.selected_container_id() {
        Some(id) => id,
        None => {
            f.render_widget(
                Paragraph::new("No container selected")
                    .style(thm::dim_label(t))
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
                Paragraph::new("Waiting for stats…")
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
            Constraint::Length(1), // blank
            Constraint::Length(1), // CPU
            Constraint::Length(1), // MEM
            Constraint::Length(1), // NET
            Constraint::Length(1), // blank
            Constraint::Length(1), // rule
            Constraint::Length(1), // summary heading
            Constraint::Length(1), // summary row 1
            Constraint::Length(1), // summary row 2
            Constraint::Min(0),    // padding
        ])
        .split(area);

    render_cpu_line(f, rows[1], app, history);
    render_mem_line(f, rows[2], app, history);
    render_net_line(f, rows[3], app, history);

    let rule_len = (area.width as usize).saturating_sub(2);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled(thm::rule(rule_len), thm::section_rule(t)),
        ])),
        rows[5],
    );

    render_summary(f, [rows[6], rows[7], rows[8]], app, container_id);
}

/// Pick the (sparkline, value) styles for a metric based on whether it is
/// currently above its alert threshold.
fn metric_styles(t: &Theme, alerting: bool) -> (Style, Style) {
    if alerting {
        (
            Style::default().fg(t.err),
            Style::default().fg(t.err).add_modifier(Modifier::BOLD),
        )
    } else {
        (Style::default().fg(t.accent_primary), Style::default().fg(t.fg_bright))
    }
}

fn render_cpu_line(f: &mut Frame, area: Rect, app: &App, history: &crate::app::StatsHistory) {
    let t = &app.theme;
    let current = history.cpu.back().copied().unwrap_or(0.0);
    let spark = sparkline_iter(history.cpu.iter().copied(), SPARK_WIDTH, 0.0, 100.0);

    let alerting = current >= app.cpu_alert_threshold;
    let (spark_style, value_style) = metric_styles(t, alerting);

    let mut spans = vec![
        Span::raw(" "),
        Span::styled("CPU ", Style::default().fg(t.accent_header)),
        Span::styled(spark, spark_style),
        Span::raw("   "),
        Span::styled(format!("{:5.1}%", current), value_style),
    ];
    if alerting {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            icons::ALERT,
            Style::default().fg(t.err).add_modifier(Modifier::BOLD),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_mem_line(f: &mut Frame, area: Rect, app: &App, history: &crate::app::StatsHistory) {
    let t = &app.theme;
    let current = history.memory.back().copied().unwrap_or(0.0);
    let limit = history.memory_limit_mb;
    let pct = if limit > 0.0 { current / limit * 100.0 } else { 0.0 };

    let spark = if limit > 0.0 {
        sparkline_iter(
            history.memory.iter().map(|v| v / limit * 100.0),
            SPARK_WIDTH,
            0.0,
            100.0,
        )
    } else {
        sparkline_iter(history.memory.iter().copied(), SPARK_WIDTH, 0.0, 100.0)
    };

    let alerting = pct >= app.memory_alert_threshold;
    let (spark_style, value_style) = metric_styles(t, alerting);

    let suffix = if limit > 0.0 {
        format!("{:5.1}%  ({} / {})", pct, format_bytes(current), format_bytes(limit))
    } else {
        format_bytes(current)
    };

    let mut spans = vec![
        Span::raw(" "),
        Span::styled("MEM ", Style::default().fg(t.accent_header)),
        Span::styled(spark, spark_style),
        Span::raw("   "),
        Span::styled(suffix, value_style),
    ];
    if alerting {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            icons::ALERT,
            Style::default().fg(t.err).add_modifier(Modifier::BOLD),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_net_line(f: &mut Frame, area: Rect, app: &App, history: &crate::app::StatsHistory) {
    let t = &app.theme;
    let dim = thm::dim_label(t);

    let rx_kb = history.net_rx.back().copied().unwrap_or(0.0) / 1024.0;
    let tx_kb = history.net_tx.back().copied().unwrap_or(0.0) / 1024.0;

    let combined_iter = || {
        history
            .net_rx
            .iter()
            .zip(history.net_tx.iter())
            .map(|(rx, tx)| (rx + tx) / 1024.0)
    };
    let max = combined_iter().fold(1.0_f64, f64::max);
    let spark = sparkline_iter(combined_iter(), SPARK_WIDTH, 0.0, max.max(1.0));

    let spans = vec![
        Span::raw(" "),
        Span::styled("NET ", Style::default().fg(t.accent_header)),
        Span::styled(spark, Style::default().fg(t.accent_primary)),
        Span::raw("   "),
        Span::styled(format!("{:7.1} KB/s", rx_kb + tx_kb), Style::default().fg(t.fg_bright)),
        Span::raw("  "),
        Span::styled(icons::ARROW_UP, Style::default().fg(t.ok)),
        Span::styled(format!(" {:5.1}", tx_kb), dim),
        Span::raw("  "),
        Span::styled(icons::ARROW_DOWN, Style::default().fg(t.warn)),
        Span::styled(format!(" {:5.1}", rx_kb), dim),
    ];
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_summary(f: &mut Frame, areas: [Rect; 3], app: &App, container_id: &str) {
    let t = &app.theme;
    let label = Style::default().fg(t.accent_header);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled("SUMMARY", thm::section_header(t, false)),
        ])),
        areas[0],
    );

    let container = app.containers.iter().find(|c| c.id.as_deref() == Some(container_id));
    let uptime = container.and_then(|c| c.status.as_deref()).unwrap_or("—");
    let state = container.and_then(|c| c.state.as_deref()).unwrap_or("—");
    let pid_count = app.container_top.as_ref().map(|rows| rows.len()).unwrap_or(0);

    let row1 = Line::from(vec![
        Span::raw(" "),
        Span::styled("state   ", label),
        Span::styled(state.to_string(), Style::default().fg(t.fg_bright)),
        Span::raw("    "),
        Span::styled("uptime  ", label),
        Span::styled(uptime.to_string(), Style::default().fg(t.fg)),
    ]);
    f.render_widget(Paragraph::new(row1), areas[1]);

    let pid_label = if pid_count > 0 {
        format!("{} processes", pid_count)
    } else {
        "(open Top tab)".to_string()
    };
    let row2 = Line::from(vec![
        Span::raw(" "),
        Span::styled("pids    ", label),
        Span::styled(pid_label, Style::default().fg(t.fg)),
    ]);
    f.render_widget(Paragraph::new(row2), areas[2]);
}
