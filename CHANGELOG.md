# Changelog

## [0.5.2](https://github.com/kennywillbe/rustydocker/compare/v0.5.1...v0.5.2) (2026-09-01)


### Dependencies

* bump the cargo-deps group across 1 directory with 2 updates ([#61](https://github.com/kennywillbe/rustydocker/issues/61)) ([57c7f2f](https://github.com/kennywillbe/rustydocker/commit/57c7f2f6348c1c00c88c8b3784d9c7d4f928a230))

## [0.5.1](https://github.com/kennywillbe/rustydocker/compare/v0.5.0...v0.5.1) (2026-09-01)


### Build System

* **deps:** bump clap from 4.6.1 to 4.6.6 ([#57](https://github.com/kennywillbe/rustydocker/issues/57)) ([b0ab5f5](https://github.com/kennywillbe/rustydocker/commit/b0ab5f565f600735794e39f863456415ddd9e492))
* **deps:** bump futures-util from 0.3.32 to 0.3.34 ([#59](https://github.com/kennywillbe/rustydocker/issues/59)) ([dcc2217](https://github.com/kennywillbe/rustydocker/commit/dcc2217e4cbe87747c6564e41cef63bde878632d))
* **deps:** bump tokio from 1.52.1 to 1.53.1 ([#56](https://github.com/kennywillbe/rustydocker/issues/56)) ([cdb14d2](https://github.com/kennywillbe/rustydocker/commit/cdb14d2f7fa88533e7f3aa54ef9662df35b4d573))
* **deps:** bump toml from 0.8.23 to 1.1.4+spec-1.1.0 ([#55](https://github.com/kennywillbe/rustydocker/issues/55)) ([e883652](https://github.com/kennywillbe/rustydocker/commit/e88365253244df73e8fa8bcfbaa22d26fe992335))
* **deps:** bump unicode-width from 0.2.0 to 0.2.2 ([#58](https://github.com/kennywillbe/rustydocker/issues/58)) ([ed8540f](https://github.com/kennywillbe/rustydocker/commit/ed8540fbc408429f2534aa466f3710530cd67756))

## [0.5.0](https://github.com/kennywillbe/rustydocker/compare/v0.4.3...v0.5.0) (2026-08-19)


### ⚠ BREAKING CHANGES

* DockerClient::new now accepts a resolved ConnectionSpec and the minimum supported Rust version is 1.88.

### Features

* ship v0.5 runtime and Docker workflows ([#53](https://github.com/kennywillbe/rustydocker/issues/53)) ([59cf063](https://github.com/kennywillbe/rustydocker/commit/59cf06343a190f65828e82503629893a105c3479))

## [0.4.3](https://github.com/kennywillbe/rustydocker/compare/v0.4.2...v0.4.3) (2026-08-18)


### Bug Fixes

* scope compose actions to startup options ([#51](https://github.com/kennywillbe/rustydocker/issues/51)) ([238ddbb](https://github.com/kennywillbe/rustydocker/commit/238ddbbeadae8f733773840e7163c4f8f37f7b38))

## [0.4.2](https://github.com/kennywillbe/rustydocker/compare/v0.4.1...v0.4.2) (2026-06-13)


### Bug Fixes

* make installer portable ([afd20b4](https://github.com/kennywillbe/rustydocker/commit/afd20b481f909e75c69a9db0411fa92f1c363940))

## [0.4.1](https://github.com/kennywillbe/rustydocker/compare/v0.4.0...v0.4.1) (2026-06-13)


### Bug Fixes

* resolve clippy warnings ([be8c4ae](https://github.com/kennywillbe/rustydocker/commit/be8c4ae982bde00a4781397909fd55b9519309e7))

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
