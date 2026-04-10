use crate::config::AppConfig;
use crate::docker::compose::ComposeProject;
use bollard::models::{ContainerSummary, ImageSummary, Network, Volume};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Logs,
    Stats,
    Info,
    Env,
    Top,
    Graph,
}

impl Tab {
    pub fn next(&self) -> Self {
        match self {
            Tab::Logs => Tab::Stats,
            Tab::Stats => Tab::Info,
            Tab::Info => Tab::Env,
            Tab::Env => Tab::Top,
            Tab::Top => Tab::Graph,
            Tab::Graph => Tab::Logs,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Tab::Logs => "Logs",
            Tab::Stats => "Stats",
            Tab::Info => "Info",
            Tab::Env => "Env",
            Tab::Top => "Top",
            Tab::Graph => "Graph",
        }
    }

    pub fn all() -> &'static [Tab] {
        &[Tab::Logs, Tab::Stats, Tab::Info, Tab::Env, Tab::Top, Tab::Graph]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarSection {
    Services,
    Images,
    Volumes,
    Networks,
}

impl SidebarSection {
    pub fn next(&self) -> Self {
        match self {
            SidebarSection::Services => SidebarSection::Images,
            SidebarSection::Images => SidebarSection::Volumes,
            SidebarSection::Volumes => SidebarSection::Networks,
            SidebarSection::Networks => SidebarSection::Services,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            SidebarSection::Services => SidebarSection::Networks,
            SidebarSection::Images => SidebarSection::Services,
            SidebarSection::Volumes => SidebarSection::Images,
            SidebarSection::Networks => SidebarSection::Volumes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScreenMode {
    Normal,     // sidebar + main panel
    Half,       // narrow sidebar + main panel
    Fullscreen, // main panel only (no sidebar)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Sidebar,
    MainPanel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
    Filter,
}

#[derive(Debug, Clone, Default)]
pub struct StatsHistory {
    pub cpu: VecDeque<f64>,
    pub memory: VecDeque<f64>,
    pub memory_limit_mb: f64,
    pub net_rx: VecDeque<f64>,
    pub net_tx: VecDeque<f64>,
    prev_net_rx: f64,
    prev_net_tx: f64,
}

impl StatsHistory {
    pub fn push(&mut self, cpu: f64, mem: f64, mem_limit: f64, rx_cumulative: f64, tx_cumulative: f64) {
        const MAX_POINTS: usize = 60;
        self.cpu.push_back(cpu);
        self.memory.push_back(mem);
        self.memory_limit_mb = mem_limit;
        // Calculate delta (bytes/s) from cumulative values
        let rx_delta = if self.prev_net_rx > 0.0 {
            (rx_cumulative - self.prev_net_rx).max(0.0)
        } else {
            0.0
        };
        let tx_delta = if self.prev_net_tx > 0.0 {
            (tx_cumulative - self.prev_net_tx).max(0.0)
        } else {
            0.0
        };
        self.prev_net_rx = rx_cumulative;
        self.prev_net_tx = tx_cumulative;
        self.net_rx.push_back(rx_delta);
        self.net_tx.push_back(tx_delta);
        while self.cpu.len() > MAX_POINTS {
            self.cpu.pop_front();
            self.memory.pop_front();
            self.net_rx.pop_front();
            self.net_tx.pop_front();
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
    pub networks: Vec<Network>,
    pub projects: Vec<ComposeProject>,
    pub logs: HashMap<String, Vec<String>>,
    pub stats: HashMap<String, StatsHistory>,
    pub log_search: Option<String>,
    pub sidebar_filter: Option<String>,
    pub log_scroll_offset: u16,
    pub show_help: bool,
    pub show_cleanup: bool,
    pub show_bulk: bool,
    pub pending_confirm: Option<PendingConfirm>,
    pub input_mode: InputMode,
    pub status_message: Option<String>,
    pub status_message_at: Option<std::time::Instant>,
    pub container_env: Option<Vec<(String, String)>>,
    pub container_inspect: Option<bollard::models::ContainerInspectResponse>,
    pub container_top: Option<Vec<Vec<String>>>,
    pub sidebar_width: u16,
    pub log_tail_lines: String,
    pub screen_mode: ScreenMode,
    pub selected_containers: HashSet<String>,
    pub pinned_containers: HashSet<String>,
    pub log_bookmarks: Vec<usize>,
    pub show_all_logs: bool,
    pub log_snapshot: Option<Vec<String>>,
    pub show_log_diff: bool,
    pub show_custom_commands: bool,
    pub custom_commands: Vec<crate::config::CustomCommand>,
    pub cpu_alert_threshold: f64,
    pub memory_alert_threshold: f64,
    pub compare_container_id: Option<String>,
    pub hooks: Vec<crate::config::Hook>,
    pub docker_host: Option<String>,
    pub theme: crate::ui::theme::Theme,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        Self {
            running: true,
            focus: Focus::Sidebar,
            active_tab: Tab::Logs,
            sidebar_section: SidebarSection::Services,
            selected_index: 0,
            containers: vec![],
            images: vec![],
            volumes: vec![],
            networks: vec![],
            projects: vec![],
            logs: HashMap::new(),
            stats: HashMap::new(),
            log_search: None,
            sidebar_filter: None,
            log_scroll_offset: 0,
            show_help: false,
            show_cleanup: false,
            show_bulk: false,
            pending_confirm: None,
            input_mode: InputMode::Normal,
            status_message: None,
            status_message_at: None,
            container_env: None,
            container_inspect: None,
            container_top: None,
            sidebar_width: config.sidebar_width,
            log_tail_lines: config.log_tail_lines,
            screen_mode: ScreenMode::Normal,
            selected_containers: HashSet::new(),
            pinned_containers: HashSet::new(),
            log_bookmarks: vec![],
            show_all_logs: false,
            log_snapshot: None,
            show_log_diff: false,
            show_custom_commands: false,
            custom_commands: config.custom_commands.clone(),
            cpu_alert_threshold: config.cpu_alert_threshold,
            memory_alert_threshold: config.memory_alert_threshold,
            compare_container_id: None,
            hooks: config.hooks.clone(),
            docker_host: None,
            theme: crate::ui::theme::Theme::from_name(&config.theme),
        }
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some(msg.to_string());
        self.status_message_at = Some(std::time::Instant::now());
    }

    pub fn clear_expired_status(&mut self) {
        if let Some(at) = self.status_message_at {
            if at.elapsed().as_secs() >= 4 {
                self.status_message = None;
                self.status_message_at = None;
            }
        }
    }

    pub fn container_has_alert(&self, container_id: &str) -> bool {
        if let Some(history) = self.stats.get(container_id) {
            if let Some(&cpu) = history.cpu.back() {
                if cpu > self.cpu_alert_threshold {
                    return true;
                }
            }
            if let Some(&mem) = history.memory.back() {
                if history.memory_limit_mb > 0.0 {
                    let mem_pct = mem / history.memory_limit_mb * 100.0;
                    if mem_pct > self.memory_alert_threshold {
                        return true;
                    }
                }
            }
        }
        false
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

    pub fn target_container_ids(&self) -> Vec<String> {
        if !self.selected_containers.is_empty() {
            self.selected_containers.iter().cloned().collect()
        } else if let Some(id) = self.selected_container_id().map(|s| s.to_string()) {
            vec![id]
        } else {
            vec![]
        }
    }

    pub fn sort_containers(&mut self) {
        let pinned = &self.pinned_containers;
        self.containers.sort_by(|a, b| {
            let a_pinned = a.id.as_ref().map(|id| pinned.contains(id)).unwrap_or(false);
            let b_pinned = b.id.as_ref().map(|id| pinned.contains(id)).unwrap_or(false);
            // Pinned first
            if a_pinned != b_pinned {
                return if a_pinned {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }
            // Then by state
            let state_order = |s: &Option<String>| -> u8 {
                match s.as_deref() {
                    Some("running") => 0,
                    Some("restarting") => 1,
                    Some("paused") => 2,
                    Some("created") => 3,
                    Some("exited") => 4,
                    _ => 5,
                }
            };
            let ord = state_order(&a.state).cmp(&state_order(&b.state));
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
            let name_a = a
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| n.trim_start_matches('/'))
                .unwrap_or("");
            let name_b = b
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| n.trim_start_matches('/'))
                .unwrap_or("");
            name_a.to_lowercase().cmp(&name_b.to_lowercase())
        });
    }

    pub fn prune_stale_selections(&mut self) {
        let live_ids: HashSet<String> = self.containers.iter().filter_map(|c| c.id.clone()).collect();
        self.selected_containers.retain(|id| live_ids.contains(id));
    }

    pub fn clamp_selected_index(&mut self) {
        let len = self.current_list_len();
        if len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= len {
            self.selected_index = len - 1;
        }
    }

    fn current_list_len(&self) -> usize {
        match self.sidebar_section {
            SidebarSection::Services => self.containers.len(),
            SidebarSection::Images => self.images.len(),
            SidebarSection::Volumes => self.volumes.len(),
            SidebarSection::Networks => self.networks.len(),
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
        // Search mode intercepts all input except Ctrl+C
        if self.input_mode == InputMode::Search {
            if let KeyCode::Char('c') = key.code {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return AppAction::Quit;
                }
            }
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.log_search = None;
                }
                KeyCode::Enter => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    if let Some(ref mut query) = self.log_search {
                        query.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(ref mut query) = self.log_search {
                        query.push(c);
                    }
                }
                _ => {}
            }
            return AppAction::None;
        }

        // Filter mode intercepts all input except Ctrl+C
        if self.input_mode == InputMode::Filter {
            if let KeyCode::Char('c') = key.code {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return AppAction::Quit;
                }
            }
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.sidebar_filter = None;
                }
                KeyCode::Enter => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    if let Some(ref mut query) = self.sidebar_filter {
                        query.pop();
                        if query.is_empty() {
                            self.sidebar_filter = None;
                            self.input_mode = InputMode::Normal;
                        }
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(ref mut query) = self.sidebar_filter {
                        query.push(c);
                    }
                }
                _ => {}
            }
            return AppAction::None;
        }

