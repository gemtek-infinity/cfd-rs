//! Signal handling and graceful shutdown.
//!
//! Covers HIS-058 through HIS-062. Split into focused submodules:
//!
//! - `grace_period`: Go-style duration parsing for `--grace-period` (HIS-059)
//! - `pidfile`: `ConnectedSignal` one-shot and pidfile helpers (HIS-061)
//! - `token_lock`: Token lock with exponential backoff (HIS-062)

use std::sync::atomic::{AtomicBool, Ordering};

mod grace_period;
mod pidfile;
mod token_lock;

pub use grace_period::{DEFAULT_GRACE_PERIOD, MAX_GRACE_PERIOD, parse_grace_period};
pub use pidfile::{ConnectedSignal, remove_pidfile, write_pidfile};
pub use token_lock::{TokenLock, acquire_token_lock, release_token_lock};

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

// --- Signal listener trait ---

/// Trait for OS signal listeners. Wired async in cfdrs-bin.
pub trait SignalListener: Send + Sync {
    /// Wait for a termination signal (SIGTERM or SIGINT).
    /// This is intended to be called from an async context.
    fn wait_for_shutdown(&self) -> cfdrs_shared::Result<()>;
}

#[cfg(test)]
mod tests {
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
}
