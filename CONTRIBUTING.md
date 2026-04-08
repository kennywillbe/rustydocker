# Contributing to rustydocker

Thanks for your interest in contributing! Here's how to get started.

## Development Setup

```bash
# Clone the repo
git clone https://github.com/kennywillbe/rustydocker.git
cd rustydocker

# Build
cargo build

# Run
cargo run

# Run tests
cargo test
```

### Requirements

- Rust 1.75+
- Docker daemon running
- User in `docker` group (Linux)

## How to Contribute

### Reporting Bugs

- Open an issue with a clear title and description
- Include your OS, terminal emulator, and Docker version
- Screenshots help a lot for UI issues

### Suggesting Features

- Open an issue with the `enhancement` label
- Describe the feature and why it would be useful
- Check existing issues first to avoid duplicates

### Pull Requests

1. Fork the repo
2. Create a feature branch from `master`
3. Write tests for new functionality
4. Run `cargo test` and `cargo clippy` before submitting
5. Keep PRs focused — one feature or fix per PR
6. Write a clear PR description

### Code Style

- Follow standard Rust conventions (`cargo fmt`)
- Run `cargo clippy` with no warnings
- Keep functions focused and files under ~300 lines
- Use the existing patterns in the codebase

### Architecture

```
src/
  app.rs          — App state, keybindings, actions
  config.rs       — Config file parsing
  event.rs        — Terminal event handling
  main.rs         — Main loop, Docker API calls
  docker/
    client.rs     — Docker API wrapper (bollard)
    compose.rs    — Docker Compose file parsing
    stats.rs      — Stats snapshot parsing
  ui/
    mod.rs        — Main draw dispatcher
    sidebar.rs    — Sidebar sections
    logs.rs       — Log viewer
    stats_panel.rs — Stats charts
    info.rs       — Container details
    env.rs        — Environment variables
    top.rs        — Process list
    graph.rs      — Dependency graph
    ...           — Popups and other views
```

### Testing

- Unit tests go in `tests/` directory
- Use the existing test helpers (`create_container_with_state`, etc.)
- Test key handlers return correct `AppAction` variants
- Run the full suite: `cargo test`

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
