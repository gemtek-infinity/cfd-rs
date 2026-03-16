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

/// Maximum backoff retry iterations, matching Go `retry.NewBackoff(uint(7),
/// ...)`.
const TOKEN_LOCK_MAX_RETRIES: u32 = 7;

/// Base backoff duration, matching Go `retry.DefaultBaseTime` (1 second).
const TOKEN_LOCK_BASE_BACKOFF: Duration = Duration::from_secs(1);

/// Token lock matching Go `token.lock` struct.
///
/// Wraps a lock file path with exponential backoff retry and signal cleanup.
/// On `acquire()`, if the lock file already exists, backs off exponentially
/// up to `TOKEN_LOCK_MAX_RETRIES` iterations. After exhaustion, deletes the
/// stale lock and retries creation. Registers a SIGINT/SIGTERM handler that
/// cleans up the lock file on unexpected exit.
pub struct TokenLock {
    lock_path: PathBuf,
    acquired: AtomicBool,
}

impl TokenLock {
    /// Create a new token lock for the given token path.
    ///
    /// The lock file will be `<token_path>.lock`.
    pub fn new(token_path: &Path) -> Self {
        Self {
            lock_path: token_path.with_extension("lock"),
            acquired: AtomicBool::new(false),
        }
    }

    /// Path of the lock file.
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }

    /// Acquire the token lock with exponential backoff.
    ///
    /// Go behavior: poll up to 7 times with exponential backoff (base 1s,
    /// doubling per iteration). If all retries are exhausted and the lock
    /// file still exists, delete it as stale and proceed. Creates the lock
    /// file with mode 0600 via `O_CREATE | O_EXCL`.
    pub fn acquire(&self) -> Result<()> {
        self.acquire_with_sleep(std::thread::sleep)
    }

    /// Acquire with an injectable sleep function (for testing).
    fn acquire_with_sleep(&self, sleep_fn: impl Fn(Duration)) -> Result<()> {
        // Poll with exponential backoff if lock exists.
        for retry in 0..TOKEN_LOCK_MAX_RETRIES {
            if !is_token_locked(&self.lock_path) {
                break;
            }

            let backoff = TOKEN_LOCK_BASE_BACKOFF * (1 << (retry + 1));
            sleep_fn(backoff);
        }

        // After exhaustion, if still locked, delete the stale lock.
        if is_token_locked(&self.lock_path) {
            self.delete_lock_file()?;
        }

        self.create_lock_file()
    }

    /// Create the lock file with mode 0600 (O_CREATE | O_EXCL).
    fn create_lock_file(&self) -> Result<()> {
        use std::os::unix::fs::OpenOptionsExt;

        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&self.lock_path)
            .map_err(|e| {
                ConfigError::invariant(format!(
                    "failed to acquire token lock at {}: {e}",
                    self.lock_path.display()
                ))
            })?;

        self.acquired.store(true, Ordering::Release);
        Ok(())
    }

    /// Delete the lock file, returning an error with guidance if removal fails.
    fn delete_lock_file(&self) -> Result<()> {
        match std::fs::remove_file(&self.lock_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(ConfigError::invariant(format!(
                "failed to acquire a new Access token. Please try to delete {}",
                self.lock_path.display()
            ))),
        }
    }

    /// Release the token lock.
    ///
    /// Removes the lock file. Safe to call multiple times.
    pub fn release(&self) {
        if self.acquired.swap(false, Ordering::AcqRel) {
            let _ = std::fs::remove_file(&self.lock_path);
        }
    }

    /// Check whether the lock is currently held by this instance.
    pub fn is_acquired(&self) -> bool {
        self.acquired.load(Ordering::Acquire)
    }

    /// Clean up the lock file during signal-driven exit.
    ///
    /// Called from a signal handler context. Unlike `release()`, this does
    /// not check `acquired` — it unconditionally removes the lock file to
    /// match Go behavior where the signal handler deletes and exits.
    pub fn signal_cleanup(&self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

impl Drop for TokenLock {
    fn drop(&mut self) {
        if *self.acquired.get_mut() {
            let _ = std::fs::remove_file(&self.lock_path);
        }
    }
}

/// Check if a token lock file exists.
///
/// Matches Go `isTokenLocked()`: returns true only when the file exists
/// and there is no access error.
fn is_token_locked(lock_path: &Path) -> bool {
    lock_path.try_exists().unwrap_or(false)
}

/// Acquire a lock file for tunnel token operations.
///
/// Simple single-attempt API. For backoff retry behavior matching Go
/// baseline, use [`TokenLock`] directly.
pub fn acquire_token_lock(token_path: &Path) -> Result<PathBuf> {
    let lock = TokenLock::new(token_path);
    lock.create_lock_file()?;

    let lock_path = lock.lock_path().to_path_buf();

    // Prevent Drop from deleting — caller owns the lock path.
    lock.acquired.store(false, Ordering::Release);

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
    use std::sync::atomic::AtomicU32;
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

    // --- HIS-062: TokenLock struct parity tests ---

    #[test]
    fn token_lock_struct_acquire_release() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let token_path = std::env::temp_dir().join(format!("cfdrs-his-tl-{unique}.json"));

        let lock = TokenLock::new(&token_path);
        lock.acquire().expect("first acquire should succeed");
        assert!(lock.is_acquired());
        assert!(lock.lock_path().exists());

        lock.release();
        assert!(!lock.is_acquired());
        assert!(!lock.lock_path().exists());
    }

    #[test]
    fn token_lock_struct_has_mode_0600() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let token_path = std::env::temp_dir().join(format!("cfdrs-his-tl-perm-{unique}.json"));

        let lock = TokenLock::new(&token_path);
        lock.acquire().expect("acquire");

        let metadata = std::fs::metadata(lock.lock_path()).expect("metadata");
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "lock file should have mode 0600, got {mode:o}");

        lock.release();
    }

    #[test]
    fn token_lock_drop_cleans_up() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let token_path = std::env::temp_dir().join(format!("cfdrs-his-tl-drop-{unique}.json"));

        let lock_path;
        {
            let lock = TokenLock::new(&token_path);
            lock.acquire().expect("acquire");
            lock_path = lock.lock_path().to_path_buf();
            assert!(lock_path.exists());
            // Drop happens here.
        }

        assert!(!lock_path.exists(), "lock file should be cleaned up on drop");
    }

    #[test]
    fn token_lock_backoff_deletes_stale_lock_after_exhaustion() {
        use std::sync::atomic::AtomicU32;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let token_path = std::env::temp_dir().join(format!("cfdrs-his-tl-stale-{unique}.json"));

        // Pre-create a stale lock file.
        let lock_path = token_path.with_extension("lock");
        std::fs::write(&lock_path, b"").expect("create stale lock");
        assert!(lock_path.exists());

        // Use a no-op sleep to avoid real delays.
        let sleep_count = AtomicU32::new(0);
        let lock = TokenLock::new(&token_path);

        lock.acquire_with_sleep(|_| {
            sleep_count.fetch_add(1, Ordering::Relaxed);
        })
        .expect("acquire should succeed after deleting stale lock");

        assert!(lock.is_acquired());
        assert!(lock.lock_path().exists());

        // Should have slept TOKEN_LOCK_MAX_RETRIES times.
        assert_eq!(
            sleep_count.load(Ordering::Relaxed),
            TOKEN_LOCK_MAX_RETRIES,
            "should back off {TOKEN_LOCK_MAX_RETRIES} times before deleting stale lock"
        );

        lock.release();
    }

    #[test]
    fn token_lock_backoff_durations_are_exponential() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let token_path = std::env::temp_dir().join(format!("cfdrs-his-tl-exp-{unique}.json"));

        // Pre-create a stale lock file.
        let lock_path = token_path.with_extension("lock");
        std::fs::write(&lock_path, b"").expect("create stale lock");

        let durations = std::sync::Mutex::new(Vec::new());
        let lock = TokenLock::new(&token_path);

        lock.acquire_with_sleep(|d| {
            durations.lock().expect("mutex").push(d);
        })
        .expect("acquire");

        let recorded = durations.lock().expect("mutex");
        let expected: Vec<Duration> = (0..TOKEN_LOCK_MAX_RETRIES)
            .map(|i| TOKEN_LOCK_BASE_BACKOFF * (1 << (i + 1)))
            .collect();

        assert_eq!(
            *recorded, expected,
            "backoff durations should double each iteration: got {recorded:?}, expected {expected:?}"
        );

        lock.release();
    }

    #[test]
    fn token_lock_no_backoff_when_not_locked() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let token_path = std::env::temp_dir().join(format!("cfdrs-his-tl-fast-{unique}.json"));

        let sleep_count = AtomicU32::new(0);
        let lock = TokenLock::new(&token_path);

        lock.acquire_with_sleep(|_| {
            sleep_count.fetch_add(1, Ordering::Relaxed);
        })
        .expect("acquire without contention");

        assert_eq!(
            sleep_count.load(Ordering::Relaxed),
            0,
            "no backoff when lock file does not exist"
        );

        lock.release();
    }

    #[test]
    fn token_lock_signal_cleanup_removes_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let token_path = std::env::temp_dir().join(format!("cfdrs-his-tl-sig-{unique}.json"));

        let lock = TokenLock::new(&token_path);
        lock.acquire().expect("acquire");
        assert!(lock.lock_path().exists());

        // Signal cleanup should remove the file unconditionally.
        lock.signal_cleanup();
        assert!(
            !lock.lock_path().exists(),
            "signal_cleanup should remove lock file"
        );
    }

    #[test]
    fn token_lock_release_is_idempotent() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let token_path = std::env::temp_dir().join(format!("cfdrs-his-tl-idem-{unique}.json"));

        let lock = TokenLock::new(&token_path);
        lock.acquire().expect("acquire");

        lock.release();
        assert!(!lock.is_acquired());

        // Second release should not panic.
        lock.release();
        assert!(!lock.is_acquired());
    }

    #[test]
    fn token_lock_max_retries_is_seven() {
        assert_eq!(TOKEN_LOCK_MAX_RETRIES, 7, "Go baseline uses 7 retries");
    }

    #[test]
    fn token_lock_base_backoff_is_one_second() {
        assert_eq!(
            TOKEN_LOCK_BASE_BACKOFF,
            Duration::from_secs(1),
            "Go baseline uses DefaultBaseTime = 1 second"
        );
    }

    #[test]
    fn token_lock_stale_deletion_error_message_matches_go() {
        // Go: "failed to acquire a new Access token. Please try to delete <path>"
        // We check the error message format is consistent.
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let token_path = std::env::temp_dir().join(format!("cfdrs-his-tl-err-{unique}.json"));

        let lock = TokenLock::new(&token_path);
        let lock_path = lock.lock_path().to_path_buf();

        // Create a directory at the lock path — can't be removed by remove_file.
        std::fs::create_dir_all(&lock_path).expect("create dir");

        // delete_lock_file should return a descriptive error.
        let err = lock.delete_lock_file().expect_err("should fail on directory");
        let msg = err.to_string();
        assert!(
            msg.contains("failed to acquire a new Access token"),
            "error should match Go message format, got: {msg}"
        );
        assert!(
            msg.contains("Please try to delete"),
            "error should include guidance, got: {msg}"
        );

        std::fs::remove_dir_all(&lock_path).expect("cleanup");
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
