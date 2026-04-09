# Phase 1: Bugfix & Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix broken features and fill foundational gaps so rustydocker works correctly before adding new capabilities.

**Architecture:** All 6 tasks modify existing modules (`app.rs`, `main.rs`, `ui/mod.rs`). Two new UI widgets are created: a search input bar and a confirmation dialog. The app state machine gains new modes (searching, confirming) that intercept key events before normal handling.

**Tech Stack:** Rust, ratatui 0.29, crossterm 0.28, bollard, tokio, chrono (for status message timestamps)

---

### Task 1: Fix Log Search

The `/` keybinding sets `app.log_search = Some("")` and returns `AppAction::EnterSearch`, but `main.rs` ignores that action (`_ => {}`). There is no way to type characters into the search field. The search highlight code in `logs.rs` works — it just never receives input.

**Files:**
- Modify: `src/app.rs` — add `InputMode` enum, search input handling in `handle_key`
- Modify: `src/ui/mod.rs` — render search input bar at bottom of main panel
- Modify: `src/ui/logs.rs` — filter logs by search query
- Test: `tests/app_state_test.rs` — search mode tests

- [ ] **Step 1: Write failing tests for search mode**

Add these tests to `tests/app_state_test.rs`:

```rust
#[test]
fn test_search_mode_typing() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('/')));
    assert_eq!(app.input_mode, InputMode::Search);
    assert_eq!(app.log_search, Some(String::new()));

    // Type "error"
    app.handle_key(key(KeyCode::Char('e')));
    app.handle_key(key(KeyCode::Char('r')));
    assert_eq!(app.log_search, Some("er".to_string()));
}

#[test]
fn test_search_mode_backspace() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('/')));
    app.handle_key(key(KeyCode::Char('a')));
    app.handle_key(key(KeyCode::Char('b')));
    assert_eq!(app.log_search, Some("ab".to_string()));

    app.handle_key(key(KeyCode::Backspace));
    assert_eq!(app.log_search, Some("a".to_string()));
}

#[test]
fn test_search_mode_esc_clears() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('/')));
    app.handle_key(key(KeyCode::Char('x')));
    assert_eq!(app.input_mode, InputMode::Search);

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.log_search, None);
}

#[test]
fn test_search_mode_enter_commits() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('/')));
    app.handle_key(key(KeyCode::Char('e')));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.input_mode, InputMode::Normal);
    // Search stays active after Enter
    assert_eq!(app.log_search, Some("e".to_string()));
}

#[test]
fn test_search_mode_blocks_actions() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('/')));
    // 'q' should NOT quit while searching — it types 'q'
    let action = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(action, AppAction::None);
    assert_eq!(app.log_search, Some("q".to_string()));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test`
Expected: FAIL — `InputMode` does not exist

- [ ] **Step 3: Add InputMode enum and search state to App**

