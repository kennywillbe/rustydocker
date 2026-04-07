use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub sidebar: Rect,
    pub main_panel: Rect,
    pub status_bar: Rect,
}

pub fn build_layout(area: Rect) -> AppLayout {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(0)])
        .split(outer[0]);

    AppLayout {
        sidebar: inner[0],
        main_panel: inner[1],
        status_bar: outer[1],
    }
}
