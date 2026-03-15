//! File watcher and config reload contracts.
//!
//! Covers HIS-041 through HIS-045.
//!
//! The watcher is trait-based: `FileWatcher` defines the contract,
//! and the actual `notify` crate integration lives in cfdrs-bin
//! where the async runtime is available.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadAction {
    Update(PathBuf),
    Remove(PathBuf),
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadLoopOutcome {
    Continue,
    Stop,
}

#[derive(Debug)]
pub struct ReloadActionReport {
    pub outcome: ReloadLoopOutcome,
    pub recovery: Option<ReloadRecovery>,
    pub error: Option<cfdrs_shared::ConfigError>,
}

pub struct ReloadActionLoop<H, A> {
    handler: H,
    app_manager: A,
}

impl<H, A> ReloadActionLoop<H, A>
where
    H: ReloadHandler,
    A: AppManager,
{
    pub fn new(handler: H, app_manager: A) -> Self {
        Self { handler, app_manager }
    }

    pub fn handle_action(&self, action: ReloadAction) -> cfdrs_shared::Result<ReloadActionReport> {
        match action {
            ReloadAction::Update(path) => match self.handler.on_config_update(&path) {
                Ok(()) => {
                    self.app_manager.restart()?;
                    Ok(ReloadActionReport {
                        outcome: ReloadLoopOutcome::Continue,
                        recovery: None,
                        error: None,
                    })
                }
                Err(error) => {
                    let recovery = reload_recovery_strategy(&error);
                    if recovery == ReloadRecovery::Shutdown {
                        self.app_manager.stop()?;
                    }

                    Ok(ReloadActionReport {
                        outcome: if recovery == ReloadRecovery::Shutdown {
                            ReloadLoopOutcome::Stop
                        } else {
                            ReloadLoopOutcome::Continue
                        },
                        recovery: Some(recovery),
                        error: Some(error),
                    })
                }
            },
            ReloadAction::Remove(path) => {
                self.handler.on_config_remove(&path)?;
                self.app_manager.stop()?;
                Ok(ReloadActionReport {
                    outcome: ReloadLoopOutcome::Continue,
                    recovery: None,
                    error: None,
                })
            }
            ReloadAction::Shutdown => {
                self.app_manager.stop()?;
                Ok(ReloadActionReport {
                    outcome: ReloadLoopOutcome::Stop,
                    recovery: None,
                    error: None,
                })
            }
        }
    }
}

pub struct InMemoryConfigOrchestrator {
    config: RwLock<serde_json::Value>,
}

impl InMemoryConfigOrchestrator {
    pub fn new(initial_config: serde_json::Value) -> Self {
        Self {
            config: RwLock::new(initial_config),
        }
    }
}

impl ConfigOrchestrator for InMemoryConfigOrchestrator {
    fn update_config(&self, config: serde_json::Value) -> cfdrs_shared::Result<()> {
        let mut current = self
            .config
            .write()
            .map_err(|_| cfdrs_shared::ConfigError::invariant("config orchestrator lock poisoned"))?;
        *current = config;
        Ok(())
    }

    fn get_config_json(&self) -> cfdrs_shared::Result<serde_json::Value> {
        let current = self
            .config
            .read()
            .map_err(|_| cfdrs_shared::ConfigError::invariant("config orchestrator lock poisoned"))?;
        Ok(current.clone())
    }
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
    use std::sync::Mutex;

    use cfdrs_shared::ConfigError;

    use super::*;

    #[derive(Default)]
    struct RecordingReloadHandler {
        updates: Mutex<Vec<PathBuf>>,
        removals: Mutex<Vec<PathBuf>>,
        update_error: Option<UpdateErrorKind>,
    }

    #[derive(Clone, Copy)]
    enum UpdateErrorKind {
        Read,
        Invariant,
    }

