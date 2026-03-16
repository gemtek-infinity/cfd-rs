//! File watcher and config reload contracts.
//!
//! Covers HIS-041 through HIS-045.
//!
//! `FileWatcher` defines the contract. `NotifyFileWatcher` provides
//! the concrete `notify`-backed implementation matching Go
//! `watcher/file.go` behavior: Write-only filter, delegate errors
//! to caller, non-blocking shutdown.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

// --- HIS-041: watcher trait ---

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
    ///
    /// `on_change` is called when a watched file is modified (Write event).
    /// `on_error` is called on watcher errors.
    fn start(
        &mut self,
        on_change: impl Fn(&Path) + Send + 'static,
        on_error: impl Fn(&dyn std::error::Error) + Send + 'static,
    );

    /// Signal shutdown (non-blocking).
    fn shutdown(&self);
}

// --- HIS-041 concrete implementation ---

/// Concrete file watcher backed by the `notify` crate (inotify on Linux).
///
/// Matches Go `watcher.File` behavior:
/// - Only Write events are forwarded to the notifier
/// - Errors are delegated, never logged internally
/// - Shutdown is non-blocking and idempotent
pub struct NotifyFileWatcher {
    watcher: Mutex<notify::RecommendedWatcher>,
    event_rx: Mutex<std::sync::mpsc::Receiver<Result<notify::Event, notify::Error>>>,
    shutdown: Arc<AtomicBool>,
}

