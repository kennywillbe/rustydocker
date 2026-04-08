# Remaining Phase 4 Tasks — Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development to implement task-by-task.

**Goal:** Complete the remaining Phase 4 roadmap items to differentiate rustydocker from lazydocker.

**Remaining tasks:** 4.1 (async stats), 4.6 (stats compare), 4.9 (log bookmarks), 4.11 (log diff), 4.12 (plugin), 4.13 (SSH remote), 4.14 (K8s)

**Feasibility assessment:**
- 4.1, 4.6, 4.9 — implementable this session
- 4.11 — implementable but complex
- 4.12, 4.13, 4.14 — long-term, skip for now (need significant architecture)

---

### Task 4.1: Async Parallel Stats Collection

**Problem:** Currently only the selected container's stats are streamed. When switching containers, previous stats history is lost (starts from scratch). Background stats collection for all running containers would make tab-switching instant.

**Files:**
- Modify: `src/main.rs` — spawn per-container stats tasks
- Modify: `src/app.rs` — stats are already stored per-container in `HashMap<String, StatsHistory>`

**Design:**

Instead of a single `stats_stream` for the selected container, spawn a background tokio task that manages stats streams for ALL running containers.

1. Create a stats collector that runs as a background task:
   - On startup, start stats streams for all running containers
   - When container list refreshes, start streams for new containers, drop streams for removed ones
   - Each stream pushes data into a shared `Arc<Mutex<HashMap<String, StatsHistory>>>` or use a channel

2. Simpler approach (recommended): Use the existing Docker event system. When container events fire, update stats streams. Keep using the `tokio::select!` approach but with multiple stats streams.

3. Simplest approach: On each tick (every 8 ticks), fetch a single stats snapshot for ALL running containers via `docker.container_stats()` with `one_shot: true`. This avoids managing multiple long-lived streams.

**Implementation (simplest approach):**

Add to DockerClient:
```rust
pub async fn container_stats_oneshot(&self, id: &str) -> Result<Stats> {
    let opts = StatsOptions { stream: false, one_shot: true };
    let mut stream = self.docker.stats(id, Some(opts));
    stream.next().await
        .ok_or_else(|| anyhow::anyhow!("No stats"))?
        .map_err(|e| e.into())
}
```

In main.rs tick handler, every 8 ticks, iterate all running containers and fetch one-shot stats:
```rust
if tick_count % 8 == 0 {
    for container in &app.containers {
        if container.state.as_deref() == Some("running") {
            if let Some(id) = &container.id {
                if let Ok(stats) = docker.container_stats_oneshot(id).await {
                    let snapshot = parse_stats(&stats);
                    let history = app.stats.entry(id.clone()).or_default();
                    history.push(...);
                }
            }
        }
    }
}
```

Remove the single `stats_stream` variable and its select arm.

**Tests:** Existing stats tests should still pass. No new tests needed (integration behavior).

---

### Task 4.6: Stats Comparison

**Problem:** No way to compare two containers' resource usage side by side.

**Files:**
- Modify: `src/app.rs` — add comparison state
- Create: `src/ui/stats_compare.rs` — split-view rendering
- Modify: `src/ui/mod.rs` — dispatch to comparison view

**Design:**

When user selects a container and presses `C` (shift+C), it enters comparison mode. The user navigates to a second container and presses `C` again to confirm. The Stats tab then shows two containers' charts side by side.

1. Add to App:
```rust
pub compare_container_id: Option<String>, // the "other" container to compare with
```

2. Keybinding `C`:
   - If `compare_container_id` is None → set it to current container, show status "Select second container to compare"
   - If `compare_container_id` is Some → comparison is active, pressing `C` again clears it

3. When `compare_container_id` is set and Stats tab is active, render split view:
   - Left half: selected container stats
   - Right half: comparison container stats
   - Use `Layout::Horizontal` with 50/50 split

4. Create `src/ui/stats_compare.rs`:
```rust
pub fn render_stats_compare(f: &mut Frame, area: Rect, app: &App) {
    let split = Layout::horizontal([Constraint::Ratio(1,2), Constraint::Ratio(1,2)]).split(area);
    // Render selected container stats in split[0]
    // Render compare container stats in split[1]
    // Reuse render_cpu/render_mem/render_net functions (make them pub)
}
```