In `src/app.rs`, add the enum after the `Focus` enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
}
```

Add `input_mode` field to the `App` struct:

```rust
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
    pub input_mode: InputMode,
}
```

Initialize it in `App::new()`:

```rust
input_mode: InputMode::Normal,
```

- [ ] **Step 4: Implement search input handling in handle_key**

In `src/app.rs`, add search mode handling at the top of `handle_key`, right after the `show_cleanup` block and before the main `match key.code`:

```rust
if self.input_mode == InputMode::Search {
    match key.code {
        KeyCode::Esc => {
            self.input_mode = InputMode::Normal;
            self.log_search = None;
        }
        KeyCode::Enter => {
            self.input_mode = InputMode::Normal;
            // Keep log_search active for continued highlighting
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
```

Also update the `/` handler to set input_mode:

```rust
KeyCode::Char('/') => {
    self.log_search = Some(String::new());
    self.input_mode = InputMode::Search;
    return AppAction::None;
}
```

Remove `AppAction::EnterSearch` from the enum since it's no longer needed. Update the `'/'` match arm to no longer return `EnterSearch`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test`
Expected: All 24 tests pass (19 existing + 5 new)

- [ ] **Step 6: Render search bar in UI**

In `src/ui/mod.rs`, add search bar rendering in `draw()` right before the popups section. When `app.input_mode == InputMode::Search`, render a one-line input at the bottom of `content_inner`:

```rust
use crate::app::InputMode;

// In draw(), after rendering main content and before popups:
if app.input_mode == InputMode::Search {
    let search_text = app.log_search.as_deref().unwrap_or("");
    let search_line = Line::from(vec![
        Span::styled(" /", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(search_text),
        Span::styled("_", Style::default().fg(Color::Yellow)),
    ]);
    let search_bar_area = Rect::new(
        content_inner.x,
        content_inner.y + content_inner.height.saturating_sub(1),
        content_inner.width,
        1,
    );
    f.render_widget(
        Paragraph::new(search_line).style(Style::default().bg(Color::Rgb(40, 40, 60))),
        search_bar_area,
    );
}
```

- [ ] **Step 7: Update existing test that checks EnterSearch**

In `tests/app_state_test.rs`, update `test_handle_key_search_enter`:

```rust
#[test]
fn test_handle_key_search_enter() {
    let mut app = App::new();
    let action = app.handle_key(key(KeyCode::Char('/')));
    assert_eq!(action, AppAction::None);
    assert_eq!(app.log_search, Some(String::new()));
    assert_eq!(app.input_mode, InputMode::Search);
}
```

Update the import line at the top to include `InputMode`:

```rust
use rustydocker::app::{App, Tab, AppAction, SidebarSection, InputMode};
```

- [ ] **Step 8: Remove AppAction::EnterSearch variant**

In `src/app.rs`, remove `EnterSearch` from the `AppAction` enum.

- [ ] **Step 9: Run all tests, verify everything passes**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 10: Commit**

```bash
git add src/app.rs src/ui/mod.rs tests/app_state_test.rs
git commit -m "feat: implement working log search with input mode"
```

---

### Task 2: Render Status Messages

`app.status_message` is set by container actions (restart, stop, prune, etc.) but never rendered. There's also no timeout mechanism — messages would stay forever.

**Files:**
- Modify: `src/app.rs` — add `status_message_at` timestamp field
- Modify: `src/ui/mod.rs` — render status message in status bar, with color
- Modify: `src/main.rs` — clear expired messages on tick

- [ ] **Step 1: Add timestamp field to App**

In `src/app.rs`, add a field to track when the message was set:

```rust
pub status_message: Option<String>,
pub status_message_at: Option<std::time::Instant>,
```

Initialize in `App::new()`:

```rust
status_message_at: None,
```

Add a helper method:

```rust
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
```

- [ ] **Step 2: Render status message in status bar**

In `src/ui/mod.rs`, replace the status bar rendering section (the `status_line` construction and `f.render_widget` for `status_bar`) with:

```rust
let status_line = if let Some(ref msg) = app.status_message {
    let color = if msg.contains("pruned") || msg.contains("Started") {
        Color::Green
    } else if msg.contains("Error") || msg.contains("Failed") {
        Color::Rgb(255, 80, 80)
    } else {
        Color::Yellow
    };
    Line::from(vec![
        Span::styled(format!(" {}", msg), Style::default().fg(color)),
    ])
} else {
    let running = app
        .containers
        .iter()
        .filter(|c| c.state.as_deref() == Some("running"))
        .count();
    let stopped = app.containers.len() - running;
    let status_left = format!(" \u{25cf} {} running  \u{25cb} {} stopped", running, stopped);
    let status_right = " ?:help  x:cleanup  q:quit ";
    Line::from(vec![
        Span::styled(status_left, Style::default().fg(Color::Green)),
        Span::styled(" \u{2502} ", Style::default().fg(Color::DarkGray)),
        Span::styled(status_right, Style::default().fg(Color::DarkGray)),
    ])
};
f.render_widget(
    Paragraph::new(status_line).style(Style::default().bg(Color::Rgb(30, 30, 40))),
    app_layout.status_bar,
);
```

- [ ] **Step 3: Use set_status in main.rs**

In `src/main.rs`, replace all `app.status_message = Some(...)` calls with `app.set_status(...)`:

```rust
// Example: change this:
app.status_message = Some("Restarting...".to_string());
// To this:
app.set_status("Restarting...");
```

Do this for all 7 occurrences: Restarting, Stopping, Starting, Removing, Images pruned, Volumes pruned.

- [ ] **Step 4: Clear expired messages on tick**

In `src/main.rs`, inside the `AppEvent::Tick` handler, add:

```rust
AppEvent::Tick => {
    tick_count += 1;
    app.clear_expired_status();
    if tick_count.is_multiple_of(10) {
        app.containers = docker.list_containers().await.unwrap_or_default();
    }
}
```

- [ ] **Step 5: Run tests, verify everything passes**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/ui/mod.rs src/main.rs
git commit -m "feat: render status messages in status bar with auto-clear"
```

---

### Task 3: Improve Error Handling

Docker API calls in `main.rs` silently swallow errors with `let _ = docker.restart_container(...)`. Users get no feedback when an operation fails.

**Files:**
- Modify: `src/main.rs` — show errors via `app.set_status`

- [ ] **Step 1: Replace silent error swallowing with status messages**

In `src/main.rs`, change each container action from ignoring the result to reporting errors. Apply this pattern to all 5 container actions (restart, stop, start, remove) and 2 prune actions:

```rust
AppAction::RestartContainer => {
    if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
        match docker.restart_container(&id).await {
            Ok(_) => app.set_status("Restarting..."),
            Err(e) => app.set_status(&format!("Error: {}", e)),
        }
    }
}
AppAction::StopContainer => {
    if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
        match docker.stop_container(&id).await {
            Ok(_) => app.set_status("Stopping..."),
            Err(e) => app.set_status(&format!("Error: {}", e)),
        }
    }
}
AppAction::StartContainer => {
    if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
        match docker.start_container(&id).await {
            Ok(_) => app.set_status("Starting..."),
            Err(e) => app.set_status(&format!("Error: {}", e)),
        }
    }
}
AppAction::RemoveContainer => {
    if let Some(id) = app.selected_container_id().map(|s| s.to_string()) {
        match docker.remove_container(&id).await {
            Ok(_) => app.set_status("Removed"),
            Err(e) => app.set_status(&format!("Error: {}", e)),
        }
    }
}
AppAction::PruneImages => {
    match docker.prune_images().await {
        Ok(_) => app.set_status("Images pruned"),
        Err(e) => app.set_status(&format!("Error: {}", e)),
    }
    app.images = docker.list_images().await.unwrap_or_default();
}
AppAction::PruneVolumes => {
    match docker.prune_volumes().await {
        Ok(_) => app.set_status("Volumes pruned"),
        Err(e) => app.set_status(&format!("Error: {}", e)),
    }
    app.volumes = docker.list_volumes().await.unwrap_or_default();
}
```

- [ ] **Step 2: Run tests, verify everything passes**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "fix: show error messages instead of silently swallowing failures"
```