impl NotifyFileWatcher {
    pub fn new() -> cfdrs_shared::Result<Self> {
        let (tx, rx) = std::sync::mpsc::channel();
        let watcher = {
            use notify::Watcher;
            notify::RecommendedWatcher::new(
                move |result| {
                    let _ = tx.send(result);
                },
                notify::Config::default(),
            )
            .map_err(|error| {
                cfdrs_shared::ConfigError::invariant(format!("file watcher creation failed: {error}"))
            })?
        };

        Ok(Self {
            watcher: Mutex::new(watcher),
            event_rx: Mutex::new(rx),
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Return a cloned shutdown flag for external shutdown coordination.
    ///
    /// The caller can hold this handle and call `store(true, Relaxed)` to
    /// signal shutdown even after the watcher has been moved into a
    /// blocking thread.
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }
}

impl FileWatcher for NotifyFileWatcher {
    fn add(&mut self, path: PathBuf) -> cfdrs_shared::Result<()> {
        use notify::Watcher;
        self.watcher
            .lock()
            .map_err(|_| cfdrs_shared::ConfigError::invariant("watcher lock poisoned"))?
            .watch(&path, notify::RecursiveMode::NonRecursive)
            .map_err(|error| cfdrs_shared::ConfigError::invariant(format!("file watch add failed: {error}")))
    }

    fn start(
        &mut self,
        on_change: impl Fn(&Path) + Send + 'static,
        on_error: impl Fn(&dyn std::error::Error) + Send + 'static,
    ) {
        let shutdown = Arc::clone(&self.shutdown);

        let event_rx = match self.event_rx.lock() {
            Ok(rx) => rx,
            Err(_) => return,
        };

        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            match event_rx.recv_timeout(std::time::Duration::from_millis(50)) {
                Ok(Ok(event)) => {
                    if event.kind.is_modify() {
                        for path in &event.paths {
                            on_change(path);
                        }
                    }
                }
                Ok(Err(error)) => {
                    on_error(&error);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn shutdown(&self) {
        // Atomic store: if start() already returned, this is harmless.
        // (matches Go's select { case f.shutdown <- struct{}{}: default: })
        self.shutdown.store(true, Ordering::Relaxed);
    }
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

/// Response from a versioned config update.
///
/// Go: `pogs.UpdateConfigurationResponse { LastAppliedVersion, Err }`.
#[derive(Debug, Clone)]
pub struct UpdateConfigResponse {
    /// The version that is currently applied after the update attempt.
    pub last_applied_version: i32,
    /// Non-fatal error description (e.g. parse failure). `None` on success
    /// or when the update was rejected due to version ordering.
    pub error: Option<String>,
}

/// Trait for the config orchestrator.
///
/// Go: `orchestration/orchestrator.go` `UpdateConfig()` atomically swaps
/// the running configuration using `atomic.Value` with version monotonicity.
///
/// This depends on CDC contracts for remote config — the trait is defined
/// here but the full implementation is blocked by CDC work.
pub trait ConfigOrchestrator: Send + Sync {
    /// Atomically update the running configuration.
    ///
    /// Rejects the update when `version <= current_version` (monotonic).
    /// Go initial version is `-1`, so version `0` is the first valid update.
    fn update_config(&self, version: i32, config: serde_json::Value) -> UpdateConfigResponse;

    /// Get the current config as JSON.
    fn get_config_json(&self) -> cfdrs_shared::Result<serde_json::Value>;

    /// Current applied version (`-1` initially, matching Go).
    fn current_version(&self) -> i32;
}

// --- HIS-043: overwatch service management ---

/// Service contract matching Go `overwatch.Service` interface.
///
/// Services are identified by name. Change detection uses content-hash
/// comparison — when a new service has the same hash as the existing one,
/// the manager skips the replacement.
pub trait ManagedService: Send + Sync {
    /// Unique service name used as the map key.
    fn name(&self) -> &str;

    /// Service category (e.g. `"forward"`, `"tunnel"`).
    fn service_type(&self) -> &str;

    /// Content hash for change detection (borrowed, zero-alloc).
    ///
    /// Go: `AppManager.Add()` compares `current.Hash() == service.Hash()`
    /// and skips the add when they match.
    fn hash(&self) -> &str;

    /// Graceful shutdown (consuming — the service cannot be reused).
    fn shutdown(self);
}

/// Outcome of adding a service to the manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddOutcome {
    /// New service added (no previous entry with this name).
    Added,
    /// Old service with different hash was shut down and replaced.
    Replaced,
    /// Existing service has the same hash — no action taken.
    Unchanged,
}

/// Service lifecycle manager matching Go `overwatch/app_manager.go`.
///
/// Key behavior: `add()` compares content hashes. If the hash matches
/// the existing service under the same name, the add is a no-op.
/// If the hash differs, the old service is shut down before replacement.
pub struct ServiceManager<S> {
    services: HashMap<String, S>,
}

impl<S: ManagedService> Default for ServiceManager<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: ManagedService> ServiceManager<S> {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Add or replace a service.
    ///
    /// Matching Go `AppManager.Add()`:
    /// - same name + same hash → skip (return `Unchanged`)
    /// - same name + different hash → shutdown old, store new (return
    ///   `Replaced`)
    /// - new name → store (return `Added`)
    pub fn add(&mut self, service: S) -> AddOutcome {
        let name = service.name().to_owned();

        if let Some(current) = self.services.get(&name) {
            if current.hash() == service.hash() {
                return AddOutcome::Unchanged;
            }

            // Remove first to get the owned value, then shut it down.
            let old = self.services.remove(&name).expect("entry confirmed present");
            old.shutdown();
            self.services.insert(name, service);
            AddOutcome::Replaced
        } else {
            self.services.insert(name, service);
            AddOutcome::Added
        }
    }

    /// Remove and shut down a service by name.
    ///
    /// Matching Go `AppManager.Remove()`.
    pub fn remove(&mut self, name: &str) {
        if let Some(service) = self.services.remove(name) {
            service.shutdown();
        }
    }

    /// Iterate over all current services.
    ///
    /// Matching Go `AppManager.Services()`.
    pub fn services(&self) -> impl Iterator<Item = &S> {
        self.services.values()
    }
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

    /// Run the action loop, processing actions from the receiver until stop.
    ///
    /// Matches Go `app_service.go` `actionLoop()` — receives config-update
    /// or shutdown actions via a channel and dispatches them with error
    /// recovery.
    pub fn run(&self, actions: std::sync::mpsc::Receiver<ReloadAction>) -> Vec<ReloadActionReport> {
        let mut reports = Vec::new();

        for action in actions {
            match self.handle_action(action) {
                Ok(report) => {
                    let should_stop = report.outcome == ReloadLoopOutcome::Stop;
                    reports.push(report);

                    if should_stop {
                        break;
                    }
                }
                Err(error) => {
                    reports.push(ReloadActionReport {
                        outcome: ReloadLoopOutcome::Stop,
                        recovery: Some(ReloadRecovery::Shutdown),
                        error: Some(error),
                    });
                    break;
                }
            }
        }

        reports
    }
}

pub struct InMemoryConfigOrchestrator {
    version: RwLock<i32>,
    config: RwLock<serde_json::Value>,
}

impl InMemoryConfigOrchestrator {
    /// Initial version is `-1`, matching Go `Orchestrator.currentVersion`.
    /// Version `0` is the first valid config-migration update.
    pub fn new(initial_config: serde_json::Value) -> Self {
        Self {
            version: RwLock::new(-1),
            config: RwLock::new(initial_config),
        }
    }
}

impl ConfigOrchestrator for InMemoryConfigOrchestrator {
    fn update_config(&self, version: i32, config: serde_json::Value) -> UpdateConfigResponse {
        let mut current_version = match self.version.write() {
            Ok(v) => v,
            Err(_) => {
                return UpdateConfigResponse {
                    last_applied_version: -1,
                    error: Some("config orchestrator version lock poisoned".to_owned()),
                };
            }
        };

        // Go: `if o.currentVersion >= version { return }` — monotonic.
        if *current_version >= version {
            return UpdateConfigResponse {
                last_applied_version: *current_version,
                error: None,
            };
        }

        match self.config.write() {
            Ok(mut current) => {
                *current = config;
                *current_version = version;
                UpdateConfigResponse {
                    last_applied_version: *current_version,
                    error: None,
                }
            }
            Err(_) => UpdateConfigResponse {
                last_applied_version: *current_version,
                error: Some("config orchestrator config lock poisoned".to_owned()),
            },
        }
    }

    fn get_config_json(&self) -> cfdrs_shared::Result<serde_json::Value> {
        let current = self
            .config
            .read()
            .map_err(|_| cfdrs_shared::ConfigError::invariant("config orchestrator lock poisoned"))?;
        Ok(current.clone())
    }

    fn current_version(&self) -> i32 {
        self.version.read().map(|v| *v).unwrap_or(-1)
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
    use std::sync::{Arc, Mutex};

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

        let response =
            orchestrator.update_config(0, serde_json::json!({"version": 2, "config": {"ingress": []}}));
        assert!(response.error.is_none());

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

        let r1 = orchestrator.update_config(0, serde_json::json!({"v": 2}));
        assert!(r1.error.is_none());

        let r2 = orchestrator.update_config(1, serde_json::json!({"v": 3}));
        assert!(r2.error.is_none());

        assert_eq!(
            orchestrator.get_config_json().expect("should load"),
            serde_json::json!({"v": 3})
        );
    }

    // --- HIS-041: NotifyFileWatcher ---

    #[test]
    fn notify_file_watcher_detects_write_event() {
        use std::io::Write;

        let dir = tempfile::tempdir().expect("tempdir");
        let file_path = dir.path().join("config.yml");
        std::fs::write(&file_path, "v: 1").expect("initial write");

        let mut watcher = NotifyFileWatcher::new().expect("watcher");
        watcher.add(file_path.clone()).expect("watch add");

        let changed: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
        let changed_clone = Arc::clone(&changed);

        let watcher_handle = std::thread::spawn(move || {
            watcher.start(
                move |path| {
                    changed_clone.lock().expect("lock").push(path.to_path_buf());
                },
                |_error| {},
            );
        });

        // Give the watcher time to start
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Trigger a write event
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&file_path)
            .expect("open for write");
        f.write_all(b"v: 2").expect("write");
        f.flush().expect("flush");
        drop(f);

        // Wait for the event to propagate
        std::thread::sleep(std::time::Duration::from_millis(200));

        let paths = changed.lock().expect("lock");
        assert!(
            !paths.is_empty(),
            "should have received at least one write notification"
        );

        // Watcher thread is still blocking; we need to drop it
        // (it will exit when the test ends and the channels drop)
        drop(watcher_handle);
    }

    #[test]
    fn notify_file_watcher_shutdown_is_nonblocking() {
        let watcher = NotifyFileWatcher::new().expect("watcher");
        // Calling shutdown before start should not block or panic
        watcher.shutdown();
        watcher.shutdown(); // idempotent
    }

    // --- HIS-042: channel-driven reload action loop ---

    #[test]
    fn action_loop_run_processes_multiple_actions() {
        let handler = RecordingReloadHandler::default();
        let app_manager = RecordingAppManager::default();
        let loop_state = ReloadActionLoop::new(handler, app_manager);

        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(ReloadAction::Update(PathBuf::from("/tmp/a.yml")))
            .expect("send");
        tx.send(ReloadAction::Update(PathBuf::from("/tmp/b.yml")))
            .expect("send");
        tx.send(ReloadAction::Shutdown).expect("send");

        let reports = loop_state.run(rx);

        assert_eq!(reports.len(), 3);
        assert_eq!(reports[0].outcome, ReloadLoopOutcome::Continue);
        assert_eq!(reports[1].outcome, ReloadLoopOutcome::Continue);
        assert_eq!(reports[2].outcome, ReloadLoopOutcome::Stop);
        assert_eq!(*loop_state.app_manager.restarts.lock().expect("lock"), 2);
        assert_eq!(*loop_state.app_manager.stops.lock().expect("lock"), 1);
    }

    #[test]
    fn action_loop_run_continues_on_nonfatal_error() {
        let handler = RecordingReloadHandler {
            update_error: Some(UpdateErrorKind::Read),
            ..RecordingReloadHandler::default()
        };
        let app_manager = RecordingAppManager::default();
        let loop_state = ReloadActionLoop::new(handler, app_manager);

        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(ReloadAction::Update(PathBuf::from("/tmp/config.yml")))
            .expect("send");
        tx.send(ReloadAction::Shutdown).expect("send");

        let reports = loop_state.run(rx);

        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].outcome, ReloadLoopOutcome::Continue);
        assert_eq!(reports[0].recovery, Some(ReloadRecovery::KeepPrevious));
        assert_eq!(reports[1].outcome, ReloadLoopOutcome::Stop);
    }

    #[test]
    fn action_loop_run_stops_early_on_invariant_error() {
        let handler = RecordingReloadHandler {
            update_error: Some(UpdateErrorKind::Invariant),
            ..RecordingReloadHandler::default()
        };
        let app_manager = RecordingAppManager::default();
        let loop_state = ReloadActionLoop::new(handler, app_manager);

        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(ReloadAction::Update(PathBuf::from("/tmp/config.yml")))
            .expect("send");
        // This second action should never be reached.
        tx.send(ReloadAction::Update(PathBuf::from("/tmp/second.yml")))
            .expect("send");

        let reports = loop_state.run(rx);

        assert_eq!(reports.len(), 1, "should stop after first fatal error");
        assert_eq!(reports[0].outcome, ReloadLoopOutcome::Stop);
        assert_eq!(reports[0].recovery, Some(ReloadRecovery::Shutdown));
    }

    #[test]
    fn action_loop_run_exits_on_channel_close() {
        let handler = RecordingReloadHandler::default();
        let app_manager = RecordingAppManager::default();
        let loop_state = ReloadActionLoop::new(handler, app_manager);

        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(ReloadAction::Update(PathBuf::from("/tmp/a.yml")))
            .expect("send");
        drop(tx);

        let reports = loop_state.run(rx);

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].outcome, ReloadLoopOutcome::Continue);
    }