        match key.code {
            KeyCode::Char('q') => return AppAction::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return AppAction::Quit,
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
                KeyCode::Char('i') => {
                    self.show_cleanup = false;
                    self.pending_confirm = Some(PendingConfirm {
                        message: "Prune dangling images? (y/n)".to_string(),
                        action: AppAction::PruneImages,
                    });
                }
                KeyCode::Char('v') => {
                    self.show_cleanup = false;
                    self.pending_confirm = Some(PendingConfirm {
                        message: "Prune unused volumes? (y/n)".to_string(),
                        action: AppAction::PruneVolumes,
                    });
                }
                _ => {}
            }
            return AppAction::None;
        }

        if self.show_bulk {
            match key.code {
                KeyCode::Esc => self.show_bulk = false,
                KeyCode::Char('s') => {
                    self.show_bulk = false;
                    self.pending_confirm = Some(PendingConfirm {
                        message: "Stop all containers? (y/n)".to_string(),
                        action: AppAction::StopAllContainers,
                    });
                }
                KeyCode::Char('r') => {
                    self.show_bulk = false;
                    self.pending_confirm = Some(PendingConfirm {
                        message: "Remove stopped containers? (y/n)".to_string(),
                        action: AppAction::RemoveStoppedContainers,
                    });
                }
                KeyCode::Char('c') => {
                    self.show_bulk = false;
                    self.pending_confirm = Some(PendingConfirm {
                        message: "Prune containers? (y/n)".to_string(),
                        action: AppAction::PruneContainers,
                    });
                }
                KeyCode::Char('i') => {
                    self.show_bulk = false;
                    self.pending_confirm = Some(PendingConfirm {
                        message: "Prune dangling images? (y/n)".to_string(),
                        action: AppAction::PruneImages,
                    });
                }
                KeyCode::Char('v') => {
                    self.show_bulk = false;
                    self.pending_confirm = Some(PendingConfirm {
                        message: "Prune unused volumes? (y/n)".to_string(),
                        action: AppAction::PruneVolumes,
                    });
                }
                KeyCode::Char('n') => {
                    self.show_bulk = false;
                    self.pending_confirm = Some(PendingConfirm {
                        message: "Prune networks? (y/n)".to_string(),
                        action: AppAction::PruneNetworks,
                    });
                }
                _ => {}
            }
            return AppAction::None;
        }

