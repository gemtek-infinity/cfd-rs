//! Self-update and auto-update contracts.
//!
//! Covers HIS-046 through HIS-049.
//!
//! The updater is fully deferred to the Host and Runtime Foundation
//! milestone. These types and constants capture the baseline contract.

use std::time::Duration;

// --- HIS-046: update command ---

/// Update server hostname.
pub const UPDATE_SERVER: &str = "update.argotunnel.com";

/// Trait for the update check/apply contract.
///
/// Go: `updater/update.go` implements version checking, downloading,
/// and self-replacement.
pub trait Updater: Send + Sync {
    /// Check for an available update.
    fn check(&self) -> cfdrs_shared::Result<Option<UpdateInfo>>;

    /// Apply an update (download and replace binary).
    fn apply(&self, info: &UpdateInfo) -> cfdrs_shared::Result<()>;
}

/// Information about an available update.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
    pub checksum: String,
}

// --- HIS-047: auto-updater ---

/// Default auto-update frequency.
///
/// Go: `--autoupdate-freq` default is `24h`.
pub const DEFAULT_AUTOUPDATE_FREQ: Duration = Duration::from_secs(24 * 60 * 60);

/// Trait for the auto-update timer loop.
///
/// Go: `AutoUpdater` in `updater/update.go` runs `update.Run()`
/// periodically and restarts the process on success.
pub trait AutoUpdater: Send + Sync {
    /// Start the auto-update loop. Blocks until shutdown.
    fn run(&self) -> cfdrs_shared::Result<()>;

    /// Signal shutdown.
    fn shutdown(&self);
}

// --- HIS-048: update exit codes ---

/// Exit code emitted by `cloudflared update` on successful binary replacement.
///
/// Go: `statusSuccess.ExitCode()` returns 11. The `cloudflared-update.service`
/// shell wrapper maps this to `systemctl restart cloudflared` + clean exit.
pub const UPDATE_EXIT_SUCCESS: i32 = 11;

/// Exit code emitted by `cloudflared update` on failure.
///
/// Go: `statusError.ExitCode()` returns 10.
pub const UPDATE_EXIT_FAILURE: i32 = 10;

// --- HIS-049: package manager awareness ---

/// Check if update should be skipped because binary was installed via
/// package manager.
///
/// Go: checks `BuiltForPackageManager` build flag and
/// `.installedFromPackageManager` marker file.
pub fn should_skip_update() -> bool {
    crate::environment::is_package_managed()
}

// --- Stub implementations ---

/// Stub updater for pre-alpha.
pub struct StubUpdater;

impl Updater for StubUpdater {
    fn check(&self) -> cfdrs_shared::Result<Option<UpdateInfo>> {
        Err(cfdrs_shared::ConfigError::deferred("update check"))
    }

    fn apply(&self, _info: &UpdateInfo) -> cfdrs_shared::Result<()> {
        Err(cfdrs_shared::ConfigError::deferred("update apply"))
    }
}

pub struct StubAutoUpdater;

impl AutoUpdater for StubAutoUpdater {
    fn run(&self) -> cfdrs_shared::Result<()> {
        Err(cfdrs_shared::ConfigError::deferred("auto-updater"))
    }

    fn shutdown(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_autoupdate_freq_is_24h() {
        assert_eq!(DEFAULT_AUTOUPDATE_FREQ, Duration::from_secs(86400));
    }

    #[test]
    fn stub_updater_returns_deferred() {
        let updater = StubUpdater;
        assert!(updater.check().is_err());
    }

    // --- HIS-048: exit code constants match Go baseline ---

    #[test]
    fn update_exit_success_is_11() {
        // Go: statusSuccess.ExitCode() returns 11.
        assert_eq!(UPDATE_EXIT_SUCCESS, 11);
    }

    #[test]
    fn update_exit_failure_is_10() {
        // Go: statusError.ExitCode() returns 10.
        assert_eq!(UPDATE_EXIT_FAILURE, 10);
    }

    // --- HIS-049: package manager awareness ---

    #[test]
    fn marker_path_matches_go_postinst() {
        // Go postinst.sh creates `.installedFromPackageManager` in
        // `/usr/local/etc/cloudflared/`.
        assert_eq!(
            crate::environment::INSTALLED_FROM_PACKAGE_MARKER,
            "/usr/local/etc/cloudflared/.installedFromPackageManager",
        );
    }

    #[test]
    fn should_skip_update_delegates_to_package_managed() {
        // Both functions use the same marker file check, so their
        // return values must agree.
        assert_eq!(should_skip_update(), crate::environment::is_package_managed(),);
    }

    #[test]
    fn update_server_matches_go() {
        // Go: update.argotunnel.com
        assert_eq!(UPDATE_SERVER, "update.argotunnel.com");
    }
}