    // --- HIS-043: overwatch service manager ---

    struct TestService {
        svc_name: String,
        svc_type: String,
        svc_hash: String,
        shutdown_count: Arc<Mutex<u32>>,
    }

    impl TestService {
        fn new(name: &str, hash: &str) -> Self {
            Self {
                svc_name: name.to_owned(),
                svc_type: "test".to_owned(),
                svc_hash: hash.to_owned(),
                shutdown_count: Arc::new(Mutex::new(0)),
            }
        }

        fn with_shutdown_counter(name: &str, hash: &str, counter: Arc<Mutex<u32>>) -> Self {
            Self {
                svc_name: name.to_owned(),
                svc_type: "test".to_owned(),
                svc_hash: hash.to_owned(),
                shutdown_count: counter,
            }
        }
    }

    impl ManagedService for TestService {
        fn name(&self) -> &str {
            &self.svc_name
        }

        fn service_type(&self) -> &str {
            &self.svc_type
        }

        fn hash(&self) -> &str {
            &self.svc_hash
        }

        fn shutdown(self) {
            *self.shutdown_count.lock().expect("lock") += 1;
        }
    }

    #[test]
    fn service_manager_adds_new_service() {
        let mut mgr = ServiceManager::new();
        let outcome = mgr.add(TestService::new("web", "abc123"));

        assert_eq!(outcome, AddOutcome::Added);
        assert_eq!(mgr.services().count(), 1);
    }

