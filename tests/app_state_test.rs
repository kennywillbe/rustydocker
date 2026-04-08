use bollard::models::{ContainerSummary, ImageSummary};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rustydocker::app::{App, AppAction, Focus, InputMode, ScreenMode, SidebarSection, Tab};
use rustydocker::config::AppConfig;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

#[test]
fn test_new_app_defaults() {
    let app = App::new(AppConfig::default());
    assert_eq!(app.active_tab, Tab::Logs);
    assert_eq!(app.sidebar_section, SidebarSection::Services);
    assert_eq!(app.selected_index, 0);
    assert!(app.running);
    assert!(!app.show_help);
    assert!(!app.show_cleanup);
}

#[test]
fn test_tab_cycling() {
    let mut app = App::new(AppConfig::default());
    assert_eq!(app.active_tab, Tab::Logs);
    app.next_tab();
    assert_eq!(app.active_tab, Tab::Stats);
    app.next_tab();
    assert_eq!(app.active_tab, Tab::Info);
    app.next_tab();
    assert_eq!(app.active_tab, Tab::Env);
    app.next_tab();
    assert_eq!(app.active_tab, Tab::Top);
    app.next_tab();
    assert_eq!(app.active_tab, Tab::Graph);
    app.next_tab();
    assert_eq!(app.active_tab, Tab::Logs); // wraps around
}

#[test]
fn test_navigation_empty_list() {
    let mut app = App::new(AppConfig::default());
    // No containers, should not panic
    app.next_item();
    assert_eq!(app.selected_index, 0);
    app.prev_item();
    assert_eq!(app.selected_index, 0);
}

#[test]
fn test_handle_key_quit() {
    let mut app = App::new(AppConfig::default());
    let action = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(action, AppAction::Quit);
}

#[test]
fn test_handle_key_ctrl_c_quit() {
    let mut app = App::new(AppConfig::default());
    let action = app.handle_key(ctrl_key(KeyCode::Char('c')));
    assert_eq!(action, AppAction::Quit);
}

#[test]
fn test_handle_key_help_toggle() {
    let mut app = App::new(AppConfig::default());
    assert!(!app.show_help);
    app.handle_key(key(KeyCode::Char('?')));
    assert!(app.show_help);
    app.handle_key(key(KeyCode::Char('?')));
    assert!(!app.show_help);
}

#[test]
fn test_handle_key_help_esc_closes() {
    let mut app = App::new(AppConfig::default());
    app.handle_key(key(KeyCode::Char('?')));
    assert!(app.show_help);
    app.handle_key(key(KeyCode::Esc));
    assert!(!app.show_help);
}

#[test]
fn test_handle_key_tab_switches() {
    let mut app = App::new(AppConfig::default());
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.active_tab, Tab::Stats);
}

#[test]
fn test_handle_key_cleanup_toggle() {
    let mut app = App::new(AppConfig::default());
    app.handle_key(key(KeyCode::Char('x')));
    assert!(app.show_cleanup);
    // In cleanup mode, 'i' now triggers confirmation
    let action = app.handle_key(key(KeyCode::Char('i')));
    assert_eq!(action, AppAction::None);
    assert!(app.pending_confirm.is_some());
    // Confirm with 'y'
    let action = app.handle_key(key(KeyCode::Char('y')));
    assert_eq!(action, AppAction::PruneImages);
}

#[test]
fn test_handle_key_cleanup_esc_closes() {
    let mut app = App::new(AppConfig::default());
    app.handle_key(key(KeyCode::Char('x')));
    assert!(app.show_cleanup);
    app.handle_key(key(KeyCode::Esc));
    assert!(!app.show_cleanup);
}

#[test]
fn test_handle_key_container_actions() {
    let mut app = App::new(AppConfig::default());
    assert_eq!(app.handle_key(key(KeyCode::Char('r'))), AppAction::RestartContainer);
    assert_eq!(app.handle_key(key(KeyCode::Char('s'))), AppAction::StopContainer);
    assert_eq!(app.handle_key(key(KeyCode::Char('u'))), AppAction::StartContainer);
    // 'd' now requires confirmation
    assert_eq!(app.handle_key(key(KeyCode::Char('d'))), AppAction::None);
    assert!(app.pending_confirm.is_some());
    app.pending_confirm = None; // clear for next test
    assert_eq!(app.handle_key(key(KeyCode::Char('e'))), AppAction::ExecShell);
    assert_eq!(app.handle_key(key(KeyCode::Char('a'))), AppAction::AttachContainer);
}