5. In mod.rs Stats dispatch:
```rust
app::Tab::Stats => {
    if app.compare_container_id.is_some() {
        stats_compare::render_stats_compare(f, content_inner, app)
    } else {
        stats_panel::render_stats(f, content_inner, app)
    }
}
```

**Tests:**
```rust
#[test]
fn test_stats_compare_toggle() {
    let mut app = App::new(AppConfig::default());
    app.containers = vec![create_container_with_state("c1", "running")];
    app.handle_key(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    assert_eq!(app.compare_container_id, Some("c1".to_string()));
    app.handle_key(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    assert_eq!(app.compare_container_id, None);
}
```

---

### Task 4.9: Log Bookmarks

**Problem:** No way to mark interesting log lines and jump between them.

**Files:**
- Modify: `src/app.rs` — bookmark state
- Modify: `src/ui/logs.rs` — render bookmark indicators, navigation

**Design:**

1. Add to App:
```rust
pub log_bookmarks: Vec<usize>, // indices into the current container's log buffer
```
Initialize as `vec![]`. Clear when switching containers.

2. Keybindings (only when Logs tab active and main panel focused):
   - `m` — toggle bookmark on the current scroll position's top visible line
   - `n` — jump to next bookmark
   - `N` — jump to previous bookmark

3. Bookmark storage: store the log line index (not scroll offset). When rendering, check if each line's index is in `log_bookmarks` and show a `▶` marker.

4. In logs.rs rendering, for each line check if its index is bookmarked:
```rust
let bookmark_indicator = if app.log_bookmarks.contains(&line_index) {
    Span::styled("▶ ", Style::default().fg(Color::Yellow))
} else {
    Span::raw("  ")
};
```

5. Navigation: `n` finds the next bookmark index after current scroll position and sets `log_scroll_offset` to show that line. `N` finds the previous.

6. `m` key handler in App:
```rust
KeyCode::Char('m') => {
    if self.active_tab == Tab::Logs && self.focus == Focus::MainPanel {
        // Calculate current top visible line from scroll offset
        let container_id = self.selected_container_id().map(|s| s.to_string());
        if let Some(id) = container_id {
            let log_len = self.logs.get(&id).map(|l| l.len()).unwrap_or(0);
            let line_idx = log_len.saturating_sub(self.log_scroll_offset as usize + 1);
            if self.log_bookmarks.contains(&line_idx) {
                self.log_bookmarks.retain(|&b| b != line_idx);
            } else {
                self.log_bookmarks.push(line_idx);
                self.log_bookmarks.sort();
            }
        }
    }
}
```

**Tests:**
```rust
#[test]
fn test_log_bookmark_toggle() {
    let mut app = App::new(AppConfig::default());
    app.containers = vec![create_container_with_state("c1", "running")];
    app.logs.insert("c1".to_string(), vec!["line1".into(), "line2".into(), "line3".into()]);
    app.active_tab = Tab::Logs;
    app.focus = Focus::MainPanel;
    app.handle_key(key(KeyCode::Char('m')));
    assert_eq!(app.log_bookmarks.len(), 1);
    // Toggle off
    app.handle_key(key(KeyCode::Char('m')));
    assert!(app.log_bookmarks.is_empty());
}
```

---

### Task 4.11: Container Log Diff

**Problem:** No way to see what changed in logs after a container restart.

**Files:**
- Modify: `src/app.rs` — snapshot state
- Create: `src/ui/log_diff.rs` — diff rendering
- Modify: `src/ui/mod.rs` — dispatch

**Design:**

1. Add snapshot capability: pressing `T` takes a "snapshot" of current log buffer.
2. Pressing `T` again shows a diff view comparing snapshot vs current logs.
3. New lines since snapshot are highlighted green. Missing lines (if container restarted) shown in red.

Add to App:
```rust
pub log_snapshot: Option<Vec<String>>, // saved log state
pub show_log_diff: bool,
```

Keybinding `T`:
```rust
KeyCode::Char('T') => {
    if self.active_tab == Tab::Logs {
        if self.log_snapshot.is_some() && !self.show_log_diff {
            self.show_log_diff = true;
        } else if self.show_log_diff {
            self.show_log_diff = false;
            self.log_snapshot = None;
        } else {
            // Take snapshot
            if let Some(id) = self.selected_container_id() {
                self.log_snapshot = self.logs.get(id).cloned();
                self.set_status("Log snapshot taken. Press T again to compare.");
            }
        }
    }
}
```

