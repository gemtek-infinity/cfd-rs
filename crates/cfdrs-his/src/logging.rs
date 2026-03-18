//! Logging sink contracts, host log collection, and re-exports.
//!
//! Covers HIS-063 through HIS-068.
//!
//! Log configuration types (`LogLevel`, `LogFormat`, `LogConfig`,
//! `RollingConfig`, `FileConfig`, `ConsoleConfig`, `build_log_config`)
//! now live in `cfdrs-shared` as cross-domain shared types (see ADR-0007).
//!
//! This module retains ownership of:
//! - `LogSink` trait (host sink contract)
//! - host log collection (HIS-036)
//!
//! It re-exports the shared config types for backward compatibility.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use thiserror::Error;

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
pub const JOURNALCTL_ARGS: &[&str] = &["--since", "2 weeks ago", "-u", "cloudflared.service"];

/// Fallback log file path if journalctl is unavailable.
pub const FALLBACK_LOG_PATH: &str = "/var/log/cloudflared.err";

pub const LINUX_SERVICE_CONFIGURATION_PATH: &str = "/etc/systemd/system/cloudflared.service";
pub const DARWIN_MANAGED_LOG_PATH: &str = "/Library/Logs/com.cloudflare.cloudflared.err.log";
pub const LOG_FILENAME: &str = "cloudflared_logs.txt";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostLogConfiguration {
    pub uid: u32,
    pub log_file: Option<PathBuf>,
    pub log_directory: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostLogSource {
    Journalctl,
    File(PathBuf),
    Directory(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostLogCollection {
    pub path: PathBuf,
    pub cleanup_required: bool,
    pub source: HostLogSource,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HostLogError {
    #[error("managed log directory not found")]
    ManagedLogNotFound,
    #[error("provided log configuration is invalid")]
    InvalidConfiguration,
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Command(String),
}

pub fn resolve_host_log_source(config: &HostLogConfiguration) -> Result<HostLogSource, HostLogError> {
    if config.uid == 0 {
        if cfg!(target_os = "linux") && Path::new(LINUX_SERVICE_CONFIGURATION_PATH).exists() {
            return Ok(HostLogSource::Journalctl);
        }

        return managed_log_path().map(HostLogSource::File);
    }

    if let Some(path) = config.log_file.as_ref() {
        return Ok(HostLogSource::File(path.clone()));
    }

    if let Some(path) = config.log_directory.as_ref() {
        return Ok(HostLogSource::Directory(path.clone()));
    }

    Err(HostLogError::InvalidConfiguration)
}

pub fn collect_host_logs(config: &HostLogConfiguration) -> Result<HostLogCollection, HostLogError> {
    let source = resolve_host_log_source(config)?;
    match &source {
        HostLogSource::Journalctl => {
            let path = write_command_output(JOURNALCTL_COMMAND, JOURNALCTL_ARGS)?;
            Ok(HostLogCollection {
                path,
                cleanup_required: true,
                source,
            })
        }
        HostLogSource::File(path) => Ok(HostLogCollection {
            path: path.clone(),
            cleanup_required: false,
            source,
        }),
        HostLogSource::Directory(path) => {
            let merged = copy_files_from_directory(path)?;
            Ok(HostLogCollection {
                path: merged,
                cleanup_required: true,
                source,
            })
        }
    }
}

fn managed_log_path() -> Result<PathBuf, HostLogError> {
    if cfg!(target_os = "macos") {
        let system_path = PathBuf::from(DARWIN_MANAGED_LOG_PATH);
        if system_path.exists() {
            return Ok(system_path);
        }

        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or(HostLogError::ManagedLogNotFound)?;
        return Ok(home.join(DARWIN_MANAGED_LOG_PATH.trim_start_matches('/')));
    }

    if cfg!(target_os = "linux") {
        return Ok(PathBuf::from(FALLBACK_LOG_PATH));
    }

    Err(HostLogError::ManagedLogNotFound)
}

fn write_command_output(command: &str, args: &[&str]) -> Result<PathBuf, HostLogError> {
    let path = unique_temp_log_path();
    let mut output_handle = std::fs::File::create(&path)
        .map_err(|error| HostLogError::Io(format!("error opening output file: {error}")))?;

    let mut child = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| HostLogError::Command(format!("error running command '{command}': {error}")))?;

    if let Some(mut stdout) = child.stdout.take() {
        copy_pipe(&mut stdout, &mut output_handle, command)?;
    }
    if let Some(mut stderr) = child.stderr.take() {
        copy_pipe(&mut stderr, &mut output_handle, command)?;
    }

    let status = child
        .wait()
        .map_err(|error| HostLogError::Command(format!("error waiting from command '{command}': {error}")))?;
    if !status.success() {
        return Err(HostLogError::Command(format!(
            "error waiting from command '{command}': exit {status}"
        )));
    }

    Ok(path)
}

fn copy_pipe(reader: &mut dyn Read, writer: &mut dyn Write, command: &str) -> Result<(), HostLogError> {
    std::io::copy(reader, writer).map_err(|error| {
        HostLogError::Io(format!(
            "error copying output from {command} to log file: {error}"
        ))
    })?;
    Ok(())
}

fn copy_files_from_directory(path: &Path) -> Result<PathBuf, HostLogError> {
    let entries = std::fs::read_dir(path)
        .map_err(|error| HostLogError::Io(format!("error reading directory {}: {error}", path.display())))?;
    let output_path = unique_temp_log_path();
    let mut output = std::fs::File::create(&output_path)
        .map_err(|error| HostLogError::Io(format!("creating file {}: {error}", output_path.display())))?;

    let mut paths: Vec<PathBuf> = Vec::new();

    for entry in entries {
        let entry =
            entry.map_err(|error| HostLogError::Io(format!("error reading directory entry: {error}")))?;
        let entry_path = entry.path();
        if entry_path.is_file() {
            paths.push(entry_path);
        }
    }

    paths.sort();

    for entry_path in &paths {
        copy_file_into(entry_path, &mut output)?;
    }

    let duplicated_current = path.join("cloudflared.log");
    if duplicated_current.is_file() {
        copy_file_into(&duplicated_current, &mut output)?;
    }

    Ok(output_path)
}

fn copy_file_into(path: &Path, writer: &mut dyn Write) -> Result<(), HostLogError> {
    let mut input = std::fs::File::open(path)
        .map_err(|error| HostLogError::Io(format!("error opening file {}:{error}", path.display())))?;
    std::io::copy(&mut input, writer)
        .map_err(|error| HostLogError::Io(format!("error copying file {}:{error}", path.display())))?;
    Ok(())
}

fn unique_temp_log_path() -> PathBuf {
    std::env::temp_dir().join(format!("{LOG_FILENAME}.{}", uuid::Uuid::new_v4()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cfdrs-his-logging-{name}-{suffix}"));
        fs::create_dir_all(&path).expect("mkdir");
        path
    }

    // --- HIS-036: journalctl constants match Go ---

    #[test]
    fn journalctl_collection_constants_match_go_baseline() {
        // Go: `journalctl --since "2 weeks ago" -u cloudflared.service`
        assert_eq!(JOURNALCTL_COMMAND, "journalctl");
        assert_eq!(
            JOURNALCTL_ARGS,
            &["--since", "2 weeks ago", "-u", "cloudflared.service"]
        );
        assert_eq!(FALLBACK_LOG_PATH, "/var/log/cloudflared.err");
    }

    #[test]
    fn journalctl_args_length_is_four() {
        // Go passes exactly 4 args: --since, time window, -u, service name
        assert_eq!(JOURNALCTL_ARGS.len(), 4);
    }

    #[test]
    fn non_root_prefers_log_file_then_directory() {
        let file_path = PathBuf::from("/tmp/cloudflared.log");
        let directory_path = PathBuf::from("/tmp/cloudflared");
        let source = resolve_host_log_source(&HostLogConfiguration {
            uid: 1000,
            log_file: Some(file_path.clone()),
            log_directory: Some(directory_path),
        })
        .expect("source");
        assert_eq!(source, HostLogSource::File(file_path));
    }

    #[test]
    fn invalid_non_root_configuration_matches_go_error() {
        let error = resolve_host_log_source(&HostLogConfiguration {
            uid: 1000,
            log_file: None,
            log_directory: None,
        })
        .expect_err("missing logs");
        assert_eq!(error.to_string(), "provided log configuration is invalid");
    }

    #[test]
    fn directory_merge_duplicates_cloudflared_log_like_go() {
        let root = temp_dir("dir-merge");
        fs::write(root.join("cloudflared.log"), "first\n").expect("write");
        fs::write(root.join("cloudflared.log.1"), "second\n").expect("write");

        let merged = collect_host_logs(&HostLogConfiguration {
            uid: 1000,
            log_file: None,
            log_directory: Some(root.clone()),
        })
        .expect("collect");
        let contents = fs::read_to_string(&merged.path).expect("read merged");
        assert!(contents.contains("first\nsecond\n"));
        assert_eq!(contents.matches("first\n").count(), 2);

        let _ = fs::remove_file(merged.path);
        let _ = fs::remove_dir_all(root);
    }
}
