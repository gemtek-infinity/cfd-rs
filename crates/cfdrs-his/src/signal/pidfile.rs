//! Pidfile and connected signal (HIS-061).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use cfdrs_shared::{ConfigError, Result};

/// One-shot connected signal matching Go `signal/safe_signal.go`.
///
/// Go creates a `signal.Signal` wrapping a `chan struct{}` that fires once
/// via `sync.Once`. Both `writePidFile` and `notifySystemd` block on it,
/// executing their side-effects exactly once when the first tunnel
/// connection is established.
///
/// This type replicates the one-shot contract: `notify()` fires exactly
/// once (subsequent calls are no-ops via `call_once`), and `is_notified()`
/// provides synchronous observability.
pub struct ConnectedSignal {
    fired: std::sync::Once,
    notified: AtomicBool,
}

impl ConnectedSignal {
    pub fn new() -> Self {
        Self {
            fired: std::sync::Once::new(),
            notified: AtomicBool::new(false),
        }
    }

    /// Fire the signal. Only the first call executes; subsequent calls
    /// are no-ops, matching Go `signal.Notify()` via `sync.Once`.
    pub fn notify(&self) {
        self.fired.call_once(|| {
            self.notified.store(true, Ordering::Release);
        });
    }

    /// Check whether the signal has fired.
    pub fn is_notified(&self) -> bool {
        self.notified.load(Ordering::Acquire)
    }
}

impl Default for ConnectedSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// Expand a leading `~/` prefix to the user's home directory.
///
/// Matches Go `homedir.Expand` used in `writePidFile`.
fn expand_pidfile_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();

    if path_str == "~"
        && let Some(home) = home_directory()
    {
        return home;
    }

    if let Some(remainder) = path_str.strip_prefix("~/")
        && let Some(home) = home_directory()
    {
        return home.join(remainder);
    }

    path.to_path_buf()
}

/// Resolve the user's home directory from the `HOME` environment variable.
fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Write a PID file after expanding `~` paths.
///
/// Matches Go `writePidFile`: expands `homedir.Expand(pidPathname)`,
/// then writes `fmt.Fprintf(file, "%d", os.Getpid())`.
pub fn write_pidfile(path: &Path) -> Result<()> {
    let expanded = expand_pidfile_path(path);
    let pid = std::process::id();
    std::fs::write(&expanded, pid.to_string()).map_err(|e| ConfigError::write_file(&expanded, e))?;
    Ok(())
}

/// Remove a PID file if it exists.
pub fn remove_pidfile(path: &Path) {
    let expanded = expand_pidfile_path(path);
    let _ = std::fs::remove_file(expanded);
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn connected_signal_fires_once() {
        let signal = ConnectedSignal::new();
        assert!(!signal.is_notified());

        signal.notify();
        assert!(signal.is_notified());
    }

    #[test]
    fn connected_signal_multiple_notify_is_idempotent() {
        let signal = ConnectedSignal::new();

        signal.notify();
        signal.notify();
        signal.notify();
        assert!(signal.is_notified());
    }

    #[test]
    fn connected_signal_default_is_not_notified() {
        let signal = ConnectedSignal::default();
        assert!(!signal.is_notified());
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
    fn pidfile_write_format_is_decimal_pid_only() {
        // Go: fmt.Fprintf(file, "%d", os.Getpid()) — decimal, no newline.
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cfdrs-his-pid-fmt-{unique}.pid"));

        write_pidfile(&path).expect("write pidfile");
        let content = std::fs::read_to_string(&path).expect("read content");

        assert!(
            !content.ends_with('\n'),
            "pidfile should not have trailing newline, got: {content:?}"
        );
        let _pid: u32 = content
            .parse()
            .expect("pidfile content should be a pure decimal integer");

        remove_pidfile(&path);
    }

    #[test]
    fn pidfile_tilde_expansion() {
        // Expand ~/foo -> $HOME/foo, matching Go homedir.Expand.
        let path = Path::new("~/test-pidfile-expansion.pid");
        let expanded = expand_pidfile_path(path);

        if let Some(home) = home_directory() {
            assert_eq!(
                expanded,
                home.join("test-pidfile-expansion.pid"),
                "~/path should expand to $HOME/path"
            );
        } else {
            assert_eq!(
                expanded,
                path.to_path_buf(),
                "without HOME, path should be returned as-is"
            );
        }
    }

    #[test]
    fn pidfile_bare_tilde_expansion() {
        let path = Path::new("~");
        let expanded = expand_pidfile_path(path);

        if let Some(home) = home_directory() {
            assert_eq!(expanded, home, "bare ~ should expand to $HOME");
        } else {
            assert_eq!(expanded, path.to_path_buf());
        }
    }

    #[test]
    fn pidfile_no_expansion_for_absolute_path() {
        let path = Path::new("/tmp/test.pid");
        let expanded = expand_pidfile_path(path);
        assert_eq!(
            expanded,
            path.to_path_buf(),
            "absolute paths should not be modified"
        );
    }

    #[test]
    fn pidfile_no_expansion_for_relative_path() {
        let path = Path::new("relative/test.pid");
        let expanded = expand_pidfile_path(path);
        assert_eq!(
            expanded,
            path.to_path_buf(),
            "relative paths without ~ should not be modified"
        );
    }
}
