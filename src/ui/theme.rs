//! Centralised theme definitions.
//!
//! Two palettes:
//! - [`Theme::EMBER`]   — warm amber on black, matching the project site.
//! - [`Theme::CLASSIC`] — the original cyan/green multicolour scheme.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders};

pub mod icons {
    pub const RUN: &str = "▸";
    pub const EXITED: &str = "▫";
    pub const RESTARTING: &str = "◉";
    pub const PAUSED: &str = "‖";
    pub const PIN: &str = "*";
    pub const SELECTED: &str = "◆";
    pub const VRULE: &str = "│";
    pub const ALERT: &str = "!";
    pub const ARROW_UP: &str = "↑";
    pub const ARROW_DOWN: &str = "↓";
}

/// 256 box-drawing rule characters. Slice by length to avoid allocating a
/// fresh `String` on every frame in header rendering code.
const RULE_FULL: &str = "────────────────────────────────\
                        ────────────────────────────────\
                        ────────────────────────────────\
                        ────────────────────────────────\
                        ────────────────────────────────\
                        ────────────────────────────────\
                        ────────────────────────────────\
                        ────────────────────────────────";

/// Return a static slice of `n` rule characters (clamped to 256).
pub fn rule(n: usize) -> &'static str {
    // Each `─` is 3 UTF-8 bytes; `n` counts characters.
    let max_chars = RULE_FULL.chars().count();
    let n = n.min(max_chars);
    // Walk to the byte offset of the nth char.
    let end = RULE_FULL
        .char_indices()
        .nth(n)
        .map(|(i, _)| i)
        .unwrap_or(RULE_FULL.len());
    &RULE_FULL[..end]
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Raised surface: selection backgrounds, popup chrome, status strips.
    pub bg_raised: Color,

    pub fg: Color,
    pub fg_bright: Color,
    pub fg_dim: Color,
    pub fg_muted: Color,

    pub accent_header: Color,
    pub accent_primary: Color,
    pub accent_strong: Color,

    /// Foreground used when drawing on top of `accent_primary` (e.g. log
    /// search-match highlights). Needs to contrast with the accent across
    /// both themes.
    pub on_accent: Color,

    pub ok: Color,
    pub err: Color,
    pub warn: Color,
    pub info: Color,

    pub rule: Color,
}

impl Theme {
    pub const EMBER: Self = Self {
        bg_raised: Color::Rgb(26, 19, 12),

        fg: Color::Rgb(215, 203, 172),
        fg_bright: Color::Rgb(240, 229, 198),
        fg_dim: Color::Rgb(122, 111, 87),
        fg_muted: Color::Rgb(106, 94, 74),

        accent_header: Color::Rgb(228, 179, 99),
        accent_primary: Color::Rgb(198, 80, 26),
        accent_strong: Color::Rgb(178, 48, 32),

        on_accent: Color::Rgb(16, 12, 8),

        ok: Color::Rgb(138, 168, 74),
        err: Color::Rgb(194, 74, 46),
        warn: Color::Rgb(198, 161, 74),
        info: Color::Rgb(138, 168, 74),

        rule: Color::Rgb(58, 46, 34),
    };

    pub const CLASSIC: Self = Self {
        bg_raised: Color::Rgb(50, 50, 70),

        fg: Color::White,
        fg_bright: Color::White,
        fg_dim: Color::DarkGray,
        fg_muted: Color::DarkGray,

        accent_header: Color::Yellow,
        accent_primary: Color::Cyan,
        accent_strong: Color::Cyan,

        on_accent: Color::Black,

        ok: Color::Rgb(80, 220, 120),
        err: Color::Rgb(255, 80, 80),
        warn: Color::Rgb(255, 200, 50),
        info: Color::Rgb(80, 220, 120),

        rule: Color::DarkGray,
    };

    pub fn from_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "classic" => Self::CLASSIC,
            _ => Self::EMBER,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::EMBER
    }
}

// ── Style helpers ───────────────────────────────────────────

pub fn section_header(t: &Theme, focused: bool) -> Style {
    let fg = if focused { t.fg_bright } else { t.accent_header };
    Style::default().fg(fg).add_modifier(Modifier::BOLD)
}

pub fn section_rule(t: &Theme) -> Style {
    Style::default().fg(t.rule)
}

pub fn tab_active(t: &Theme) -> Style {
    Style::default()
        .fg(t.fg_bright)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

pub fn tab_inactive(t: &Theme) -> Style {
    Style::default().fg(t.fg_dim)
}

pub fn tab_separator(t: &Theme) -> Style {
    Style::default().fg(t.fg_muted)
}

pub fn dim_label(t: &Theme) -> Style {
    Style::default().fg(t.fg_dim)
}

/// Bold amber (or yellow in classic) — the most-repeated style in the UI.
pub fn header_label(t: &Theme) -> Style {
    Style::default().fg(t.accent_header).add_modifier(Modifier::BOLD)
}

pub fn selected_row(t: &Theme) -> Style {
    Style::default().bg(t.bg_raised).fg(t.fg_bright)
}

/// Foreground style for a list item's name, picked from selected/alert flags.
pub fn name_style(t: &Theme, is_selected: bool, is_alert: bool) -> Style {
    if is_alert {
        Style::default().fg(t.err).add_modifier(Modifier::BOLD)
    } else if is_selected {
        Style::default().fg(t.fg_bright).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.fg)
    }
}

