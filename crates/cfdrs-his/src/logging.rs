//! Logging sink contracts, journalctl constants, and re-exports.
//!
//! Covers HIS-063 through HIS-068.
//!
//! Log configuration types (`LogLevel`, `LogFormat`, `LogConfig`,
//! `RollingConfig`, `FileConfig`, `ConsoleConfig`, `build_log_config`)
//! now live in `cfdrs-shared` as cross-domain shared types (see ADR-0007).
//!
//! This module retains ownership of:
//! - `LogSink` trait (host sink contract)
//! - journalctl collection constants (HIS-036)
//!
//! It re-exports the shared config types for backward compatibility.

// Re-export shared log config types so existing `cfdrs_his::logging::*`
// paths continue to work.
pub use cfdrs_shared::{
    ConsoleConfig, DEFAULT_LOG_DIRECTORY, FileConfig, LOG_DIR_PERM_MODE, LOG_FILE_PERM_MODE, LogConfig,
    LogFormat, LogLevel, RollingConfig, build_log_config,
};

// --- HIS-066: journald / systemd logging ---

/// Trait for host log sinks. Implementations live in cfdrs-bin.
///
/// Go uses `resilientMultiWriter` to fan out to console, file, rolling,
/// and management logger simultaneously.
pub trait LogSink: Send + Sync {
    /// Write a structured log event.
    fn write_event(&self, level: LogLevel, message: &str, fields: &[(&str, &str)]);

    /// Flush pending log data.
    fn flush(&self);
}

// --- HIS-036: journalctl log collection ---

/// The journalctl command Go uses for log collection.
pub const JOURNALCTL_COMMAND: &str = "journalctl";

/// Arguments matching Go `log_collector_host.go`.
pub const JOURNALCTL_ARGS: &[&str] = &["-u", "cloudflared.service", "--since", "2 weeks ago"];

/// Fallback log file path if journalctl is unavailable.
pub const FALLBACK_LOG_PATH: &str = "/var/log/cloudflared.err";

#[cfg(test)]
mod tests {
    use super::*;

    // --- HIS-036: journalctl constants match Go ---

    #[test]
    fn journalctl_collection_constants_match_go_baseline() {
        // Go: `journalctl -u cloudflared.service --since "2 weeks ago"`
        assert_eq!(JOURNALCTL_COMMAND, "journalctl");
        assert_eq!(
            JOURNALCTL_ARGS,
            &["-u", "cloudflared.service", "--since", "2 weeks ago"]
        );
        assert_eq!(FALLBACK_LOG_PATH, "/var/log/cloudflared.err");
    }

    #[test]
    fn journalctl_args_length_is_four() {
        // Go passes exactly 4 args: -u, service name, --since, time window
        assert_eq!(JOURNALCTL_ARGS.len(), 4);
    }
}