Diff rendering: simple approach — show lines in current logs that weren't in snapshot (green) and vice versa. Use a HashSet for O(1) lookup.

```rust
pub fn render_log_diff(f: &mut Frame, area: Rect, app: &App) {
    let current = app.logs.get(app.selected_container_id().unwrap_or("")).cloned().unwrap_or_default();
    let snapshot = app.log_snapshot.as_deref().unwrap_or(&[]);

    let snapshot_set: HashSet<&str> = snapshot.iter().map(|s| s.as_str()).collect();

    let lines: Vec<Line> = current.iter().map(|line| {
        if snapshot_set.contains(line.as_str()) {
            Line::from(Span::styled(line.clone(), Style::default().fg(Color::DarkGray)))
        } else {
            Line::from(Span::styled(format!("+ {}", line), Style::default().fg(Color::Green)))
        }
    }).collect();

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let scroll_y = lines.len().saturating_sub(area.height as usize) as u16;
    f.render_widget(paragraph.scroll((scroll_y, 0)), area);
}
```

**Tests:**
```rust
#[test]
fn test_log_snapshot() {
    let mut app = App::new(AppConfig::default());
    app.containers = vec![create_container_with_state("c1", "running")];
    app.logs.insert("c1".to_string(), vec!["line1".into()]);
    app.active_tab = Tab::Logs;
    app.handle_key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT));
    assert!(app.log_snapshot.is_some());
}
```

---

### Task 4.12: Plugin System (Long-term — Simplified Version)

**Problem:** Users want to extend rustydocker with custom behavior.

**Simplified approach:** Instead of full Lua/WASM, implement a "script hooks" system where users define shell scripts in config that run on events.

**Files:**
- Modify: `src/config.rs` — add hooks config
- Modify: `src/main.rs` — run hooks on events

**Design:**

Config:
```toml
[[hooks]]
event = "container_start"
command = "notify-send 'Container {container_name} started'"

[[hooks]]
event = "container_stop"
command = "echo '{container_name} stopped' >> ~/docker.log"
```

Supported events: `container_start`, `container_stop`, `container_restart`, `container_remove`

In the Docker event handler, after processing each event, check hooks and spawn matching commands in background.

This is much simpler than a plugin system but gives 80% of the value.

---

### Task 4.13: SSH Remote Docker (Long-term — Simplified)

**Problem:** Can't manage Docker on remote hosts.

**Simplified approach:** Support `DOCKER_HOST` environment variable. Bollard already supports this — just need to pass it through.

**Files:**
- Modify: `src/docker/client.rs` — use DOCKER_HOST if set
- Modify: `src/config.rs` — add docker_host config option
- Modify: `src/ui/mod.rs` — show connected host in title

**Design:**

Bollard's `Docker::connect_with_local_defaults()` already respects `DOCKER_HOST`. Add a config option:

```toml
docker_host = "tcp://192.168.1.100:2376"
```

In DockerClient::new():
```rust
pub fn new(docker_host: Option<&str>) -> Result<Self> {
    let docker = if let Some(host) = docker_host {
        Docker::connect_with_http(host, 120, bollard::API_DEFAULT_VERSION)?
    } else {
        Docker::connect_with_local_defaults()?
    };
    Ok(Self { docker })
}
```

Show host in sidebar title: `" rustydocker (remote) "` when connected to non-local host.

---

### Task 4.14: Kubernetes Support (Long-term — Out of Scope)

This requires a fundamentally different data model (pods, deployments, services, namespaces vs containers, images, volumes). It would essentially be a separate application mode.

**Recommendation:** Skip for now. If pursued later, implement as a separate binary or feature flag.

---

## Execution Order

1. **4.1 Async parallel stats** — changes main loop, do first
2. **4.9 Log bookmarks** — self-contained UI feature
3. **4.11 Log diff** — builds on log infrastructure
4. **4.6 Stats comparison** — needs async stats from 4.1
5. **4.12 Script hooks** — simplified plugin system
6. **4.13 Remote Docker** — config + bollard change
7. ~~4.14 K8s~~ — skip
