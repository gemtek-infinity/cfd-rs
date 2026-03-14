//! File watcher and config reload contracts.
//!
//! Covers HIS-041 through HIS-045.
//!
//! The watcher is trait-based: `FileWatcher` defines the contract,
//! and the actual `notify` crate integration lives in cfdrs-bin
//! where the async runtime is available.

use std::path::{Path, PathBuf};

// --- HIS-041: watcher trait ---

/// Notification callbacks for file watcher events.
///
/// Matches Go `watcher.Notification` interface.
pub trait WatcherNotification: Send + Sync {
    /// Called when a watched file is modified (Write event).
    fn item_did_change(&self, path: &Path);

    /// Called on watcher errors.
    fn watcher_did_error(&self, error: &dyn std::error::Error);
}

/// File watcher contract matching Go `watcher.Notifier` interface.
///
/// Implementations must:
/// - Only dispatch on Write events (not Create/Remove/Rename/Chmod)
/// - Support non-blocking shutdown
/// - Close underlying resources on shutdown
pub trait FileWatcher: Send + Sync {
    /// Add a path to watch.
    fn add(&mut self, path: PathBuf) -> cfdrs_shared::Result<()>;

    /// Start the watch loop (blocking until shutdown).
    fn start(&mut self, notifier: Box<dyn WatcherNotification>);

    /// Signal shutdown (non-blocking).
    fn shutdown(&self);
}

// --- HIS-042: reload trait ---

/// Trait for the app-service reload loop.
///
/// Go: `app_service.go` `actionLoop()` receives Update/Remove operations
/// from the file watcher and applies them.
pub trait ReloadHandler: Send + Sync {
    /// Apply a config update from a changed file.
    fn on_config_update(&self, path: &Path) -> cfdrs_shared::Result<()>;

    /// Handle a config removal event.
    fn on_config_remove(&self, path: &Path) -> cfdrs_shared::Result<()>;
}

// --- HIS-043: app manager ---

/// Trait for the overwatch app manager.
///
/// Go: `overwatch/app_manager.go` `AppManager` manages starting/stopping
/// the application based on config changes.
pub trait AppManager: Send + Sync {
    /// Start the managed application.
    fn start(&self) -> cfdrs_shared::Result<()>;

    /// Stop the managed application.
    fn stop(&self) -> cfdrs_shared::Result<()>;

    /// Restart with a new config.
    fn restart(&self) -> cfdrs_shared::Result<()>;
}

// --- HIS-044: config orchestrator ---

/// Trait for the config orchestrator.
///
/// Go: `orchestration/orchestrator.go` `UpdateConfig()` atomically swaps
/// the running configuration using `atomic.Value`.
///
/// This depends on CDC contracts for remote config — the trait is defined
/// here but the full implementation is blocked by CDC work.
pub trait ConfigOrchestrator: Send + Sync {
    /// Atomically update the running configuration.
    fn update_config(&self, config: serde_json::Value) -> cfdrs_shared::Result<()>;

    /// Get the current config as JSON.
    fn get_config_json(&self) -> cfdrs_shared::Result<serde_json::Value>;
}

// --- HIS-045: reload error recovery ---

/// Error recovery strategy for reload failures.
///
/// Go behavior: if reload fails, the previous configuration continues
/// to run. The error is logged but does not crash the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadRecovery {
    /// Keep the previous configuration running.
    KeepPrevious,
    /// Shut down the application (only for fatal schema errors).
    Shutdown,
}

/// Determine the recovery strategy for a reload error.
pub fn reload_recovery_strategy(error: &cfdrs_shared::ConfigError) -> ReloadRecovery {
    use cfdrs_shared::ErrorCategory;

    match error.category() {
        // Fatal schema errors cannot be recovered — shut down.
        ErrorCategory::InvariantViolation => ReloadRecovery::Shutdown,
        // Everything else: keep the previous config.
        _ => ReloadRecovery::KeepPrevious,
    }
}

#[cfg(test)]
mod tests {
    use cfdrs_shared::ConfigError;

    use super::*;

    #[test]
    fn reload_recovery_keeps_previous_on_normal_errors() {
        let err = ConfigError::read(
            std::path::PathBuf::from("/etc/cloudflared/config.yml"),
            std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        );
        assert_eq!(reload_recovery_strategy(&err), ReloadRecovery::KeepPrevious);
    }

    #[test]
    fn reload_recovery_shuts_down_on_invariant() {
        let err = ConfigError::invariant("fatal schema error");
        assert_eq!(reload_recovery_strategy(&err), ReloadRecovery::Shutdown);
    }
}