#[test]
fn test_attach_container_action() {
    let mut app = App::new(AppConfig::default());
    let action = app.handle_key(key(KeyCode::Char('a')));
    assert_eq!(action, AppAction::AttachContainer);
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
    assert_eq!(all.len(), 6);
    assert_eq!(all[0], Tab::Logs);
    assert_eq!(all[3], Tab::Env);
    assert_eq!(all[4], Tab::Top);
    assert_eq!(all[5], Tab::Graph);
}

#[test]
fn test_handle_key_search_enter() {
    let mut app = App::new(AppConfig::default());
    app.focus = Focus::MainPanel;
    let action = app.handle_key(key(KeyCode::Char('/')));
    assert_eq!(action, AppAction::None);
    assert_eq!(app.input_mode, InputMode::Search);
    assert_eq!(app.log_search, Some(String::new()));
}

#[test]
fn test_search_mode_typing() {
    let mut app = App::new(AppConfig::default());
    app.focus = Focus::MainPanel;
    app.handle_key(key(KeyCode::Char('/')));
    assert_eq!(app.input_mode, InputMode::Search);
    assert_eq!(app.log_search, Some(String::new()));
    app.handle_key(key(KeyCode::Char('e')));
    app.handle_key(key(KeyCode::Char('r')));
    assert_eq!(app.log_search, Some("er".to_string()));
}

#[test]
fn test_search_mode_backspace() {
    let mut app = App::new(AppConfig::default());
    app.focus = Focus::MainPanel;
    app.handle_key(key(KeyCode::Char('/')));
    app.handle_key(key(KeyCode::Char('a')));
    app.handle_key(key(KeyCode::Char('b')));
    assert_eq!(app.log_search, Some("ab".to_string()));
    app.handle_key(key(KeyCode::Backspace));
    assert_eq!(app.log_search, Some("a".to_string()));
}

#[test]
fn test_search_mode_esc_clears() {
    let mut app = App::new(AppConfig::default());
    app.focus = Focus::MainPanel;
    app.handle_key(key(KeyCode::Char('/')));
    app.handle_key(key(KeyCode::Char('x')));
    assert_eq!(app.input_mode, InputMode::Search);
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.log_search, None);
}

#[test]
fn test_search_mode_enter_commits() {
    let mut app = App::new(AppConfig::default());
    app.focus = Focus::MainPanel;
    app.handle_key(key(KeyCode::Char('/')));
    app.handle_key(key(KeyCode::Char('e')));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.log_search, Some("e".to_string()));
}

#[test]
fn test_search_mode_blocks_actions() {
    let mut app = App::new(AppConfig::default());
    app.focus = Focus::MainPanel;
    app.handle_key(key(KeyCode::Char('/')));
    let action = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(action, AppAction::None);
    assert_eq!(app.log_search, Some("q".to_string()));
}

#[test]
fn test_help_mode_blocks_other_keys() {
    let mut app = App::new(AppConfig::default());
    app.handle_key(key(KeyCode::Char('?')));
    assert!(app.show_help);
    // While help is shown, 'r' should NOT trigger RestartContainer
    let action = app.handle_key(key(KeyCode::Char('r')));
    assert_eq!(action, AppAction::None);
}

#[test]
fn test_cleanup_mode_blocks_other_keys() {
    let mut app = App::new(AppConfig::default());
    app.handle_key(key(KeyCode::Char('x')));
    assert!(app.show_cleanup);
    // While cleanup is shown, 'r' should NOT trigger RestartContainer
    let action = app.handle_key(key(KeyCode::Char('r')));
    assert_eq!(action, AppAction::None);
}

#[test]
fn test_cleanup_prune_volumes() {
    let mut app = App::new(AppConfig::default());
    app.handle_key(key(KeyCode::Char('x')));
    let action = app.handle_key(key(KeyCode::Char('v')));
    assert_eq!(action, AppAction::None);
    assert!(app.pending_confirm.is_some());
    let action = app.handle_key(key(KeyCode::Char('y')));
    assert_eq!(action, AppAction::PruneVolumes);
}

