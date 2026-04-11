//! Unit tests for the pure helpers in `src/update.rs`.
//! Network-touching code (`spawn_check`, `run_self_update`) is not tested
//! here — that runs against the real GitHub API and is validated manually.

use rustydocker::update::{is_cache_fresh, is_newer_stable, CachedCheck};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

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

#[test]
fn cache_round_trips_through_json() {
    let original = CachedCheck {
        checked_at: 1_700_000_000,
        latest_version: "0.3.1".to_string(),
    };
    let json = serde_json::to_string(&original).unwrap();
    let decoded: CachedCheck = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.checked_at, 1_700_000_000);
    assert_eq!(decoded.latest_version, "0.3.1");
}

#[test]
fn cache_is_fresh_when_within_ttl() {
    let fresh = CachedCheck {
        checked_at: now_secs() - (4 * 60 * 60), // 4h ago
        latest_version: "0.3.1".to_string(),
    };
    assert!(is_cache_fresh(&fresh));
}

#[test]
fn cache_is_stale_when_past_ttl() {
    let stale = CachedCheck {
        checked_at: now_secs() - (7 * 60 * 60), // 7h ago
        latest_version: "0.3.1".to_string(),
    };
    assert!(!is_cache_fresh(&stale));
}

#[test]
fn cache_with_future_timestamp_is_treated_as_fresh() {
    // Clock skew shouldn't cause thrashing.
    let skewed = CachedCheck {
        checked_at: now_secs() + 3600,
        latest_version: "0.3.1".to_string(),
    };
    assert!(is_cache_fresh(&skewed));
}
