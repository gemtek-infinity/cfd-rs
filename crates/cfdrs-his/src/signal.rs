//! Signal handling and graceful shutdown.
//!
//! Covers HIS-058 through HIS-062.
//!
//! This module defines synchronous signal types and contracts. Async signal
//! listening (tokio::signal) is wired in cfdrs-bin.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use cfdrs_shared::{ConfigError, Result};

// --- HIS-059: grace period ---

/// Default grace period matching Go `--grace-period` default (30s).
pub const DEFAULT_GRACE_PERIOD: Duration = Duration::from_secs(30);

// --- HIS-058, HIS-060: shutdown signal contract ---

/// One-shot signal matching Go `signal/safe_signal.go`.
///
/// Idempotent: calling `notify()` multiple times is safe.
pub struct ShutdownSignal {
    notified: AtomicBool,
}

impl ShutdownSignal {
    pub fn new() -> Self {
        Self {
            notified: AtomicBool::new(false),
        }
    }

    /// Mark the shutdown signal as fired. Idempotent.
    pub fn notify(&self) {
        self.notified.store(true, Ordering::Release);
    }

    /// Check whether shutdown has been signalled.
    pub fn is_notified(&self) -> bool {
        self.notified.load(Ordering::Acquire)
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

// --- HIS-061: pidfile ---

/// Write a PID file. Matches Go `--pidfile` behavior.
pub fn write_pidfile(path: &Path) -> Result<()> {
    let pid = std::process::id();
    std::fs::write(path, pid.to_string()).map_err(|e| ConfigError::write_file(path, e))?;
    Ok(())
}

/// Remove a PID file if it exists.
pub fn remove_pidfile(path: &Path) {
    let _ = std::fs::remove_file(path);
}

// --- HIS-062: token lock file ---

/// Acquire a lock file for tunnel token operations.
///
/// Go: creates `<token-path>.lock` with mode 0600.
/// Uses creation as the lock mechanism (O_CREATE | O_EXCL).
pub fn acquire_token_lock(token_path: &Path) -> Result<PathBuf> {
    let lock_path = token_path.with_extension("lock");

    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .map_err(|e| {
            ConfigError::invariant(format!(
                "failed to acquire token lock at {}: {e}",
                lock_path.display()
            ))
        })?;

    Ok(lock_path)
}

/// Release a token lock file.
pub fn release_token_lock(lock_path: &Path) {
    let _ = std::fs::remove_file(lock_path);
}

// --- Signal listener trait ---

/// Trait for OS signal listeners. Wired async in cfdrs-bin.
pub trait SignalListener: Send + Sync {
    /// Wait for a termination signal (SIGTERM or SIGINT).
    /// This is intended to be called from an async context.
    fn wait_for_shutdown(&self) -> cfdrs_shared::Result<()>;
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn shutdown_signal_is_idempotent() {
        let signal = ShutdownSignal::new();
        assert!(!signal.is_notified());

        signal.notify();
        assert!(signal.is_notified());

        // Second call is safe.
        signal.notify();
        assert!(signal.is_notified());
    }

    #[test]
    fn pidfile_round_trip() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cfdrs-his-pid-{unique}.pid"));

        write_pidfile(&path).expect("write pidfile");
        let content = std::fs::read_to_string(&path).expect("read");
        let pid: u32 = content.parse().expect("parse pid");
        assert_eq!(pid, std::process::id());

        remove_pidfile(&path);
        assert!(!path.exists());
    }

    #[test]
    fn token_lock_acquire_release() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let token_path = std::env::temp_dir().join(format!("cfdrs-his-token-{unique}.json"));

        let lock_path = acquire_token_lock(&token_path).expect("acquire lock");
        assert!(lock_path.exists());

        // Second acquire should fail (file exists).
        let result = acquire_token_lock(&token_path);
        assert!(result.is_err());

        release_token_lock(&lock_path);
        assert!(!lock_path.exists());
    }

    #[test]
    fn default_grace_period_is_30s() {
        assert_eq!(DEFAULT_GRACE_PERIOD, Duration::from_secs(30));
    }
}