    impl UpdateErrorKind {
        fn build(self) -> ConfigError {
            match self {
                Self::Read => ConfigError::read(
                    PathBuf::from("/tmp/config.yml"),
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "bad yaml"),
                ),
                Self::Invariant => ConfigError::invariant("fatal"),
            }
        }
    }

    impl ReloadHandler for RecordingReloadHandler {
        fn on_config_update(&self, path: &Path) -> cfdrs_shared::Result<()> {
            if let Some(error) = self.update_error {
                return Err(error.build());
            }

            self.updates.lock().expect("lock").push(path.to_path_buf());
            Ok(())
        }

        fn on_config_remove(&self, path: &Path) -> cfdrs_shared::Result<()> {
            self.removals.lock().expect("lock").push(path.to_path_buf());
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingAppManager {
        starts: Mutex<u32>,
        stops: Mutex<u32>,
        restarts: Mutex<u32>,
    }

    impl AppManager for RecordingAppManager {
        fn start(&self) -> cfdrs_shared::Result<()> {
            *self.starts.lock().expect("lock") += 1;
            Ok(())
        }

        fn stop(&self) -> cfdrs_shared::Result<()> {
            *self.stops.lock().expect("lock") += 1;
            Ok(())
        }

        fn restart(&self) -> cfdrs_shared::Result<()> {
            *self.restarts.lock().expect("lock") += 1;
            Ok(())
        }
    }

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

    #[test]
    fn action_loop_restarts_on_successful_update() {
        let handler = RecordingReloadHandler::default();
        let app_manager = RecordingAppManager::default();
        let loop_state = ReloadActionLoop::new(handler, app_manager);

        let report = loop_state
            .handle_action(ReloadAction::Update(PathBuf::from("/tmp/config.yml")))
            .expect("update should succeed");

        assert_eq!(report.outcome, ReloadLoopOutcome::Continue);
        assert_eq!(report.recovery, None);
        assert!(report.error.is_none());
        assert_eq!(*loop_state.app_manager.restarts.lock().expect("lock"), 1);
    }

    #[test]
    fn action_loop_keeps_previous_config_on_nonfatal_update_error() {
        let handler = RecordingReloadHandler {
            update_error: Some(UpdateErrorKind::Read),
            ..RecordingReloadHandler::default()
        };
        let app_manager = RecordingAppManager::default();
        let loop_state = ReloadActionLoop::new(handler, app_manager);

        let report = loop_state
            .handle_action(ReloadAction::Update(PathBuf::from("/tmp/config.yml")))
            .expect("nonfatal update error should be reported");

        assert_eq!(report.outcome, ReloadLoopOutcome::Continue);
        assert_eq!(report.recovery, Some(ReloadRecovery::KeepPrevious));
        assert!(report.error.is_some());
        assert_eq!(*loop_state.app_manager.stops.lock().expect("lock"), 0);
    }

    #[test]
    fn action_loop_stops_on_invariant_update_error() {
        let handler = RecordingReloadHandler {
            update_error: Some(UpdateErrorKind::Invariant),
            ..RecordingReloadHandler::default()
        };
        let app_manager = RecordingAppManager::default();
        let loop_state = ReloadActionLoop::new(handler, app_manager);

        let report = loop_state
            .handle_action(ReloadAction::Update(PathBuf::from("/tmp/config.yml")))
            .expect("fatal update error should be reported");

        assert_eq!(report.outcome, ReloadLoopOutcome::Stop);
        assert_eq!(report.recovery, Some(ReloadRecovery::Shutdown));
        assert!(report.error.is_some());
        assert_eq!(*loop_state.app_manager.stops.lock().expect("lock"), 1);
    }

    #[test]
    fn action_loop_stops_on_shutdown_action() {
        let handler = RecordingReloadHandler::default();
        let app_manager = RecordingAppManager::default();
        let loop_state = ReloadActionLoop::new(handler, app_manager);

        let report = loop_state
            .handle_action(ReloadAction::Shutdown)
            .expect("shutdown should succeed");

        assert_eq!(report.outcome, ReloadLoopOutcome::Stop);
        assert_eq!(*loop_state.app_manager.stops.lock().expect("lock"), 1);
    }

    #[test]
    fn in_memory_config_orchestrator_swaps_config() {
        let orchestrator = InMemoryConfigOrchestrator::new(serde_json::json!({"version": 1}));

        orchestrator
            .update_config(serde_json::json!({"version": 2, "config": {"ingress": []}}))
            .expect("config update should succeed");

        assert_eq!(
            orchestrator.get_config_json().expect("config should load"),
            serde_json::json!({"version": 2, "config": {"ingress": []}})
        );
    }

    // --- HIS-045: reload recovery strategy ---

    #[test]
    fn reload_recovery_keeps_previous_on_io_error() {
        let err = ConfigError::read(
            PathBuf::from("/etc/cloudflared/config.yml"),
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied"),
        );
        assert_eq!(reload_recovery_strategy(&err), ReloadRecovery::KeepPrevious);
    }

    #[test]
    fn action_loop_handles_remove_and_continues() {
        let handler = RecordingReloadHandler::default();
        let app_manager = RecordingAppManager::default();
        let loop_state = ReloadActionLoop::new(handler, app_manager);

        let report = loop_state
            .handle_action(ReloadAction::Remove(PathBuf::from("/tmp/config.yml")))
            .expect("remove should succeed");

        assert_eq!(report.outcome, ReloadLoopOutcome::Continue);
        assert_eq!(loop_state.handler.removals.lock().expect("lock").len(), 1);
    }

    // --- HIS-044: in-memory config orchestrator ---

    #[test]
    fn in_memory_config_orchestrator_returns_initial_config() {
        let initial = serde_json::json!({"version": 1, "ingress": []});
        let orchestrator = InMemoryConfigOrchestrator::new(initial.clone());

        assert_eq!(orchestrator.get_config_json().expect("should load"), initial);
    }

    #[test]
    fn in_memory_config_orchestrator_preserves_latest_update() {
        let orchestrator = InMemoryConfigOrchestrator::new(serde_json::json!({"v": 1}));

        orchestrator
            .update_config(serde_json::json!({"v": 2}))
            .expect("first update");
        orchestrator
            .update_config(serde_json::json!({"v": 3}))
            .expect("second update");

        assert_eq!(
            orchestrator.get_config_json().expect("should load"),
            serde_json::json!({"v": 3})
        );
    }
}
