use crate::docker::compose::ComposeProject;
use bollard::models::{ContainerSummary, ImageSummary, Volume};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Logs,
    Stats,
    Info,
    Graph,
}

impl Tab {
    pub fn next(&self) -> Self {
        match self {
            Tab::Logs => Tab::Stats,
            Tab::Stats => Tab::Info,
            Tab::Info => Tab::Graph,
            Tab::Graph => Tab::Logs,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Tab::Logs => "Logs",
            Tab::Stats => "Stats",
            Tab::Info => "Info",
            Tab::Graph => "Graph",
        }
    }

    pub fn all() -> &'static [Tab] {
        &[Tab::Logs, Tab::Stats, Tab::Info, Tab::Graph]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarSection {
    Services,
    Images,
    Volumes,
}

impl SidebarSection {
    pub fn next(&self) -> Self {
        match self {
            SidebarSection::Services => SidebarSection::Images,
            SidebarSection::Images => SidebarSection::Volumes,
            SidebarSection::Volumes => SidebarSection::Services,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            SidebarSection::Services => SidebarSection::Volumes,
            SidebarSection::Images => SidebarSection::Services,
            SidebarSection::Volumes => SidebarSection::Images,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Sidebar,
    MainPanel,
}

#[derive(Debug, Clone, Default)]
pub struct StatsHistory {
    pub cpu: Vec<f64>,
    pub memory: Vec<f64>,
    pub memory_limit_mb: f64,
    pub net_rx: Vec<f64>,
    pub net_tx: Vec<f64>,
}

impl StatsHistory {
    pub fn push(&mut self, cpu: f64, mem: f64, mem_limit: f64, rx: f64, tx: f64) {
        const MAX_POINTS: usize = 60;
        self.cpu.push(cpu);
        self.memory.push(mem);
        self.memory_limit_mb = mem_limit;
        self.net_rx.push(rx);
        self.net_tx.push(tx);
        if self.cpu.len() > MAX_POINTS {
            self.cpu.remove(0);
            self.memory.remove(0);
            self.net_rx.remove(0);
            self.net_tx.remove(0);
        }
    }
}

pub struct App {
    pub running: bool,
    pub focus: Focus,
    pub active_tab: Tab,
    pub sidebar_section: SidebarSection,
    pub selected_index: usize,
    pub containers: Vec<ContainerSummary>,
    pub images: Vec<ImageSummary>,
    pub volumes: Vec<Volume>,
    pub projects: Vec<ComposeProject>,
    pub logs: HashMap<String, Vec<String>>,
    pub stats: HashMap<String, StatsHistory>,
    pub log_search: Option<String>,
    pub log_scroll_offset: u16,
    pub show_help: bool,
    pub show_cleanup: bool,
    pub status_message: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            focus: Focus::Sidebar,
            active_tab: Tab::Logs,
            sidebar_section: SidebarSection::Services,
            selected_index: 0,
            containers: vec![],
            images: vec![],
            volumes: vec![],
            projects: vec![],
            logs: HashMap::new(),
            stats: HashMap::new(),
            log_search: None,
            log_scroll_offset: 0,
            show_help: false,
            show_cleanup: false,
            status_message: None,
        }
    }

    pub fn selected_container(&self) -> Option<&ContainerSummary> {
        if self.sidebar_section == SidebarSection::Services {
            self.containers.get(self.selected_index)
        } else {
            None
        }
    }

    pub fn selected_container_id(&self) -> Option<&str> {
        self.selected_container().and_then(|c| c.id.as_deref())
    }

    fn current_list_len(&self) -> usize {
        match self.sidebar_section {
            SidebarSection::Services => self.containers.len(),
            SidebarSection::Images => self.images.len(),
            SidebarSection::Volumes => self.volumes.len(),
        }
    }

    pub fn next_item(&mut self) {
        self.log_scroll_offset = 0;
        let len = self.current_list_len();
        if len > 0 {
            if self.selected_index + 1 >= len {
                self.sidebar_section = self.sidebar_section.next();
                self.selected_index = 0;
            } else {
                self.selected_index += 1;
            }
        } else {
            self.sidebar_section = self.sidebar_section.next();
            self.selected_index = 0;
        }
    }

