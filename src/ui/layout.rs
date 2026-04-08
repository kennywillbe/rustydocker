use crate::app::ScreenMode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub sidebar: Rect,
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

    let inner = if effective_sidebar > 0 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(effective_sidebar), Constraint::Min(0)])
            .split(outer[0])
    } else {
        // Fullscreen: main panel takes all space
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0)])
            .split(outer[0]);
        // Return with sidebar as zero-width rect
        return AppLayout {
            sidebar: Rect::new(0, 0, 0, 0),
            main_panel: chunks[0],
            status_bar: outer[1],
        };
    };

    AppLayout {
        sidebar: inner[0],
        main_panel: inner[1],
        status_bar: outer[1],
    }
}
