//! Update check and self-update.
//!
//! This module is intentionally decoupled from the rest of the app: it
//! only communicates with the main event loop via mpsc channels and
//! plain data types. It runs blocking `self_update` calls inside
//! `tokio::task::spawn_blocking` so the UI stays responsive.

use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::UnboundedSender;

/// 6-hour cache TTL for the GitHub latest-release check.
pub const CACHE_TTL_SECS: u64 = 6 * 60 * 60;

pub const GITHUB_REPO_OWNER: &str = "kennywillbe";
pub const GITHUB_REPO_NAME: &str = "rustydocker";

/// Result of a background update check, delivered to the main loop.
#[derive(Debug, Clone)]
pub enum UpdateCheckOutcome {
    /// Newer version available. `self_updatable` tells the UI whether
    /// to offer the in-app update flow or fall back to
    /// "update via package manager".
    Available {
        version: String,
        self_updatable: bool,
    },
    /// We're on the latest stable, or the check was skipped / failed
    /// silently. Either way, no banner.
    UpToDate,
}

/// Spawn the background update check. Returns immediately. If checking
/// is disabled (config or env var), does nothing at all — no task, no
/// network, no delay. Any failure inside the task is silently swallowed
/// (results in `UpToDate`).
pub fn spawn_check(
    current_version: &'static str,
    check_enabled: bool,
    tx: UnboundedSender<UpdateCheckOutcome>,
) {
    if !check_enabled {
        return;
    }

    tokio::task::spawn_blocking(move || {
        let outcome = check_blocking(current_version);
        // Channel close = app shutting down; ignore.
        let _ = tx.send(outcome);
    });
}

fn check_blocking(current_version: &'static str) -> UpdateCheckOutcome {
    // 1. Try the cache first.
    let latest = match read_cache() {
        Some(cache) if is_cache_fresh(&cache) => cache.latest_version,
        _ => match fetch_latest_from_github() {
            Some(v) => {
                write_cache(&CachedCheck {
                    checked_at: now_secs(),
                    latest_version: v.clone(),
                });
                v
            }
            None => return UpdateCheckOutcome::UpToDate,
        },
    };

    // 2. Compare. Pre-release tags are silently ignored.
    if is_newer_stable(current_version, &latest) {
        UpdateCheckOutcome::Available {
            version: latest,
            self_updatable: is_self_updatable(),
        }
    } else {
        UpdateCheckOutcome::UpToDate
    }
}

fn fetch_latest_from_github() -> Option<String> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(GITHUB_REPO_OWNER)
        .repo_name(GITHUB_REPO_NAME)
        .build()
        .ok()?
        .fetch()
        .ok()?;
    let latest = releases.into_iter().next()?;
    // `Release::version` is already stripped of the leading `v` by
    // self_update, but trim defensively just in case.
    Some(latest.version.trim_start_matches('v').to_string())
}

/// True iff `latest` parses as a strictly-newer semver than `current`
/// AND `latest` is not a pre-release. Garbage input returns false.
pub fn is_newer_stable(current: &str, latest: &str) -> bool {
    let Ok(cur) = Version::parse(current) else {
        return false;
    };
    let Ok(new) = Version::parse(latest) else {
        return false;
    };
    new > cur && new.pre.is_empty()
}

/// Cached result of the last GitHub release check. Stored as JSON at
/// `$XDG_CACHE_HOME/rustydocker/latest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCheck {
    pub checked_at: u64, // unix seconds
    pub latest_version: String,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// True if the cache entry is still within the TTL window. Future-dated
/// timestamps (clock skew) are also treated as fresh so we don't thrash
/// the GitHub API when a user's clock is off.
pub fn is_cache_fresh(cache: &CachedCheck) -> bool {
    let now = now_secs();
    if cache.checked_at >= now {
        return true;
    }
    now - cache.checked_at < CACHE_TTL_SECS
}

fn cache_path() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("rustydocker").join("latest.json"))
}

/// Read and parse the cache file, if it exists and is valid JSON.
pub fn read_cache() -> Option<CachedCheck> {
    let path = cache_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Write the cache file, creating parent directories as needed.
/// Errors are silently swallowed — the cache is best-effort.
pub fn write_cache(cache: &CachedCheck) {
    let Some(path) = cache_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = std::fs::write(&path, json);
    }
}

/// Whether the current binary can be replaced in place by the current
/// user. Returns false for distro-packaged / system installs so AUR
/// and Homebrew users are never pushed into a broken self-update path.
pub fn is_self_updatable() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };

    // Blocklist common system prefixes — these are owned by the package
    // manager and even if they're technically writable, we shouldn't touch
    // them.
    let exe_str = exe.to_string_lossy();
    const SYSTEM_PREFIXES: &[&str] = &["/usr/", "/opt/", "/bin/", "/sbin/"];
    for prefix in SYSTEM_PREFIXES {
        if exe_str.starts_with(prefix) {
            return false;
        }
    }

    // Probe-write to the binary's parent directory to verify we can
    // actually replace the file. We don't touch the binary itself.
    let Some(parent) = exe.parent() else {
        return false;
    };
    let probe = parent.join(".rustydocker-update-probe");
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}
