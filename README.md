# rustydocker

A fast, compose-first Docker TUI written in Rust. A modern alternative to lazydocker.

[![CI](https://github.com/kennywillbe/rustydocker/actions/workflows/ci.yml/badge.svg)](https://github.com/kennywillbe/rustydocker/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88+-orange)](https://www.rust-lang.org/)

![rustydocker](docs/demo.gif)

Streaming logs with a regex search over them, live stats, the container's
environment, and the `depends_on` graph — one Compose project, no `docker`
subcommands.

The default `ember` theme ships with a warm amber-on-black palette. Set
`theme = "classic"` in `~/.config/rustydocker/config.toml` to switch back
to the original cyan/green look.

## Features

- **Compose-first** — services grouped by Docker Compose project
- **Real-time logs** — concurrent, reconnecting streams for every running container with regex search (`/`)
- **Live stats** — CPU, memory, and network delta charts (KB/s)
- **Dependency graph** — visualize `depends_on` relationships (unique to rustydocker)
- **Environment variables** — inspect container env vars
- **Process list** — live `docker top` view with auto-refresh
- **Health check history** — recent healthcheck results with timestamps
- **Network management** — list, inspect Docker networks
- **Docker event stream** — instant UI refresh on container/image/volume changes
- **Effective Compose model** — merged files, interpolation, profiles, include/extends, build metadata
- **Compose actions** — up/down/restart, pull, rebuild, and watch from the TUI
- **Image workflows** — inspect platform/digest/layers, layer history, pull, and remove
- **Docker contexts** — current context, `--context`, `DOCKER_CONTEXT`, SSH, and verified TLS endpoints
- **Bulk commands** — stop all, remove stopped, prune containers/images/volumes/networks
- **Multi-select** — Space to select containers, batch actions on selection
- **Disk cleanup** — prune dangling images, unused volumes with confirmation
- **Container management** — start, stop, restart, remove, pause/unpause, exec, attach
- **Log export** — save container logs to file (`S`)
- **Open in browser** — open container's public port (`w`)
- **Screen modes** — normal/half/fullscreen toggle (`+`/`_`)
- **All-logs view** — combined logs from all containers with color-coded names (`L`)
- **Fuzzy find** — fuzzy matching sidebar filter, score-based ranking
- **Pin containers** — bookmark containers to top of list (`*`)
- **Resource alerts** — sidebar turns red when CPU/memory thresholds exceeded
- **Custom commands** — user-defined commands via config (`c`)
- **Config file** — `~/.config/rustydocker/config.toml` for customization
- **CLI flags** — repeatable `-f`, `-p`, `--context`, repeatable `--profile`, `--version`
- **Vim-style navigation** — `j/k` movement, keyboard-driven
- **Mouse support** — click containers, tabs, scroll logs

## Install

### Binary (recommended)

```bash
curl https://raw.githubusercontent.com/kennywillbe/rustydocker/master/install.sh | bash
```

Or specify a custom install directory:

```bash
DIR=/usr/local/bin curl https://raw.githubusercontent.com/kennywillbe/rustydocker/master/install.sh | bash
```

### From source

```bash
cargo install --path .
```

### Arch Linux (AUR)

```bash
yay -S rustydocker
```

## Usage

```bash
# Run in a directory with docker-compose.yml
cd my-project
rustydocker
```

Compose actions keep the same file and project scope passed at startup. For
example, `rustydocker -f compose.yml -f compose.dev.yml -p demo` applies `U`,
`D`, `R`, `P`, `B`, and `W` to that exact merged Compose project. The graph is
built from `docker compose config --format json`, so interpolation, merged
files, profiles, includes, and extensions match Docker Compose itself.

```bash
rustydocker \
  --context staging \
  -f compose.yml \
  -f compose.staging.yml \
  -p demo \
  --profile metrics
```

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j/k` | Navigate up/down |
| `h/l` | Switch focus sidebar/main |
| `Tab` | Next tab |
| `1-6` | Jump to tab (Logs/Stats/Info/Env/Top/Graph) |
| `/` | Search logs |
| `?` | Help |
| `q` | Quit |

### Container Actions

| Key | Action |
|-----|--------|
| `r` | Restart container |
| `s` | Stop container |
| `u` | Start container |
| `d` | Remove container (with confirmation) |
| `p` | Pause/unpause container |
| `e` | Exec shell (`/bin/sh`) |
| `a` | Attach to container |
| `w` | Open in browser (first public port) |
| `S` | Export logs to file |
| `P` | Pull an image (when the Images section is selected) |
| `d` | Remove selected image (with confirmation) |
| `Space` | Select/deselect container (multi-select) |
| `x` | Disk cleanup menu |
| `b` | Bulk commands menu |
| `+`/`_` | Screen mode (normal/half/fullscreen) |

### Compose Actions

| Key | Action |
|-----|--------|
| `U` | `docker compose up -d` |
| `D` | `docker compose down` (with confirmation) |
| `R` | `docker compose restart` |
| `P` | `docker compose pull` |
| `B` | `docker compose up -d --build` |
| `W` | Start/stop `docker compose watch` |

## Tabs

| Tab | Description |
|-----|-------------|
| **Logs** | Real-time log streaming with regex search highlighting |
| **Stats** | CPU, memory, network charts |
| **Info** | Container metadata, mounts, labels, health checks |
| **Env** | Environment variables table |
| **Top** | Running processes inside the container |
| **Graph** | Compose service dependency visualization |

## Sidebar

The sidebar shows four sections, each with 25% height:
- **Containers** — with status indicators and uptime
- **Images** — with tags and sizes
- **Volumes** — scrollable with `↑/↓ N more` indicators
- **Networks** — with driver info

## Docker Access

Endpoint precedence is:

1. `--context`
2. `DOCKER_CONTEXT`
3. `DOCKER_HOST`
4. `docker_context` in the config file
5. `docker_host` in the config file
6. the current Docker CLI context

The chosen endpoint is used consistently by the Engine API, Compose actions,
and interactive `exec`/`attach` commands. `unix://`, `tcp://`, `https://`, and
`ssh://` endpoints are supported. TLS connections require all three files:
`ca.pem`, `cert.pem`, and `key.pem`; startup fails clearly if the bundle is
incomplete instead of silently falling back to plaintext.

- **Linux:** Your user must be in the `docker` group, or you need to run with `sudo`.
  ```bash
  sudo usermod -aG docker $USER
  # Log out and back in for this to take effect
  ```
- **macOS:** Docker Desktop handles permissions automatically.

## Configuration

Create `~/.config/rustydocker/config.toml`:

```toml
# Timing
tick_rate_ms = 250
log_tail_lines = "100"
sidebar_width = 40

# Alerts (percentage thresholds)
cpu_alert_threshold = 80.0
memory_alert_threshold = 90.0

# Remote Docker host (optional)
# docker_host = "tcp://192.168.1.100:2376"
# docker_context = "staging"
# docker_tls_verify = true
# docker_cert_path = "/Users/me/.docker/certs/staging"
# docker_ssh_key_path = "/Users/me/.ssh/id_ed25519"

# Compose profiles (CLI --profile values override this list)
compose_profiles = ["metrics"]

# Custom commands (accessed with 'c' key)
[[custom_commands]]
name = "View logs in less"
command = "docker logs {container_id} 2>&1 | less"
attach = true

[[custom_commands]]
name = "Restart with fresh env"
command = "docker restart {container_id}"
attach = false

# Event hooks (run shell commands on Docker events)
[[hooks]]
event = "container_start"
command = "notify-send 'Container {container_name} started'"

[[hooks]]
event = "container_die"
command = "echo '{container_name} died at $(date)' >> ~/docker-events.log"
```

### Hook Events

| Event | Trigger |
|-------|---------|
| `container_start` | Container started |
| `container_stop` | Container stopped |
| `container_die` | Container exited |
| `container_restart` | Container restarted |
| `image_pull` | Image pulled |

## Remote Docker

Connect to a remote Docker daemon:

```bash
# Via environment variable
DOCKER_HOST=tcp://192.168.1.100:2376 rustydocker

# Docker context (also supports ssh:// contexts)
rustydocker --context staging

# Direct verified TLS endpoint
DOCKER_HOST=tcp://docker.example:2376 \
DOCKER_TLS_VERIFY=1 \
DOCKER_CERT_PATH="$PWD/certs" \
rustydocker

# Or via config file (see above)
```

The sidebar title shows the connected host when using a remote connection.

## Requirements

- Docker daemon running
- Rust 1.88+ (only for building from source)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

[MIT](LICENSE)
