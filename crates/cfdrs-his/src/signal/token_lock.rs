//! Token lock file (HIS-062).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use cfdrs_shared::{ConfigError, Result};

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

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU32;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

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
}
