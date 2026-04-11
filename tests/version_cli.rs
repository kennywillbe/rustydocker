//! Integration tests for the `--version` / `--help` short-circuits.
//!
//! Regression test for a bug where `rustydocker --version` failed with
//! `ENXIO (os error 6)` because terminal setup ran before `Cli::parse()`,
//! so clap never got the chance to print the version and exit.

use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rustydocker"))
}

#[test]
fn version_flag_prints_version() {
    let output = bin()
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to spawn rustydocker");

    assert!(
        output.status.success(),
        "rustydocker --version exited non-zero: stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("rustydocker "),
        "unexpected version output: {:?}",
        stdout
    );
}

#[test]
fn help_flag_prints_usage() {
    let output = bin()
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to spawn rustydocker");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage"), "help missing Usage: {:?}", stdout);
}