---

### Task 4: Fix Selected Index Bounds

When switching from a sidebar section with many items (e.g. 30 volumes) to one with fewer items (e.g. 3 containers), `selected_index` stays at 30 — causing out-of-bounds access.

**Files:**
- Modify: `src/app.rs` — clamp index on section switch
- Modify: `src/ui/mod.rs` — guard `selected_index` before accessing lists
- Test: `tests/app_state_test.rs`

- [ ] **Step 1: Write failing test**

Add to `tests/app_state_test.rs`:

```rust
#[test]
fn test_selected_index_clamps_on_section_switch() {
    let mut app = App::new();
    // Simulate: 2 containers, 5 images
    app.containers = vec![
        create_dummy_container("c1"),
        create_dummy_container("c2"),
    ];
    app.images = vec![
        create_dummy_image("img1"),
        create_dummy_image("img2"),
        create_dummy_image("img3"),
        create_dummy_image("img4"),
        create_dummy_image("img5"),
    ];

    // Navigate to images section, select index 4
    app.sidebar_section = SidebarSection::Images;
    app.selected_index = 4;

    // Switch back to Services — index 4 would be out of bounds for 2 containers
    app.sidebar_section = SidebarSection::Services;
    app.clamp_selected_index();
    assert_eq!(app.selected_index, 1); // clamped to last valid index
}
```

