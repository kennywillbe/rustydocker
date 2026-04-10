use crate::app::ScreenMode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub sidebar: Rect,
    /// 1-column strip between sidebar and main panel, used to draw a vertical
    /// rule. Zero-width when there is no sidebar (fullscreen mode).
    pub divider: Rect,
    pub main_panel: Rect,
    pub status_bar: Rect,
}

pub fn build_layout(area: Rect, sidebar_width: u16, screen_mode: ScreenMode) -> AppLayout {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let effective_sidebar = match screen_mode {
        ScreenMode::Normal => sidebar_width,
        ScreenMode::Half => sidebar_width / 2,
        ScreenMode::Fullscreen => 0,
    };

    if effective_sidebar == 0 {
        // Fullscreen: main panel takes all space, no divider
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0)])
            .split(outer[0]);
        return AppLayout {
            sidebar: Rect::new(0, 0, 0, 0),
            divider: Rect::new(0, 0, 0, 0),
            main_panel: chunks[0],
            status_bar: outer[1],
        };
    }

    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(effective_sidebar),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(outer[0]);

    AppLayout {
        sidebar: inner[0],
        divider: inner[1],
        main_panel: inner[2],
        status_bar: outer[1],
    }
}