        if self.show_custom_commands {
            match key.code {
                KeyCode::Esc => self.show_custom_commands = false,
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let idx = c.to_digit(10).unwrap() as usize;
                    if idx > 0 && idx <= self.custom_commands.len() {
                        self.show_custom_commands = false;
                        return AppAction::RunCustomCommand(idx - 1);
                    }
                }
                _ => {}
            }
            return AppAction::None;
        }

        if let Some(ref pending) = self.pending_confirm {
            let action = pending.action.clone();
            match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.pending_confirm = None;
                    return action;
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.pending_confirm = None;
                    return AppAction::None;
                }
                _ => return AppAction::None, // ignore other keys, keep dialog open
            }
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
            KeyCode::Char('4') => self.active_tab = Tab::Env,
            KeyCode::Char('5') => self.active_tab = Tab::Top,
            KeyCode::Char('6') => self.active_tab = Tab::Graph,
            // Container actions
            KeyCode::Char('r') => return AppAction::RestartContainer,
            KeyCode::Char('s') => return AppAction::StopContainer,
            KeyCode::Char('u') => return AppAction::StartContainer,
            KeyCode::Char('d') => {
                self.pending_confirm = Some(PendingConfirm {
                    message: "Remove this container? (y/n)".to_string(),
                    action: AppAction::RemoveContainer,
                });
                return AppAction::None;
            }
            KeyCode::Char('p') => {
                if let Some(container) = self.selected_container() {
                    match container.state.as_deref() {
                        Some("paused") => return AppAction::UnpauseContainer,
                        Some("running") => return AppAction::PauseContainer,
                        _ => {}
                    }
                }
            }
            KeyCode::Char('w') => return AppAction::OpenInBrowser,
            KeyCode::Char('e') => return AppAction::ExecShell,
            KeyCode::Char('a') => return AppAction::AttachContainer,
            KeyCode::Char('U') => return AppAction::ComposeUp,
            KeyCode::Char('D') => {
                self.pending_confirm = Some(PendingConfirm {
                    message: "Docker compose down? (y/n)".to_string(),
                    action: AppAction::ComposeDown,
                });
                return AppAction::None;
            }
            KeyCode::Char('R') => return AppAction::ComposeRestart,
            KeyCode::Char('L') => {
                self.show_all_logs = !self.show_all_logs;
                if self.show_all_logs {
                    self.active_tab = Tab::Logs;
                }
            }
            KeyCode::Char('S') => return AppAction::ExportLogs,
            KeyCode::Char('+') => {
                self.screen_mode = match self.screen_mode {
                    ScreenMode::Normal => ScreenMode::Half,
                    ScreenMode::Half => ScreenMode::Fullscreen,
                    ScreenMode::Fullscreen => ScreenMode::Normal,
                };
            }
            KeyCode::Char('_') => {
                self.screen_mode = match self.screen_mode {
                    ScreenMode::Normal => ScreenMode::Fullscreen,
                    ScreenMode::Fullscreen => ScreenMode::Half,
                    ScreenMode::Half => ScreenMode::Normal,
                };
            }
            KeyCode::Char('x') => self.show_cleanup = !self.show_cleanup,
            KeyCode::Char(' ') => {
                if self.sidebar_section == SidebarSection::Services {
                    if let Some(id) = self.selected_container_id().map(|s| s.to_string()) {
                        if self.selected_containers.contains(&id) {
                            self.selected_containers.remove(&id);
                        } else {
                            self.selected_containers.insert(id);
                        }
                    }
                    self.next_item();
                }
            }
            KeyCode::Char('*') => {
                if self.sidebar_section == SidebarSection::Services {
                    if let Some(id) = self.selected_container_id().map(|s| s.to_string()) {
                        if self.pinned_containers.contains(&id) {
                            self.pinned_containers.remove(&id);
                        } else {
                            self.pinned_containers.insert(id);
                        }
                    }
                }
            }
            KeyCode::Char('c') => {
                if !self.custom_commands.is_empty() {
                    self.show_custom_commands = !self.show_custom_commands;
                }
                return AppAction::None;
            }
            KeyCode::Char('b') => {
                self.show_bulk = !self.show_bulk;
                return AppAction::None;
            }
            KeyCode::Char('/') => {
                if self.focus == Focus::Sidebar {
                    self.sidebar_filter = Some(String::new());
                    self.input_mode = InputMode::Filter;
                } else {
                    self.log_search = Some(String::new());
                    self.input_mode = InputMode::Search;
                }
                return AppAction::None;
            }
            KeyCode::Char('m') => {
                if self.active_tab == Tab::Logs {
                    if let Some(id) = self.selected_container_id() {
                        let log_len = self.logs.get(id).map(|l| l.len()).unwrap_or(0);
                        if log_len > 0 {
                            let line_idx = if self.log_scroll_offset == 0 {
                                log_len.saturating_sub(1)
                            } else {
                                log_len.saturating_sub(self.log_scroll_offset as usize + 1)
                            };
                            if let Some(pos) = self.log_bookmarks.iter().position(|&b| b == line_idx) {
                                self.log_bookmarks.remove(pos);
                            } else {
                                self.log_bookmarks.push(line_idx);
                                self.log_bookmarks.sort();
                            }
                        }
                    }
                }
            }
            KeyCode::Char('n') => {
                if self.active_tab == Tab::Logs && !self.log_bookmarks.is_empty() {
                    if let Some(id) = self.selected_container_id() {
                        let log_len = self.logs.get(id).map(|l| l.len()).unwrap_or(0);
                        let current_line = if self.log_scroll_offset == 0 {
                            log_len.saturating_sub(1)
                        } else {
                            log_len.saturating_sub(self.log_scroll_offset as usize + 1)
                        };
                        if let Some(&next) = self.log_bookmarks.iter().find(|&&b| b > current_line) {
                            self.log_scroll_offset = log_len.saturating_sub(next + 1) as u16;
                        } else if let Some(&first) = self.log_bookmarks.first() {
                            self.log_scroll_offset = log_len.saturating_sub(first + 1) as u16;
                        }
                    }
                }
            }
            KeyCode::Char('T') => {
                if self.active_tab == Tab::Logs {
                    if self.show_log_diff {
                        // Exit diff mode
                        self.show_log_diff = false;
                        self.log_snapshot = None;
                    } else if self.log_snapshot.is_some() {
                        // Show diff
                        self.show_log_diff = true;
                    } else {
                        // Take snapshot
                        if let Some(id) = self.selected_container_id() {
                            self.log_snapshot = self.logs.get(id).cloned();
                        }
                        return AppAction::None;
                    }
                }
            }
            KeyCode::Char('C') => {
                if self.sidebar_section == SidebarSection::Services {
                    if self.compare_container_id.is_some() {
                        self.compare_container_id = None;
                    } else {
                        self.compare_container_id = self.selected_container_id().map(|s| s.to_string());
                    }
                }
            }
            KeyCode::Char('N') => {
                if self.active_tab == Tab::Logs && !self.log_bookmarks.is_empty() {
                    if let Some(id) = self.selected_container_id() {
                        let log_len = self.logs.get(id).map(|l| l.len()).unwrap_or(0);
                        let current_line = if self.log_scroll_offset == 0 {
                            log_len.saturating_sub(1)
                        } else {
                            log_len.saturating_sub(self.log_scroll_offset as usize + 1)
                        };
                        if let Some(&prev) = self.log_bookmarks.iter().rev().find(|&&b| b < current_line) {
                            self.log_scroll_offset = log_len.saturating_sub(prev + 1) as u16;
                        } else if let Some(&last) = self.log_bookmarks.last() {
                            self.log_scroll_offset = log_len.saturating_sub(last + 1) as u16;
                        }
                    }
                }
            }
            _ => {}
        }
        AppAction::None
    }

    pub fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent, terminal_size: ratatui::layout::Rect) {
        use crossterm::event::{MouseButton, MouseEventKind};

        let x = mouse.column;
        let y = mouse.row;
        let sidebar_width = match self.screen_mode {
            ScreenMode::Normal => self.sidebar_width,
            ScreenMode::Half => self.sidebar_width / 2,
            ScreenMode::Fullscreen => 0,
        };
        let in_sidebar = sidebar_width > 0 && x < sidebar_width;

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if in_sidebar && y > 0 && y < terminal_size.height - 1 {
                    self.focus = Focus::Sidebar;
                    let clicked_row = (y - 1) as usize;
                    let mut row = 0usize;

                    // Containers header
                    if clicked_row == row {
                        return;
                    }
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
                    if clicked_row == row {
                        return;
                    }
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
                    if clicked_row == row {
                        return;
                    }
                    row += 1;

                    for i in 0..self.volumes.len() {
                        if clicked_row == row {
                            self.sidebar_section = SidebarSection::Volumes;
                            self.selected_index = i;
                            return;
                        }
                        row += 1;
                    }

                    row += 1; // separator

                    // Networks header
                    if clicked_row == row {
                        return;
                    }
                    row += 1;

                    for i in 0..self.networks.len() {
                        if clicked_row == row {
                            self.sidebar_section = SidebarSection::Networks;
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
                        // Tab layout: " Logs │ Stats │ Info │ Env │ Top │ Graph "
                        // Each tab: " X " (label+2) + " │ " (3) separator
                        let tab_x = x.saturating_sub(sidebar_width + 1) as usize;
                        let tabs = Tab::all();
                        let mut pos = 0usize;
                        let mut clicked_tab = tabs[tabs.len() - 1]; // default to last
                        for (i, t) in tabs.iter().enumerate() {
                            let tab_width = t.label().len() + 2; // " Label "
                            let sep_width = if i < tabs.len() - 1 { 3 } else { 0 }; // " │ "
                            if tab_x < pos + tab_width {
                                clicked_tab = *t;
                                break;
                            }
                            pos += tab_width + sep_width;
                        }
                        self.active_tab = clicked_tab;
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
        self.clamp_selected_index();
    }

    pub fn filtered_containers(&self) -> Vec<(usize, &ContainerSummary)> {
        let filter = self.sidebar_filter.as_deref().unwrap_or("");
        if filter.is_empty() {
            return self.containers.iter().enumerate().collect();
        }
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(i64, usize, &ContainerSummary)> = self
            .containers
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let name = c
                    .names
                    .as_ref()
                    .and_then(|n| n.first())
                    .map(|n| n.trim_start_matches('/'))
                    .unwrap_or("");
                matcher.fuzzy_match(name, filter).map(|score| (score, i, c))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, i, c)| (i, c)).collect()
    }

    pub fn filtered_images(&self) -> Vec<(usize, &ImageSummary)> {
        let filter = self.sidebar_filter.as_deref().unwrap_or("");
        if filter.is_empty() {
            return self.images.iter().enumerate().collect();
        }
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(i64, usize, &ImageSummary)> = self
            .images
            .iter()
            .enumerate()
            .filter_map(|(i, img)| {
                let tag = img.repo_tags.first().map(|t| t.as_str()).unwrap_or("");
                matcher.fuzzy_match(tag, filter).map(|score| (score, i, img))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, i, img)| (i, img)).collect()
    }

    pub fn filtered_volumes(&self) -> Vec<(usize, &Volume)> {
        let filter = self.sidebar_filter.as_deref().unwrap_or("");
        if filter.is_empty() {
            return self.volumes.iter().enumerate().collect();
        }
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(i64, usize, &Volume)> = self
            .volumes
            .iter()
            .enumerate()
            .filter_map(|(i, v)| matcher.fuzzy_match(&v.name, filter).map(|score| (score, i, v)))
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, i, v)| (i, v)).collect()
    }

    pub fn filtered_networks(&self) -> Vec<(usize, &Network)> {
        let filter = self.sidebar_filter.as_deref().unwrap_or("");
        if filter.is_empty() {
            return self.networks.iter().enumerate().collect();
        }
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(i64, usize, &Network)> = self
            .networks
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                let name = n.name.as_deref().unwrap_or("");
                matcher.fuzzy_match(name, filter).map(|score| (score, i, n))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, i, n)| (i, n)).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingConfirm {
    pub message: String,
    pub action: AppAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    None,
    Quit,
    RestartContainer,
    StopContainer,
    StartContainer,
    RemoveContainer,
    ExecShell,
    AttachContainer,
    PauseContainer,
    UnpauseContainer,
    PruneImages,
    PruneVolumes,
    ComposeUp,
    ComposeDown,
    ComposeRestart,
    OpenInBrowser,
    StopAllContainers,
    RemoveStoppedContainers,
    PruneContainers,
    PruneNetworks,
    ExportLogs,
    RunCustomCommand(usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn create_container_with_state(id: &str, state: &str) -> ContainerSummary {
        ContainerSummary {
            id: Some(id.to_string()),
            names: Some(vec![format!("/{}", id)]),
            state: Some(state.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_pin_container_toggle() {
        let mut app = App::new(AppConfig::default());
        app.containers = vec![create_container_with_state("c1", "running")];
        app.handle_key(key(KeyCode::Char('*')));
        assert!(app.pinned_containers.contains("c1"));
        app.handle_key(key(KeyCode::Char('*')));
        assert!(!app.pinned_containers.contains("c1"));
    }

    #[test]
    fn test_filter_mode_sidebar() {
        let mut app = App::new(AppConfig::default());
        app.focus = Focus::Sidebar;
        app.handle_key(key(KeyCode::Char('/')));
        assert_eq!(app.input_mode, InputMode::Filter);
        assert_eq!(app.sidebar_filter, Some(String::new()));
    }

    #[test]
    fn test_search_mode_main_panel() {
        let mut app = App::new(AppConfig::default());
        app.focus = Focus::MainPanel;
        app.handle_key(key(KeyCode::Char('/')));
        assert_eq!(app.input_mode, InputMode::Search);
        assert_eq!(app.log_search, Some(String::new()));
    }
}