    #[test]
    fn service_manager_skips_same_hash() {
        let mut mgr = ServiceManager::new();
        mgr.add(TestService::new("web", "abc123"));

        let outcome = mgr.add(TestService::new("web", "abc123"));

        assert_eq!(outcome, AddOutcome::Unchanged);
        assert_eq!(mgr.services().count(), 1);
    }

    #[test]
    fn service_manager_replaces_on_hash_change() {
        let mut mgr = ServiceManager::new();
        let shutdown_counter = Arc::new(Mutex::new(0));
        mgr.add(TestService::with_shutdown_counter(
            "web",
            "v1",
            Arc::clone(&shutdown_counter),
        ));

        let outcome = mgr.add(TestService::new("web", "v2"));

        assert_eq!(outcome, AddOutcome::Replaced);
        assert_eq!(
            *shutdown_counter.lock().expect("lock"),
            1,
            "old service should be shut down"
        );
        assert_eq!(mgr.services().count(), 1);
    }

    #[test]
    fn service_manager_removes_and_shuts_down() {
        let mut mgr = ServiceManager::new();
        let shutdown_counter = Arc::new(Mutex::new(0));
        mgr.add(TestService::with_shutdown_counter(
            "web",
            "v1",
            Arc::clone(&shutdown_counter),
        ));

        mgr.remove("web");

        assert_eq!(*shutdown_counter.lock().expect("lock"), 1);
        assert_eq!(mgr.services().count(), 0);
    }