#[test]
fn test_selected_container_none_when_empty() {
    let app = App::new(AppConfig::default());
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

#[test]
fn test_set_status_sets_message_and_timestamp() {
    let mut app = App::new(AppConfig::default());
    assert!(app.status_message.is_none());
    assert!(app.status_message_at.is_none());
    app.set_status("Restarting...");
    assert_eq!(app.status_message, Some("Restarting...".to_string()));
    assert!(app.status_message_at.is_some());
}

#[test]
fn test_clear_expired_status_does_not_clear_fresh() {
    let mut app = App::new(AppConfig::default());
    app.set_status("Fresh message");
    app.clear_expired_status();
    // Should NOT be cleared — it was just set
    assert!(app.status_message.is_some());
}

#[test]
fn test_clear_expired_status_clears_old() {
    let mut app = App::new(AppConfig::default());
    app.status_message = Some("Old message".to_string());
    app.status_message_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(5));
    app.clear_expired_status();
    assert!(app.status_message.is_none());
    assert!(app.status_message_at.is_none());
}

#[test]
fn test_selected_index_clamps_on_section_switch() {
    let mut app = App::new(AppConfig::default());
    app.containers = vec![create_dummy_container("c1"), create_dummy_container("c2")];
    app.images = vec![
        create_dummy_image("img1"),
        create_dummy_image("img2"),
        create_dummy_image("img3"),
        create_dummy_image("img4"),
        create_dummy_image("img5"),
    ];

    app.sidebar_section = SidebarSection::Images;
    app.selected_index = 4;

    app.sidebar_section = SidebarSection::Services;
    app.clamp_selected_index();
    assert_eq!(app.selected_index, 1); // clamped to last valid index
}

fn create_dummy_container(id: &str) -> ContainerSummary {
    create_container_with_state(id, "running")
}

fn create_container_with_state(name: &str, state: &str) -> ContainerSummary {
    ContainerSummary {
        id: Some(name.to_string()),
        names: Some(vec![format!("/{}", name)]),
        state: Some(state.to_string()),
        ..Default::default()
    }
}

#[test]
fn test_sort_containers() {
    let mut app = App::new(AppConfig::default());
    app.containers = vec![
        create_container_with_state("zz-stopped", "exited"),
        create_container_with_state("aa-running", "running"),
        create_container_with_state("bb-running", "running"),
        create_container_with_state("aa-stopped", "exited"),
    ];
    app.sort_containers();

    let names: Vec<String> = app
        .containers
        .iter()
        .map(|c| c.names.as_ref().unwrap()[0].trim_start_matches('/').to_string())
        .collect();
    assert_eq!(names, vec!["aa-running", "bb-running", "aa-stopped", "zz-stopped"]);
}

#[test]
fn test_pause_running_container() {
    let mut app = App::new(AppConfig::default());
    app.containers = vec![create_container_with_state("test", "running")];
    let action = app.handle_key(key(KeyCode::Char('p')));
    assert_eq!(action, AppAction::PauseContainer);
}

#[test]
fn test_unpause_paused_container() {
    let mut app = App::new(AppConfig::default());
    app.containers = vec![create_container_with_state("test", "paused")];
    let action = app.handle_key(key(KeyCode::Char('p')));
    assert_eq!(action, AppAction::UnpauseContainer);
}

#[test]
fn test_compose_up_action() {
    let mut app = App::new(AppConfig::default());
    let action = app.handle_key(KeyEvent::new(KeyCode::Char('U'), KeyModifiers::SHIFT));
    assert_eq!(action, AppAction::ComposeUp);
}

#[test]
fn test_compose_down_requires_confirmation() {
    let mut app = App::new(AppConfig::default());
    let action = app.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
    assert_eq!(action, AppAction::None);
    assert!(app.pending_confirm.is_some());
}

#[test]
fn test_compose_restart_action() {
    let mut app = App::new(AppConfig::default());
    let action = app.handle_key(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT));
    assert_eq!(action, AppAction::ComposeRestart);
}

#[test]
fn test_screen_mode_toggle() {
    let mut app = App::new(AppConfig::default());
    assert_eq!(app.screen_mode, ScreenMode::Normal);
    app.handle_key(key(KeyCode::Char('+')));
    assert_eq!(app.screen_mode, ScreenMode::Half);
    app.handle_key(key(KeyCode::Char('+')));
    assert_eq!(app.screen_mode, ScreenMode::Fullscreen);
    app.handle_key(key(KeyCode::Char('+')));
    assert_eq!(app.screen_mode, ScreenMode::Normal);
}