Also add these helper functions at the bottom of the test file:

```rust
fn create_dummy_container(id: &str) -> ContainerSummary {
    ContainerSummary {
        id: Some(id.to_string()),
        names: Some(vec![format!("/{}", id)]),
        state: Some("running".to_string()),
        ..Default::default()
    }
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
```

Add `ContainerSummary` and `ImageSummary` to the imports:

```rust
use bollard::models::{ContainerSummary, ImageSummary};
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_selected_index_clamps`
Expected: FAIL — `clamp_selected_index` method does not exist

- [ ] **Step 3: Add clamp_selected_index method**

In `src/app.rs`, add this method to the `App` impl:

```rust
pub fn clamp_selected_index(&mut self) {
    let len = self.current_list_len();
    if len == 0 {
        self.selected_index = 0;
    } else if self.selected_index >= len {
        self.selected_index = len - 1;
    }
}
```

- [ ] **Step 4: Call clamp after every section change**

In `src/app.rs`, the `next_item()` method already sets `selected_index = 0` when switching sections — that's fine. But `handle_mouse` sets `sidebar_section` directly without clamping. Add a clamp call at the end of `handle_mouse`:

At the very end of `handle_mouse`, before the closing `}`, add:

```rust
self.clamp_selected_index();
```

- [ ] **Step 5: Guard image/volume detail rendering in mod.rs**

In `src/ui/mod.rs`, the `render_image_detail` and `render_volume_detail` functions use `app.selected_index` directly. They already handle `None` from `.get()`, so they're safe. No change needed here.

- [ ] **Step 6: Clamp after container list refresh in main.rs**

In `src/main.rs`, after the periodic container refresh (inside `AppEvent::Tick`), add a clamp:

```rust
if tick_count.is_multiple_of(10) {
    app.containers = docker.list_containers().await.unwrap_or_default();
    app.clamp_selected_index();
}
```

- [ ] **Step 7: Run all tests**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 8: Commit**

```bash
git add src/app.rs src/main.rs tests/app_state_test.rs
git commit -m "fix: clamp selected index when switching sections or refreshing lists"
```

---

### Task 5: Container Sorting

Containers appear in whatever order the Docker API returns them. Running containers should appear first, then by name alphabetically.

**Files:**
- Modify: `src/app.rs` — add `sort_containers` method
- Modify: `src/main.rs` — call sort after every container list refresh
- Test: `tests/app_state_test.rs`

- [ ] **Step 1: Write failing test**

Add to `tests/app_state_test.rs`:

```rust
#[test]
fn test_sort_containers() {
    let mut app = App::new();
    app.containers = vec![
        create_container_with_state("zz-stopped", "exited"),
        create_container_with_state("aa-running", "running"),
        create_container_with_state("bb-running", "running"),
        create_container_with_state("aa-stopped", "exited"),
    ];
    app.sort_containers();

    let names: Vec<String> = app.containers.iter()
        .map(|c| c.names.as_ref().unwrap()[0].trim_start_matches('/').to_string())
        .collect();
    assert_eq!(names, vec!["aa-running", "bb-running", "aa-stopped", "zz-stopped"]);
}
```