    #[test]
    fn service_manager_remove_nonexistent_is_noop() {
        let mut mgr: ServiceManager<TestService> = ServiceManager::new();
        mgr.remove("nonexistent"); // should not panic
    }

    #[test]
    fn service_manager_manages_multiple_services() {
        let mut mgr = ServiceManager::new();
        mgr.add(TestService::new("web", "v1"));
        mgr.add(TestService::new("api", "v1"));
        mgr.add(TestService::new("worker", "v1"));

        assert_eq!(mgr.services().count(), 3);

        mgr.remove("api");

        assert_eq!(mgr.services().count(), 2);
    }

    // --- HIS-044: versioned config orchestrator ---

    #[test]
    fn versioned_config_update_applies_higher_version() {
        let orchestrator = InMemoryConfigOrchestrator::new(serde_json::json!({"v": 0}));

        let response = orchestrator.update_config(0, serde_json::json!({"v": 1}));

        assert_eq!(response.last_applied_version, 0);
        assert!(response.error.is_none());
        assert_eq!(
            orchestrator.get_config_json().expect("should load"),
            serde_json::json!({"v": 1})
        );
    }

    #[test]
    fn versioned_config_rejects_same_version() {
        let orchestrator = InMemoryConfigOrchestrator::new(serde_json::json!({"v": 0}));

        orchestrator.update_config(1, serde_json::json!({"v": 1}));

        let response = orchestrator.update_config(1, serde_json::json!({"v": 2}));

        assert_eq!(response.last_applied_version, 1);
        assert_eq!(
            orchestrator.get_config_json().expect("should load"),
            serde_json::json!({"v": 1}),
            "config should not change on same version"
        );
    }

    #[test]
    fn versioned_config_rejects_lower_version() {
        let orchestrator = InMemoryConfigOrchestrator::new(serde_json::json!({"v": 0}));

        orchestrator.update_config(5, serde_json::json!({"v": 5}));

        let response = orchestrator.update_config(3, serde_json::json!({"v": 3}));

        assert_eq!(response.last_applied_version, 5);
        assert_eq!(
            orchestrator.get_config_json().expect("should load"),
            serde_json::json!({"v": 5}),
            "config should not downgrade"
        );
    }

    #[test]
    fn versioned_config_starts_at_version_negative_one() {
        let orchestrator = InMemoryConfigOrchestrator::new(serde_json::json!({}));
        assert_eq!(orchestrator.current_version(), -1);
    }

    #[test]
    fn versioned_config_version_zero_succeeds_from_initial() {
        let orchestrator = InMemoryConfigOrchestrator::new(serde_json::json!({}));

        let response = orchestrator.update_config(0, serde_json::json!({"migrated": true}));

        assert_eq!(response.last_applied_version, 0);
        assert_eq!(orchestrator.current_version(), 0);
    }
}