pub fn label_cell(t: &Theme) -> Style {
    Style::default().fg(t.accent_header)
}

pub fn value_cell(t: &Theme) -> Style {
    Style::default().fg(t.fg_bright).add_modifier(Modifier::BOLD)
}

/// Resolve a container state to (glyph, styled colour).
pub fn state_style(t: &Theme, state: &str) -> (&'static str, Style) {
    match state {
        "running" => (icons::RUN, Style::default().fg(t.ok)),
        "exited" => (icons::EXITED, Style::default().fg(t.err)),
        "restarting" => (icons::RESTARTING, Style::default().fg(t.warn)),
        "paused" => (icons::PAUSED, Style::default().fg(t.warn)),
        _ => (icons::EXITED, Style::default().fg(t.fg_muted)),
    }
}

/// Just the colour for a container state (used where the glyph is irrelevant).
pub fn state_color(t: &Theme, state: &str) -> Color {
    match state {
        "running" => t.ok,
        "exited" => t.err,
        "restarting" | "paused" => t.warn,
        _ => t.fg_muted,
    }
}

/// Intent for popup chrome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupKind {
    Info,
    Danger,
}

/// Build a popup `Block` with a titled border. The caller is expected to
/// render `Clear` + a `Paragraph::new(...).block(block).style(bg(t.bg_raised))`
/// inside the chosen `Rect`.
pub fn popup_block(t: &Theme, title: &'static str, kind: PopupKind) -> Block<'static> {
    let border_color = match kind {
        PopupKind::Info => t.rule,
        PopupKind::Danger => t.accent_strong,
    };
    let title_color = match kind {
        PopupKind::Info => t.accent_header,
        PopupKind::Danger => t.accent_strong,
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title,
            Style::default().fg(title_color).add_modifier(Modifier::BOLD),
        ))
}

/// Sparkline from an iterator of f64 values, normalised to `[min, max]`.
/// `width` is the glyph count; short input is left-padded with the minimum
/// bar, long input is averaged into `width` buckets.
pub fn sparkline_iter<I>(values: I, width: usize, min: f64, max: f64) -> String
where
    I: IntoIterator<Item = f64>,
    I::IntoIter: ExactSizeIterator,
{
    const BARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if width == 0 {
        return String::new();
    }

    let iter = values.into_iter();
    let len = iter.len();
    if len == 0 {
        return " ".repeat(width);
    }

    let range = (max - min).max(1e-9);
    let mut out = String::with_capacity(width * 3);
    let mut push = |v: f64| {
        let norm = ((v - min) / range).clamp(0.0, 1.0);
        let idx = ((norm * (BARS.len() - 1) as f64).round()) as usize;
        out.push(BARS[idx.min(BARS.len() - 1)]);
    };

    if len <= width {
        for _ in 0..(width - len) {
            push(min);
        }
        for v in iter {
            push(v);
        }
    } else {
        // Collect into a Vec once so we can slice into buckets. Small by
        // construction (width ≤ 30 in practice).
        let buf: Vec<f64> = iter.collect();
        let step = buf.len() as f64 / width as f64;
        for i in 0..width {
            let start = (i as f64 * step) as usize;
            let end = ((i + 1) as f64 * step) as usize;
            let end = end.clamp(start + 1, buf.len());
            let slice = &buf[start..end];
            let avg = slice.iter().copied().sum::<f64>() / slice.len() as f64;
            push(avg);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spark(values: &[f64], width: usize, min: f64, max: f64) -> String {
        sparkline_iter(values.iter().copied(), width, min, max)
    }

    #[test]
    fn sparkline_pads_short_input_with_min() {
        let s = spark(&[10.0, 50.0, 90.0], 6, 0.0, 100.0);
        assert_eq!(s.chars().count(), 6);
        assert_eq!(s.chars().next(), Some('▁'));
    }

    #[test]
    fn sparkline_downsamples_long_input() {
        let values: Vec<f64> = (0..120).map(|i| i as f64).collect();
        let s = spark(&values, 20, 0.0, 119.0);
        assert_eq!(s.chars().count(), 20);
        let first = s.chars().next().unwrap();
        let last = s.chars().last().unwrap();
        assert!(first < last);
    }

    #[test]
    fn sparkline_width_zero() {
        assert_eq!(spark(&[1.0], 0, 0.0, 1.0), "");
    }

    #[test]
    fn theme_from_name_defaults_to_ember() {
        let t = Theme::from_name("nonsense");
        assert_eq!(t.accent_primary, Theme::EMBER.accent_primary);
    }

    #[test]
    fn theme_from_name_classic() {
        let t = Theme::from_name("classic");
        assert_eq!(t.accent_primary, Theme::CLASSIC.accent_primary);
    }

    #[test]
    fn rule_slice_counts_chars_not_bytes() {
        assert_eq!(rule(5).chars().count(), 5);
        assert_eq!(rule(0), "");
        // Clamped to the static length.
        assert!(rule(10_000).chars().count() <= 256);
    }

    #[test]
    fn name_style_alert_wins() {
        let t = Theme::EMBER;
        let s = name_style(&t, true, true);
        assert_eq!(s.fg, Some(t.err));
    }
}