    pub fn prev_item(&mut self) {
        self.log_scroll_offset = 0;
        if self.selected_index == 0 {
            self.sidebar_section = self.sidebar_section.prev();
            let len = self.current_list_len();
            self.selected_index = len.saturating_sub(1);
        } else {
            self.selected_index -= 1;
        }
    }

    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Char('q') => return AppAction::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return AppAction::Quit
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                return AppAction::None;
            }
            _ => {}
        }

        if self.show_help {
            if key.code == KeyCode::Esc {
                self.show_help = false;
            }
            return AppAction::None;
        }

        if self.show_cleanup {
            match key.code {
                KeyCode::Esc => self.show_cleanup = false,
                KeyCode::Char('i') => return AppAction::PruneImages,
                KeyCode::Char('v') => return AppAction::PruneVolumes,
                _ => {}
            }
            return AppAction::None;
        }

        match key.code {
            // Navigation
            KeyCode::Char('j') | KeyCode::Down => self.next_item(),
            KeyCode::Char('k') | KeyCode::Up => self.prev_item(),
            // Focus switching
            KeyCode::Left | KeyCode::Char('h') => self.focus = Focus::Sidebar,
            KeyCode::Right | KeyCode::Char('l') => self.focus = Focus::MainPanel,
            // Tab switching (works regardless of focus)
            KeyCode::Tab => self.next_tab(),
            KeyCode::Char('1') => self.active_tab = Tab::Logs,
            KeyCode::Char('2') => self.active_tab = Tab::Stats,
            KeyCode::Char('3') => self.active_tab = Tab::Info,
            KeyCode::Char('4') => self.active_tab = Tab::Graph,
            // Container actions
            KeyCode::Char('r') => return AppAction::RestartContainer,
            KeyCode::Char('s') => return AppAction::StopContainer,
            KeyCode::Char('u') => return AppAction::StartContainer,
            KeyCode::Char('d') => return AppAction::RemoveContainer,
            KeyCode::Char('e') => return AppAction::ExecShell,
            KeyCode::Char('x') => self.show_cleanup = !self.show_cleanup,
            KeyCode::Char('/') => {
                self.log_search = Some(String::new());
                return AppAction::EnterSearch;
            }
            _ => {}
        }
        AppAction::None
    }

    pub fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent, terminal_size: ratatui::layout::Rect) {
        use crossterm::event::{MouseEventKind, MouseButton};

        let x = mouse.column;
        let y = mouse.row;
        let sidebar_width = 28u16;
        let in_sidebar = x < sidebar_width;

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if in_sidebar && y > 0 && y < terminal_size.height - 1 {
                    self.focus = Focus::Sidebar;
                    let clicked_row = (y - 1) as usize;
                    let mut row = 0usize;

                    // Containers header
                    if clicked_row == row { return; }
                    row += 1;

                    // Container items
                    for i in 0..self.containers.len() {
                        if clicked_row == row {
                            self.sidebar_section = SidebarSection::Services;
                            self.selected_index = i;
                            self.log_scroll_offset = 0;
                            return;
                        }
                        row += 1;
                    }

                    row += 1; // separator

                    // Images header
                    if clicked_row == row { return; }
                    row += 1;

                    for i in 0..self.images.len() {
                        if clicked_row == row {
                            self.sidebar_section = SidebarSection::Images;
                            self.selected_index = i;
                            return;
                        }
                        row += 1;
                    }

                    row += 1; // separator

                    // Volumes header
                    if clicked_row == row { return; }
                    row += 1;

                    for i in 0..self.volumes.len() {
                        if clicked_row == row {
                            self.sidebar_section = SidebarSection::Volumes;
                            self.selected_index = i;
                            return;
                        }
                        row += 1;
                    }
                } else if !in_sidebar {
                    self.focus = Focus::MainPanel;

                    // Click on tab bar: top border row of main panel
                    // Title: " Logs  │  Stats  │  Info  │  Graph "
                    // Positions (0-indexed from main panel left border):
                    //   border(1) + " Logs "(6) + " │ "(3) + " Stats "(7) + " │ "(3) + " Info "(6) + " │ "(3) + " Graph "(7)
                    if y <= 1 && self.sidebar_section == SidebarSection::Services {
                        let tab_x = x.saturating_sub(sidebar_width + 1) as usize;
                        if tab_x < 8 { self.active_tab = Tab::Logs; }
                        else if tab_x < 18 { self.active_tab = Tab::Stats; }
                        else if tab_x < 27 { self.active_tab = Tab::Info; }
                        else { self.active_tab = Tab::Graph; }
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if in_sidebar {
                    self.next_item();
                } else {
                    // Scroll down = towards older logs = decrease offset (closer to bottom)
                    self.log_scroll_offset = self.log_scroll_offset.saturating_sub(3);
                }
            }
            MouseEventKind::ScrollUp => {
                if in_sidebar {
                    self.prev_item();
                } else {
                    // Scroll up = towards newer logs = increase offset (further from bottom)
                    self.log_scroll_offset = self.log_scroll_offset.saturating_add(3);
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum AppAction {
    None,
    Quit,
    RestartContainer,
    StopContainer,
    StartContainer,
    RemoveContainer,
    ExecShell,
    PruneImages,
    PruneVolumes,
    EnterSearch,
}
