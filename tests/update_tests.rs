//! Unit tests for the pure helpers in `src/update.rs`.
//! Network-touching code (`spawn_check`, `run_self_update`) is not tested
//! here — that runs against the real GitHub API and is validated manually.

use rustydocker::update::is_newer_stable;

#[test]
fn newer_patch_is_newer() {
    assert!(is_newer_stable("0.3.0", "0.3.1"));
}

#[test]
fn same_version_is_not_newer() {
    assert!(!is_newer_stable("0.3.0", "0.3.0"));
}

#[test]
fn older_is_not_newer() {
    assert!(!is_newer_stable("0.3.0", "0.2.9"));
}

#[test]
fn prerelease_latest_is_suppressed() {
    // We never nag mainline users about pre-releases.
    assert!(!is_newer_stable("0.3.0", "0.3.1-rc1"));
    assert!(!is_newer_stable("0.3.0", "0.4.0-beta.2"));
}

#[test]
fn prerelease_current_to_stable_latest_is_newer() {
    // Inverse: if the user is on a pre-release and a stable drops,
    // semver ordering makes stable > pre and we do prompt.
    assert!(is_newer_stable("0.3.0-rc1", "0.3.0"));
}

#[test]
fn garbage_input_returns_false() {
    assert!(!is_newer_stable("not-a-version", "0.3.1"));
    assert!(!is_newer_stable("0.3.0", "garbage"));
}
