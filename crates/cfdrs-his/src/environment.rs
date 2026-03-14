//! Host environment detection and OS-level queries.
//!
//! Covers HIS-050 through HIS-055.

use std::path::PathBuf;

use cfdrs_shared::{ConfigError, Result};

// --- HIS-050: UID detection ---

/// Get the current effective user ID.
///
/// Go: `os.Getuid()` in `diagnostic/handlers.go`.
pub fn current_uid() -> u32 {
    // std::os::unix does not expose getuid(); use /proc instead.
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Uid:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(u32::MAX)
}

/// Check if running as root.
pub fn is_root() -> bool {
    current_uid() == 0
}

// --- HIS-051: terminal detection ---

/// Check if stdout is a terminal.
///
/// Go: `term.IsTerminal(os.Stdout.Fd())` in `updater/update.go`.
pub fn is_terminal() -> bool {
    use std::os::unix::io::AsRawFd;
    let fd = std::io::stdout().as_raw_fd();
    // Use the /proc/self/fd/<N> symlink to check if it points to a pts/tty.
    std::fs::read_link(format!("/proc/self/fd/{fd}"))
        .map(|target| {
            let s = target.to_string_lossy();
            s.starts_with("/dev/pts/") || s.starts_with("/dev/tty")
        })
        .unwrap_or(false)
}

// --- HIS-052: OS-specific build tags ---

/// Target OS string for the current build.
///
/// Go uses build tags (`//go:build linux`). Rust uses `cfg(target_os)`.
pub const TARGET_OS: &str = std::env::consts::OS;

/// Target architecture string.
pub const TARGET_ARCH: &str = std::env::consts::ARCH;

// --- HIS-054: current executable path ---

/// Get the path to the currently running executable.
///
/// Go: `os.Executable()`.
pub fn current_executable() -> Result<PathBuf> {
    std::env::current_exe()
        .map_err(|e| ConfigError::invariant(format!("failed to determine executable path: {e}")))
}

// --- HIS-053: no-installer / no-systemd-unit markers ---

/// Whether the binary was built as a standalone (no-installer) artifact.
///
/// Go: controlled by build tags. Rust: controlled by Cargo features
/// or environment variables at build time.
pub fn is_standalone_build() -> bool {
    // Default: standalone unless package manager detection says otherwise.
    !is_package_managed()
}

// --- HIS-056, HIS-057: package manager detection ---

/// Marker file path that Go's `postinst.sh` creates.
pub const INSTALLED_FROM_PACKAGE_MARKER: &str = "/usr/local/etc/cloudflared/.installedFromPackageManager";

/// Check if installed via a package manager.
///
/// Go: checks for `.installedFromPackageManager` file.
pub fn is_package_managed() -> bool {
    std::path::Path::new(INSTALLED_FROM_PACKAGE_MARKER).exists()
}

/// Installation paths used by package manager installs.
///
/// Go `postinst.sh`: `/usr/local/bin/cloudflared`.
pub const PACKAGE_INSTALL_BIN: &str = "/usr/local/bin/cloudflared";
pub const PACKAGE_INSTALL_CONFIG_DIR: &str = "/usr/local/etc/cloudflared/";

// --- HIS-055: dynamic linker paths ---

/// Known dynamic linker paths for x86_64-linux-gnu.
///
/// These are checked in Go to verify glibc availability.
pub const KNOWN_LINKER_PATHS: &[&str] = &[
    "/lib64/ld-linux-x86-64.so.2",
    "/lib/x86_64-linux-gnu/libc.so.6",
    "/usr/lib64/libc.so.6",
];

/// Check if a compatible C runtime is available.
pub fn has_compatible_libc() -> bool {
    KNOWN_LINKER_PATHS
        .iter()
        .any(|p| std::path::Path::new(p).exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_uid_returns_something() {
        // Just verify it doesn't panic.
        let _ = current_uid();
    }

    #[test]
    fn target_os_is_linux() {
        assert_eq!(TARGET_OS, "linux");
    }

    #[test]
    fn target_arch_is_set() {
        assert!(!TARGET_ARCH.is_empty());
    }

    #[test]
    fn current_executable_succeeds() {
        let exe = current_executable().expect("should find exe");
        assert!(exe.exists());
    }

    #[test]
    fn is_terminal_does_not_panic() {
        // In test context, stdout is usually not a terminal.
        let _ = is_terminal();
    }
}