Add the helper:

```rust
fn create_container_with_state(name: &str, state: &str) -> ContainerSummary {
    ContainerSummary {
        id: Some(name.to_string()),
        names: Some(vec![format!("/{}", name)]),
        state: Some(state.to_string()),
        ..Default::default()
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_sort_containers`
Expected: FAIL — `sort_containers` method does not exist

- [ ] **Step 3: Implement sort_containers**

In `src/app.rs`, add to the `App` impl:

```rust
pub fn sort_containers(&mut self) {
    self.containers.sort_by(|a, b| {
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
        let name_a = a.names.as_ref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/'))
            .unwrap_or("");
        let name_b = b.names.as_ref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/'))
            .unwrap_or("");
        name_a.to_lowercase().cmp(&name_b.to_lowercase())
    });
}
```

- [ ] **Step 4: Call sort in main.rs**

In `src/main.rs`, call `app.sort_containers()` in two places:

1. After initial data load (after `app.containers = docker.list_containers()...`):
```rust
app.containers = docker.list_containers().await.unwrap_or_default();
app.sort_containers();
```

2. After periodic refresh in the Tick handler:
```rust
app.containers = docker.list_containers().await.unwrap_or_default();
app.sort_containers();
app.clamp_selected_index();
```

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/main.rs tests/app_state_test.rs
git commit -m "feat: sort containers by state (running first) then alphabetically"
```

---

### Task 6: Confirmation Dialogs

Destructive actions (remove container, prune images/volumes) execute immediately with no confirmation. Add a confirmation dialog.

**Files:**
- Modify: `src/app.rs` — add `PendingAction` enum, confirmation state
- Create: `src/ui/confirm.rs` — confirmation dialog widget
- Modify: `src/ui/mod.rs` — register module, render dialog
- Modify: `src/main.rs` — handle confirmed actions
- Test: `tests/app_state_test.rs`

- [ ] **Step 1: Write failing tests**

Add to `tests/app_state_test.rs`:

```rust
#[test]
fn test_remove_container_requires_confirmation() {
    let mut app = App::new();
    let action = app.handle_key(key(KeyCode::Char('d')));
    assert_eq!(action, AppAction::None);
    assert!(app.pending_confirm.is_some());
}

#[test]
fn test_confirm_yes_returns_action() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('d'))); // trigger confirm
    let action = app.handle_key(key(KeyCode::Char('y')));
    assert_eq!(action, AppAction::RemoveContainer);
    assert!(app.pending_confirm.is_none());
}

#[test]
fn test_confirm_no_cancels() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('d'))); // trigger confirm
    let action = app.handle_key(key(KeyCode::Char('n')));
    assert_eq!(action, AppAction::None);
    assert!(app.pending_confirm.is_none());
}

