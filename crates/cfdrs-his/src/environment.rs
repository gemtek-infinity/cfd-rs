//! Host environment detection and OS-level queries.
//!
//! Covers HIS-050 through HIS-055.

use std::path::PathBuf;

use cfdrs_shared::{ConfigError, Result};

// --- HIS-050: UID detection ---

/// Get the current effective user ID.
///
/// Returns `u32::MAX` when `/proc/self/status` is unreadable or the
/// `Uid:` line cannot be parsed, which ensures `is_root()` returns
/// `false` under constrained environments.
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
/// Returns `false` if the `/proc/self/fd/<N>` symlink cannot be read,
/// which is the safe default for non-interactive contexts.
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

// --- HIS-031: container runtime detection ---
//
// Runtime detection strategy (steps 2 and 3) adapted from the isdocker crate
// by Shahnoor Mujawar (MIT license):
// https://github.com/shahnoormujawar/isdocker

/// Whether the current process is running inside a container runtime.
///
/// Go baseline: the Makefile checks `ifdef CONTAINER_BUILD` and passes
/// `-X "metrics.Runtime=virtual"` at link time. The Dockerfile sets
/// `CONTAINER_BUILD=1`. This is a **compile-time** signal.
///
/// The Rust equivalent combines:
/// 1. Compile-time: `option_env!("CONTAINER_BUILD")` — exact Go parity.
/// 2. Runtime: `/.dockerenv` existence — Docker creates this in every
///    container.
/// 3. Runtime: `/proc/self/cgroup` containing `docker`, `kubepods`, or
///    `containerd` — catches containerd and Kubernetes pods.
///
/// When true, the metrics server binds to `0.0.0.0` instead of `localhost`.
pub fn is_container_runtime() -> bool {
    is_container_build() || dockerenv_exists() || cgroup_indicates_container()
}

/// Compile-time container build flag, matching Go `CONTAINER_BUILD`.
///
/// The Go Makefile sets `-X "metrics.Runtime=virtual"` when the
/// `CONTAINER_BUILD` env var is defined at build time.
fn is_container_build() -> bool {
    option_env!("CONTAINER_BUILD").is_some()
}

/// Docker creates `/.dockerenv` inside every container.
fn dockerenv_exists() -> bool {
    std::path::Path::new("/.dockerenv").exists()
}

/// Scan `/proc/self/cgroup` for known container runtime markers.
fn cgroup_indicates_container() -> bool {
    std::fs::read_to_string("/proc/self/cgroup")
        .map(|contents| contains_container_marker(&contents))
        .unwrap_or(false)
}

/// Check cgroup contents for Docker, Kubernetes, or containerd markers.
fn contains_container_marker(contents: &str) -> bool {
    contents.contains("docker") || contents.contains("kubepods") || contents.contains("containerd")
}

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

    // --- HIS-050: UID detection ---

    #[test]
    fn current_uid_returns_something() {
        // Just verify it doesn't panic.
        let _ = current_uid();
    }

    #[test]
    fn current_uid_reads_from_proc_self_status() {
        // On Linux, /proc/self/status always exists and has a Uid line.
        let uid = current_uid();
        // In a non-root test context uid should be a real id, not the
        // fallback value, unless running with unusual isolation.
        assert_ne!(uid, u32::MAX, "expected a real uid from /proc/self/status");
    }

    #[test]
    fn is_root_consistent_with_uid() {
        assert_eq!(is_root(), current_uid() == 0);
    }

    // --- HIS-051: terminal detection ---

    #[test]
    fn is_terminal_does_not_panic() {
        // In test context, stdout is usually not a terminal, but
        // this depends on the runner. Just verify it returns a bool.
        let _ = is_terminal();
    }

    // --- HIS-052: target OS/arch ---

    #[test]
    fn target_os_is_linux() {
        assert_eq!(TARGET_OS, "linux");
    }

    #[test]
    fn target_arch_is_set() {
        assert!(!TARGET_ARCH.is_empty());
    }

    // --- HIS-054: current executable ---

    #[test]
    fn current_executable_succeeds() {
        let exe = current_executable().expect("should find exe");
        assert!(exe.exists());
    }

    // --- HIS-049/056: package manager marker ---

    #[test]
    fn installed_from_package_marker_matches_go() {
        assert_eq!(
            INSTALLED_FROM_PACKAGE_MARKER,
            "/usr/local/etc/cloudflared/.installedFromPackageManager",
        );
    }

    #[test]
    fn package_install_paths_match_go() {
        // Go postinst.sh installs to /usr/local/bin/cloudflared
        assert_eq!(PACKAGE_INSTALL_BIN, "/usr/local/bin/cloudflared");
        assert_eq!(PACKAGE_INSTALL_CONFIG_DIR, "/usr/local/etc/cloudflared/");
    }

    // --- HIS-031: container runtime detection ---

    #[test]
    fn is_container_runtime_does_not_panic() {
        // Smoke test: must return a bool without panicking.
        let _ = is_container_runtime();
    }

    #[test]
    fn is_container_build_false_in_normal_tests() {
        // In normal test builds, CONTAINER_BUILD is not set.
        assert!(
            !is_container_build(),
            "default test build should not have CONTAINER_BUILD"
        );
    }

    #[test]
    fn container_marker_detection_positive() {
        assert!(contains_container_marker("12:blkio:/docker/abc123"));
        assert!(contains_container_marker("11:cpu:/kubepods/besteffort/pod123"));
        assert!(contains_container_marker("10:memory:/containerd/tasks/abc"));
    }

    #[test]
    fn container_marker_detection_negative() {
        assert!(!contains_container_marker(
            "11:cpu:/user.slice/user-1000.slice/session-1.scope"
        ));
        assert!(!contains_container_marker(""));
    }

    // --- HIS-055: linker paths ---

    #[test]
    fn known_linker_paths_not_empty() {
        assert!(!KNOWN_LINKER_PATHS.is_empty());
        // First entry must be the standard x86_64 dynamic linker.
        assert_eq!(KNOWN_LINKER_PATHS[0], "/lib64/ld-linux-x86-64.so.2");
    }
}
