# Changelog

## [0.4.0](https://github.com/kennywillbe/rustydocker/compare/v0.3.1...v0.4.0) (2026-04-11)


### Features

* ember theme with compact stats and HTTP status highlighting ([#20](https://github.com/kennywillbe/rustydocker/issues/20)) ([cfb044e](https://github.com/kennywillbe/rustydocker/commit/cfb044e3714fcbe681397ecda9625895ff388179))
* rustydocker — a fast, compose-first Docker TUI written in Rust ([78ef268](https://github.com/kennywillbe/rustydocker/commit/78ef26891835952cc33affe77cb78eff773afd8e))


### Bug Fixes

* --version flag and in-app update check ([#22](https://github.com/kennywillbe/rustydocker/issues/22)) ([1695166](https://github.com/kennywillbe/rustydocker/commit/1695166986ae5c1b1b6bf5ea1f196488bf10e97f))
* use PAT for release-please CI trigger ([#15](https://github.com/kennywillbe/rustydocker/issues/15)) ([77aea83](https://github.com/kennywillbe/rustydocker/commit/77aea8393f22a6e1fa544b188c53bf1dd0edeec6))

## [0.3.1](https://github.com/kennywillbe/rustydocker/compare/v0.3.0...v0.3.1) (2026-04-11)


### Bug Fixes

* --version flag and in-app update check ([#22](https://github.com/kennywillbe/rustydocker/issues/22)) ([1695166](https://github.com/kennywillbe/rustydocker/commit/1695166986ae5c1b1b6bf5ea1f196488bf10e97f))

## [0.3.0](https://github.com/kennywillbe/rustydocker/compare/v0.2.0...v0.3.0) (2026-04-10)


### Features

* ember theme with compact stats and HTTP status highlighting ([#20](https://github.com/kennywillbe/rustydocker/issues/20)) ([cfb044e](https://github.com/kennywillbe/rustydocker/commit/cfb044e3714fcbe681397ecda9625895ff388179))


### Bug Fixes

* use PAT for release-please CI trigger ([#15](https://github.com/kennywillbe/rustydocker/issues/15)) ([77aea83](https://github.com/kennywillbe/rustydocker/commit/77aea8393f22a6e1fa544b188c53bf1dd0edeec6))

## [0.2.1](https://github.com/kennywillbe/rustydocker/compare/v0.2.0...v0.2.1) (2026-04-09)


### Bug Fixes

* use PAT for release-please CI trigger ([#15](https://github.com/kennywillbe/rustydocker/issues/15)) ([77aea83](https://github.com/kennywillbe/rustydocker/commit/77aea8393f22a6e1fa544b188c53bf1dd0edeec6))

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