#[test]
fn test_confirm_esc_cancels() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('d'))); // trigger confirm
    let action = app.handle_key(key(KeyCode::Esc));
    assert_eq!(action, AppAction::None);
    assert!(app.pending_confirm.is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_remove_container_requires`
Expected: FAIL — `pending_confirm` field does not exist

- [ ] **Step 3: Add PendingAction and confirmation state to App**

In `src/app.rs`, add before `AppAction`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct PendingConfirm {
    pub message: String,
    pub action: AppAction,
}
```

Add `pending_confirm` field to `App`:

```rust
pub pending_confirm: Option<PendingConfirm>,
```

Initialize in `App::new()`:

```rust
pending_confirm: None,
```

- [ ] **Step 4: Add confirmation handling to handle_key**

In `src/app.rs`, add confirmation dialog handling after the `show_cleanup` block and before the `input_mode == InputMode::Search` block:

```rust
if let Some(ref pending) = self.pending_confirm {
    let action = pending.action.clone();
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => {
            self.pending_confirm = None;
            return action;
        }
        _ => {
            self.pending_confirm = None;
            return AppAction::None;
        }
    }
}
```

Change the `'d'` handler to set pending_confirm instead of returning directly:

```rust
KeyCode::Char('d') => {
    self.pending_confirm = Some(PendingConfirm {
        message: "Remove this container? (y/n)".to_string(),
        action: AppAction::RemoveContainer,
    });
    return AppAction::None;
}
```

Also make the prune actions in cleanup mode go through confirmation. Change the cleanup handler:

```rust
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
```

Add `Clone` derive to `AppAction`:

```rust
#[derive(Debug, Clone, PartialEq)]
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
}
```

- [ ] **Step 5: Create confirmation dialog widget**

Create `src/ui/confirm.rs`:

```rust
use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render_confirm(f: &mut Frame, area: Rect, app: &App) {
    let pending = match &app.pending_confirm {
        Some(p) => p,
        None => return,
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::raw(format!("  {}", pending.message))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" confirm    "),
            Span::styled("n/Esc", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" cancel"),
        ]),
    ];

    let popup_width = 42;
    let popup_height = text.len() as u16 + 2;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm ")
        .title_style(Style::default().fg(Color::Yellow))
        .border_style(Style::default().fg(Color::Yellow));

    f.render_widget(Clear, popup_area);
    f.render_widget(Paragraph::new(text).block(block), popup_area);
}
```

- [ ] **Step 6: Register and render the confirmation dialog**

In `src/ui/mod.rs`, add the module:

```rust
pub mod confirm;
```

In the `draw()` function, add after the help/cleanup popups:

```rust
if app.pending_confirm.is_some() {
    confirm::render_confirm(f, f.area(), app);
}
```

- [ ] **Step 7: Update existing tests that expect direct AppAction from 'd'**

In `tests/app_state_test.rs`, update `test_handle_key_container_actions`:

```rust
#[test]
fn test_handle_key_container_actions() {
    let mut app = App::new();
    assert_eq!(app.handle_key(key(KeyCode::Char('r'))), AppAction::RestartContainer);
    assert_eq!(app.handle_key(key(KeyCode::Char('s'))), AppAction::StopContainer);
    assert_eq!(app.handle_key(key(KeyCode::Char('u'))), AppAction::StartContainer);
    // 'd' now requires confirmation
    assert_eq!(app.handle_key(key(KeyCode::Char('d'))), AppAction::None);
    assert!(app.pending_confirm.is_some());
    assert_eq!(app.handle_key(key(KeyCode::Char('e'))), AppAction::ExecShell);
}
```

Also update `test_cleanup_mode_blocks_other_keys` — after pressing 'i' in cleanup mode, the cleanup closes and a confirmation appears, so 'r' would be handled by the confirmation handler (which cancels on non-y):

```rust
#[test]
fn test_cleanup_prune_images_requires_confirmation() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('x')));
    assert!(app.show_cleanup);
    // 'i' now triggers confirmation instead of direct action
    let action = app.handle_key(key(KeyCode::Char('i')));
    assert_eq!(action, AppAction::None);
    assert!(app.pending_confirm.is_some());
    // Confirm with 'y'
    let action = app.handle_key(key(KeyCode::Char('y')));
    assert_eq!(action, AppAction::PruneImages);
}
```

And `test_cleanup_prune_volumes`:

```rust
#[test]
fn test_cleanup_prune_volumes_requires_confirmation() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('x')));
    let action = app.handle_key(key(KeyCode::Char('v')));
    assert_eq!(action, AppAction::None);
    assert!(app.pending_confirm.is_some());
    let action = app.handle_key(key(KeyCode::Char('y')));
    assert_eq!(action, AppAction::PruneVolumes);
}
```

- [ ] **Step 8: Run all tests**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 9: Commit**

```bash
git add src/app.rs src/ui/confirm.rs src/ui/mod.rs src/main.rs tests/app_state_test.rs
git commit -m "feat: add confirmation dialogs for destructive actions"
```
