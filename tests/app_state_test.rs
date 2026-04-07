use rustydocker::app::{App, Tab, AppAction, SidebarSection};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

#[test]
fn test_new_app_defaults() {
    let app = App::new();
    assert_eq!(app.active_tab, Tab::Logs);
    assert_eq!(app.sidebar_section, SidebarSection::Services);
    assert_eq!(app.selected_index, 0);
    assert!(app.running);
    assert!(!app.show_help);
    assert!(!app.show_cleanup);
}

#[test]
fn test_tab_cycling() {
    let mut app = App::new();
    assert_eq!(app.active_tab, Tab::Logs);
    app.next_tab();
    assert_eq!(app.active_tab, Tab::Stats);
    app.next_tab();
    assert_eq!(app.active_tab, Tab::Info);
    app.next_tab();
    assert_eq!(app.active_tab, Tab::Graph);
    app.next_tab();
    assert_eq!(app.active_tab, Tab::Logs); // wraps around
}

#[test]
fn test_navigation_empty_list() {
    let mut app = App::new();
    // No containers, should not panic
    app.next_item();
    assert_eq!(app.selected_index, 0);
    app.prev_item();
    assert_eq!(app.selected_index, 0);
}

#[test]
fn test_handle_key_quit() {
    let mut app = App::new();
    let action = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(action, AppAction::Quit);
}

#[test]
fn test_handle_key_ctrl_c_quit() {
    let mut app = App::new();
    let action = app.handle_key(ctrl_key(KeyCode::Char('c')));
    assert_eq!(action, AppAction::Quit);
}

#[test]
fn test_handle_key_help_toggle() {
    let mut app = App::new();
    assert!(!app.show_help);
    app.handle_key(key(KeyCode::Char('?')));
    assert!(app.show_help);
    app.handle_key(key(KeyCode::Char('?')));
    assert!(!app.show_help);
}

#[test]
fn test_handle_key_help_esc_closes() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('?')));
    assert!(app.show_help);
    app.handle_key(key(KeyCode::Esc));
    assert!(!app.show_help);
}

#[test]
fn test_handle_key_tab_switches() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.active_tab, Tab::Stats);
}

#[test]
fn test_handle_key_cleanup_toggle() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('x')));
    assert!(app.show_cleanup);
    // In cleanup mode, 'i' prunes images
    let action = app.handle_key(key(KeyCode::Char('i')));
    assert_eq!(action, AppAction::PruneImages);
}

#[test]
fn test_handle_key_cleanup_esc_closes() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('x')));
    assert!(app.show_cleanup);
    app.handle_key(key(KeyCode::Esc));
    assert!(!app.show_cleanup);
}

#[test]
fn test_handle_key_container_actions() {
    let mut app = App::new();
    assert_eq!(app.handle_key(key(KeyCode::Char('r'))), AppAction::RestartContainer);
    assert_eq!(app.handle_key(key(KeyCode::Char('s'))), AppAction::StopContainer);
    assert_eq!(app.handle_key(key(KeyCode::Char('u'))), AppAction::StartContainer);
    assert_eq!(app.handle_key(key(KeyCode::Char('d'))), AppAction::RemoveContainer);
    assert_eq!(app.handle_key(key(KeyCode::Char('e'))), AppAction::ExecShell);
}

#[test]
fn test_tab_labels() {
    assert_eq!(Tab::Logs.label(), "Logs");
    assert_eq!(Tab::Stats.label(), "Stats");
    assert_eq!(Tab::Info.label(), "Info");
    assert_eq!(Tab::Graph.label(), "Graph");
}

#[test]
fn test_tab_all() {
    let all = Tab::all();
    assert_eq!(all.len(), 4);
    assert_eq!(all[0], Tab::Logs);
    assert_eq!(all[3], Tab::Graph);
}

#[test]
fn test_handle_key_search_enter() {
    let mut app = App::new();
    let action = app.handle_key(key(KeyCode::Char('/')));
    assert_eq!(action, AppAction::EnterSearch);
    assert_eq!(app.log_search, Some(String::new()));
}

#[test]
fn test_help_mode_blocks_other_keys() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('?')));
    assert!(app.show_help);
    // While help is shown, 'r' should NOT trigger RestartContainer
    let action = app.handle_key(key(KeyCode::Char('r')));
    assert_eq!(action, AppAction::None);
}

#[test]
fn test_cleanup_mode_blocks_other_keys() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('x')));
    assert!(app.show_cleanup);
    // While cleanup is shown, 'r' should NOT trigger RestartContainer
    let action = app.handle_key(key(KeyCode::Char('r')));
    assert_eq!(action, AppAction::None);
}

#[test]
fn test_cleanup_prune_volumes() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('x')));
    let action = app.handle_key(key(KeyCode::Char('v')));
    assert_eq!(action, AppAction::PruneVolumes);
}

#[test]
fn test_selected_container_none_when_empty() {
    let app = App::new();
    assert!(app.selected_container().is_none());
    assert!(app.selected_container_id().is_none());
}

#[test]
fn test_stats_history_push_and_cap() {
    use rustydocker::app::StatsHistory;
    let mut h = StatsHistory::default();
    // Push 65 entries, should cap at 60
    for i in 0..65 {
        h.push(i as f64, i as f64, i as f64, i as f64, i as f64);
    }
    assert_eq!(h.cpu.len(), 60);
    assert_eq!(h.memory.len(), 60);
    assert_eq!(h.net_rx.len(), 60);
    assert_eq!(h.net_tx.len(), 60);
    // First element should be 5.0 (0..4 were removed)
    assert_eq!(h.cpu[0], 5.0);
    assert_eq!(h.cpu[59], 64.0);
}