#[test]
fn test_open_in_browser_action() {
    let mut app = App::new(AppConfig::default());
    let action = app.handle_key(key(KeyCode::Char('w')));
    assert_eq!(action, AppAction::OpenInBrowser);
}

#[test]
fn test_export_logs_action() {
    let mut app = App::new(AppConfig::default());
    let action = app.handle_key(KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT));
    assert_eq!(action, AppAction::ExportLogs);
}

#[test]
fn test_multi_select_toggle() {
    let mut app = App::new(AppConfig::default());
    app.containers = vec![
        create_container_with_state("c1", "running"),
        create_container_with_state("c2", "running"),
        create_container_with_state("c3", "running"),
    ];
    assert!(app.selected_containers.is_empty());

    // Space selects c1 and moves to c2
    app.handle_key(key(KeyCode::Char(' ')));
    assert!(app.selected_containers.contains("c1"));
    assert_eq!(app.selected_index, 1);

    // Space selects c2 and moves to c3
    app.handle_key(key(KeyCode::Char(' ')));
    assert!(app.selected_containers.contains("c2"));
    assert_eq!(app.selected_index, 2);
}

#[test]
fn test_log_bookmark_toggle() {
    let mut app = App::new(AppConfig::default());
    app.containers = vec![create_container_with_state("c1", "running")];
    app.logs
        .insert("c1".to_string(), vec!["line1".into(), "line2".into(), "line3".into()]);
    app.active_tab = Tab::Logs;

    // Bookmark at bottom (scroll_offset=0, log_len=3, line_idx=2)
    app.handle_key(key(KeyCode::Char('m')));
    assert_eq!(app.log_bookmarks, vec![2]);

    // Toggle off
    app.handle_key(key(KeyCode::Char('m')));
    assert!(app.log_bookmarks.is_empty());
}

#[test]
fn test_log_snapshot_and_diff() {
    let mut app = App::new(AppConfig::default());
    app.containers = vec![create_container_with_state("c1", "running")];
    app.logs.insert("c1".to_string(), vec!["line1".into(), "line2".into()]);
    app.active_tab = Tab::Logs;

    // Take snapshot
    app.handle_key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT));
    assert!(app.log_snapshot.is_some());
    assert!(!app.show_log_diff);

    // Show diff
    app.handle_key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT));
    assert!(app.show_log_diff);

    // Exit diff
    app.handle_key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT));
    assert!(!app.show_log_diff);
    assert!(app.log_snapshot.is_none());
}

#[test]
fn test_stats_compare_toggle() {
    let mut app = App::new(AppConfig::default());
    app.containers = vec![create_container_with_state("c1", "running")];
    app.handle_key(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    assert_eq!(app.compare_container_id, Some("c1".to_string()));
    app.handle_key(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    assert_eq!(app.compare_container_id, None);
}

fn create_dummy_image(id: &str) -> ImageSummary {
    ImageSummary {
        id: id.to_string(),
        repo_tags: vec![format!("{}:latest", id)],
        size: 1000,
        created: 0,
        ..Default::default()
    }
}

#[test]
fn test_remove_container_requires_confirmation() {
    let mut app = App::new(AppConfig::default());
    let action = app.handle_key(key(KeyCode::Char('d')));
    assert_eq!(action, AppAction::None);
    assert!(app.pending_confirm.is_some());
}

#[test]
fn test_confirm_yes_returns_action() {
    let mut app = App::new(AppConfig::default());
    app.handle_key(key(KeyCode::Char('d')));
    let action = app.handle_key(key(KeyCode::Char('y')));
    assert_eq!(action, AppAction::RemoveContainer);
    assert!(app.pending_confirm.is_none());
}

#[test]
fn test_confirm_no_cancels() {
    let mut app = App::new(AppConfig::default());
    app.handle_key(key(KeyCode::Char('d')));
    let action = app.handle_key(key(KeyCode::Char('n')));
    assert_eq!(action, AppAction::None);
    assert!(app.pending_confirm.is_none());
}

#[test]
fn test_confirm_esc_cancels() {
    let mut app = App::new(AppConfig::default());
    app.handle_key(key(KeyCode::Char('d')));
    let action = app.handle_key(key(KeyCode::Esc));
    assert_eq!(action, AppAction::None);
    assert!(app.pending_confirm.is_none());
}
