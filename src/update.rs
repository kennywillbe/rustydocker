//! Update check and self-update.
//!
//! This module is intentionally decoupled from the rest of the app: it
//! only communicates with the main event loop via mpsc channels and
//! plain data types. It runs blocking `self_update` calls inside
//! `tokio::task::spawn_blocking` so the UI stays responsive.

use semver::Version;

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
