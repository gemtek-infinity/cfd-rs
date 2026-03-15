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

/// Maximum grace period matching Go `connection.MaxGracePeriod` (3m).
pub const MAX_GRACE_PERIOD: Duration = Duration::from_secs(3 * 60);

/// Parse a Go-style duration string for `--grace-period`.
pub fn parse_grace_period(value: Option<&str>) -> Result<Duration> {
    let Some(raw_value) = value else {
        return Ok(DEFAULT_GRACE_PERIOD);
    };

    let trimmed = raw_value.trim();

    if trimmed.is_empty() {
        return Ok(DEFAULT_GRACE_PERIOD);
    }

    let duration = parse_go_duration(trimmed)?;

    if duration > MAX_GRACE_PERIOD {
        return Err(ConfigError::invariant(format!(
            "grace-period must be equal or less than {:?}",
            MAX_GRACE_PERIOD
        )));
    }

    Ok(duration)
}

fn parse_go_duration(raw: &str) -> Result<Duration> {
    if raw == "0" {
        return Ok(Duration::ZERO);
    }

    let mut total_nanos = 0f64;
    let mut rest = raw;

    while !rest.is_empty() {
        let number_end = rest
            .find(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
            .ok_or_else(|| ConfigError::invariant(format!("invalid grace-period duration `{raw}`")))?;

        let number_text = &rest[..number_end];

        if number_text.is_empty() {
            return Err(ConfigError::invariant(format!(
                "invalid grace-period duration `{raw}`"
            )));
        }

        let value = number_text
            .parse::<f64>()
            .map_err(|_| ConfigError::invariant(format!("invalid grace-period duration `{raw}`")))?;

        rest = &rest[number_end..];

        let (unit_nanos, next_rest) = parse_go_duration_unit(rest, raw)?;
        total_nanos += value * unit_nanos;
        rest = next_rest;
    }

    if !total_nanos.is_finite() || total_nanos.is_sign_negative() {
        return Err(ConfigError::invariant(format!(
            "invalid grace-period duration `{raw}`"
        )));
    }

    if total_nanos > u64::MAX as f64 {
        return Err(ConfigError::invariant(format!(
            "invalid grace-period duration `{raw}`"
        )));
    }

    Ok(Duration::from_nanos(total_nanos.round() as u64))
}

fn parse_go_duration_unit<'a>(raw: &'a str, full_value: &str) -> Result<(f64, &'a str)> {
    for (unit, nanos) in [
        ("ms", 1_000_000f64),
        ("us", 1_000f64),
        ("ns", 1f64),
        ("h", 3_600_000_000_000f64),
        ("m", 60_000_000_000f64),
        ("s", 1_000_000_000f64),
    ] {
        if let Some(next_rest) = raw.strip_prefix(unit) {
            return Ok((nanos, next_rest));
        }
    }

    Err(ConfigError::invariant(format!(
        "invalid grace-period duration `{full_value}`"
    )))
}

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
    use std::os::unix::fs::OpenOptionsExt;

    let lock_path = token_path.with_extension("lock");

    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
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
    fn token_lock_file_has_mode_0600() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let token_path = std::env::temp_dir().join(format!("cfdrs-his-token-perm-{unique}.json"));

        let lock_path = acquire_token_lock(&token_path).expect("acquire lock");
        let metadata = std::fs::metadata(&lock_path).expect("metadata");
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "lock file should have mode 0600, got {mode:o}");

        release_token_lock(&lock_path);
    }

    #[test]
    fn default_grace_period_is_30s() {
        assert_eq!(DEFAULT_GRACE_PERIOD, Duration::from_secs(30));
    }

    #[test]
    fn max_grace_period_is_three_minutes() {
        assert_eq!(MAX_GRACE_PERIOD, Duration::from_secs(180));
    }

    #[test]
    fn parse_grace_period_defaults_when_unset() {
        let duration = parse_grace_period(None).expect("default grace period should parse");
        assert_eq!(duration, Duration::from_secs(30));
    }

    #[test]
    fn parse_grace_period_accepts_go_style_sequences() {
        let duration = parse_grace_period(Some("1m30s")).expect("sequence duration should parse");
        assert_eq!(duration, Duration::from_secs(90));

        let milliseconds = parse_grace_period(Some("250ms")).expect("millisecond duration should parse");
        assert_eq!(milliseconds, Duration::from_millis(250));
    }

    #[test]
    fn parse_grace_period_rejects_values_above_max() {
        let error = parse_grace_period(Some("181s")).expect_err("duration above max should fail");
        assert!(
            error
                .to_string()
                .contains("grace-period must be equal or less than")
        );
    }

    // --- HIS-059: additional grace-period parity tests ---

    #[test]
    fn parse_grace_period_empty_string_returns_default() {
        let duration = parse_grace_period(Some("")).expect("empty should return default");
        assert_eq!(duration, DEFAULT_GRACE_PERIOD);
    }

    #[test]
    fn parse_grace_period_whitespace_returns_default() {
        let duration = parse_grace_period(Some("   ")).expect("whitespace should return default");
        assert_eq!(duration, DEFAULT_GRACE_PERIOD);
    }

    #[test]
    fn parse_grace_period_accepts_pure_seconds() {
        let duration = parse_grace_period(Some("45s")).expect("45s should parse");
        assert_eq!(duration, Duration::from_secs(45));
    }

    #[test]
    fn parse_grace_period_accepts_zero() {
        let duration = parse_grace_period(Some("0")).expect("zero should parse");
        assert_eq!(duration, Duration::ZERO);
    }

    #[test]
    fn parse_grace_period_boundary_at_max() {
        // Exactly 3m (180s) should be accepted.
        let duration = parse_grace_period(Some("3m")).expect("3m should parse");
        assert_eq!(duration, Duration::from_secs(180));
    }

    #[test]
    fn parse_grace_period_rejects_invalid_unit() {
        assert!(parse_grace_period(Some("10x")).is_err());
    }

    #[test]
    fn parse_grace_period_rejects_bare_number() {
        assert!(parse_grace_period(Some("30")).is_err());
    }

    #[test]
    fn parse_grace_period_accepts_hours() {
        let duration = parse_grace_period(Some("0h1m")).expect("0h1m should parse");
        assert_eq!(duration, Duration::from_secs(60));
    }
}
