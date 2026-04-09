# Changelog

## [0.2.1](https://github.com/kennywillbe/rustydocker/compare/v0.2.0...v0.2.1) (2026-04-09)


### Bug Fixes

* bump ci workflow versions ([#13](https://github.com/kennywillbe/rustydocker/issues/13)) ([6bc9353](https://github.com/kennywillbe/rustydocker/commit/6bc93536c705a5a43265f3a02c762d178465bcea))

## 0.2.0 (2026-04-09)


### Features

* Log search with input mode, regex support, per-match highlighting
* Environment variables tab, process list tab (Top)
* Network panel with list and detail view
* Docker event stream for instant UI refresh
* Compose actions (up/down/restart)
* Pause/unpause, attach, open in browser
* Sidebar filtering with fuzzy find
* Config file support (~/.config/rustydocker/config.toml)
* CLI flags (-f, -p, --version)
* Screen modes (normal/half/fullscreen)
* Bulk commands menu, custom commands
* Multi-select containers with batch actions
* Container pinning/bookmarking
* Log bookmarks, log diff/snapshot
* Stats comparison split view
* Resource usage alerts
* Script hooks on Docker events
* Remote Docker host support
* Log export to file


### Bug Fixes

* Terminal panic guard
* Net stats delta calculation
* Memory stats cache subtraction
* Unicode-safe string truncation
* Selected index bounds clamping
* Confirmation dialogs for destructive actions


## 0.1.0 (2026-04-07)


### Features

* rustydocker — a fast, compose-first Docker TUI written in Rust
