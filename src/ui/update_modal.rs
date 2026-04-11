//! Update-flow modals: Confirming, Downloading, Installing, Complete, Failed.
//!
//! Mirrors the pattern in `confirm.rs` — a single `render` entry point
//! that dispatches on `app.update_flow`. `Idle` and
//! `InstalledPendingRestart` render nothing (those are banner-only).

use crate::app::{App, UpdateFlow};
use crate::ui::theme::{self, PopupKind, Theme};
use ratatui::prelude::*;
use ratatui::widgets::{Clear, Paragraph, Wrap};

const POPUP_WIDTH: u16 = 48;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    match &app.update_flow {
        UpdateFlow::Confirming => render_confirming(f, area, app),
        UpdateFlow::Downloading(p) => render_progress(f, area, app, ProgressLabel::Downloading(*p)),
        UpdateFlow::Installing => render_progress(f, area, app, ProgressLabel::Installing),
        UpdateFlow::Complete => render_complete(f, area, app),
        UpdateFlow::Failed(msg) => render_failed(f, area, app, msg),
        UpdateFlow::Idle | UpdateFlow::InstalledPendingRestart => {}
    }
}

enum ProgressLabel {
    Downloading(u8),
    Installing,
}

fn render_confirming(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let current = env!("CARGO_PKG_VERSION");
    let target = app.update_available.as_ref().map(|i| i.version.as_str()).unwrap_or("?");

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Update from {} to {}?", current, target),
            Style::default().fg(t.fg_bright).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  This will download the release binary from".to_string(),
            Style::default().fg(t.fg),
        )),
        Line::from(Span::styled(
            "  GitHub and replace the current one in place.".to_string(),
            Style::default().fg(t.fg),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y", Style::default().fg(t.ok).add_modifier(Modifier::BOLD)),
            Span::styled(" confirm    ", Style::default().fg(t.fg)),
            Span::styled("n/Esc", Style::default().fg(t.err).add_modifier(Modifier::BOLD)),
            Span::styled(" cancel", Style::default().fg(t.fg)),
        ]),
        Line::from(""),
    ];

    draw_popup(f, area, t, " UPDATE ", PopupKind::Info, lines);
}

fn render_progress(f: &mut Frame, area: Rect, app: &App, label: ProgressLabel) {
    let t = &app.theme;
    let target = app.update_available.as_ref().map(|i| i.version.as_str()).unwrap_or("?");

    let (heading, percent) = match label {
        ProgressLabel::Downloading(p) => (format!("Downloading {}", target), p),
        ProgressLabel::Installing => (format!("Installing {}…", target), 100),
    };

    let bar_width: usize = 34;
    let filled = (bar_width * percent as usize) / 100;
    let empty = bar_width - filled;
    let bar: String = "▓".repeat(filled) + &"░".repeat(empty);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", heading),
            Style::default().fg(t.fg_bright).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(bar, Style::default().fg(t.accent_primary)),
            Span::styled(format!("  {:>3}%", percent), Style::default().fg(t.fg_bright)),
        ]),
        Line::from(""),
    ];

    draw_popup(f, area, t, " UPDATING ", PopupKind::Info, lines);
}

fn render_complete(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let target = app.update_available.as_ref().map(|i| i.version.as_str()).unwrap_or("?");

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  ✓ rustydocker {} installed", target),
            Style::default().fg(t.ok).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Restart now to use the new version, or".to_string(),
            Style::default().fg(t.fg),
        )),
        Line::from(Span::styled(
            format!("  keep using {}.", env!("CARGO_PKG_VERSION")),
            Style::default().fg(t.fg),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  r", Style::default().fg(t.ok).add_modifier(Modifier::BOLD)),
            Span::styled(" restart now    ", Style::default().fg(t.fg)),
            Span::styled("l/Esc", Style::default().fg(t.fg_muted).add_modifier(Modifier::BOLD)),
            Span::styled(" later", Style::default().fg(t.fg)),
        ]),
        Line::from(""),
    ];

    draw_popup(f, area, t, " UPDATE COMPLETE ", PopupKind::Info, lines);
}

fn render_failed(f: &mut Frame, area: Rect, app: &App, msg: &str) {
    let t = &app.theme;

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ✗ Could not update rustydocker".to_string(),
            Style::default().fg(t.err).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(format!("  {}", msg), Style::default().fg(t.fg))),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Esc/Enter",
                Style::default().fg(t.fg_muted).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" dismiss", Style::default().fg(t.fg)),
        ]),
        Line::from(""),
    ];

    let popup_area = super::centered_rect(area, POPUP_WIDTH, lines.len() as u16 + 2);
    f.render_widget(Clear, popup_area);
    f.render_widget(
        Paragraph::new(lines)
            .block(theme::popup_block(t, " UPDATE FAILED ", PopupKind::Danger))
            .style(Style::default().bg(t.bg_raised))
            .wrap(Wrap { trim: true }),
        popup_area,
    );
}

fn draw_popup(f: &mut Frame, area: Rect, t: &Theme, title: &'static str, kind: PopupKind, lines: Vec<Line<'static>>) {
    let popup_area = super::centered_rect(area, POPUP_WIDTH, lines.len() as u16 + 2);
    f.render_widget(Clear, popup_area);
    f.render_widget(
        Paragraph::new(lines)
            .block(theme::popup_block(t, title, kind))
            .style(Style::default().bg(t.bg_raised)),
        popup_area,
    );
}
