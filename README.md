# rustydocker

A fast, compose-first Docker TUI written in Rust. A modern alternative to lazydocker.

[![Rust](https://img.shields.io/badge/rust-stable-orange)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Features

- **Compose-first** — services grouped by Docker Compose project
- **Real-time logs** — streaming logs with search (`/`)
- **Live stats** — CPU, memory, and network sparkline charts
- **Dependency graph** — visualize `depends_on` relationships
- **Disk cleanup** — prune dangling images, unused volumes
- **Container management** — start, stop, restart, remove, exec
- **Vim-style navigation** — `j/k` movement, keyboard-driven

## Install

### Binary (recommended)

```bash
curl https://raw.githubusercontent.com/kennywillbe/rustydocker/main/install.sh | bash
```

Or specify a custom install directory:

```bash
DIR=/usr/local/bin curl https://raw.githubusercontent.com/kennywillbe/rustydocker/main/install.sh | bash
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

## Keybindings

| Key | Action |
|-----|--------|
| `j/k` | Navigate up/down |
| `Tab` | Switch panel tab |
| `r` | Restart container |
| `s` | Stop container |
| `u` | Start container |
| `d` | Remove container |
| `e` | Exec shell |
| `/` | Search logs |
| `x` | Disk cleanup |
| `?` | Help |
| `q` | Quit |

## Docker Access

rustydocker connects to Docker via the local socket (`/var/run/docker.sock`).

- **Linux:** Your user must be in the `docker` group, or you need to run with `sudo`.
  ```bash
  sudo usermod -aG docker $USER
  # Log out and back in for this to take effect
  ```
- **macOS:** Docker Desktop handles permissions automatically.

This is the same requirement as lazydocker, docker-compose, and any other tool that talks to the Docker daemon. No root access is needed if your user is in the `docker` group.

## Requirements

- Docker daemon running
- Rust 1.75+ (only for building from source)

## License

[MIT](LICENSE)
