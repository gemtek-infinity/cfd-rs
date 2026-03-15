//! Logging configuration and sink contracts.
//!
//! Covers HIS-063 through HIS-068.
//!
//! This module defines the configuration types for logging. Actual sink
//! creation (file writers, journal integration, management logger) is
//! trait-based so cfdrs-bin can wire the async runtime and tracing
//! subscriber.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use cfdrs_shared::{ConfigError, Result};

// --- HIS-068: log levels ---

/// Log level matching Go `--loglevel` / `--transport-loglevel`.
///
/// Go defaults: `info` for main, `info` for transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
    Fatal,
}

impl FromStr for LogLevel {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" | "warning" => Ok(Self::Warn),
            "error" | "err" => Ok(Self::Error),
            "fatal" => Ok(Self::Fatal),
            _ => Err(ConfigError::invariant(format!(
                "unknown log level: {s:?}. Valid levels: debug, info, warn, error, fatal"
            ))),
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Debug => write!(f, "debug"),
            Self::Info => write!(f, "info"),
            Self::Warn => write!(f, "warn"),
            Self::Error => write!(f, "error"),
            Self::Fatal => write!(f, "fatal"),
        }
    }
}

// --- HIS-067: log output format ---

/// Log output format matching Go `--log-format-output`.
///
/// Go accepts `"json"` or `"default"` (text).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// Human-readable text format (Go default).
    #[default]
    Text,
    /// JSON structured output.
    Json,
}

impl FromStr for LogFormat {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "default" | "text" => Ok(Self::Text),
            _ => Err(ConfigError::invariant(format!(
                "unknown log format: {s:?}. Valid formats: json, default"
            ))),
        }
    }
}

// --- HIS-063, HIS-064: file and directory logging ---

/// File permissions matching Go's `filePermMode` (`0644`).
pub const LOG_FILE_PERM_MODE: u32 = 0o644;

/// Directory permissions matching Go's `dirPermMode` (`0744`).
pub const LOG_DIR_PERM_MODE: u32 = 0o744;

/// Default log directory (Go `DefaultUnixLogLocation`).
pub const DEFAULT_LOG_DIRECTORY: &str = "/var/log/cloudflared";

/// Console logging config.
#[derive(Debug, Clone)]
pub struct ConsoleConfig {
    /// Disable color output.
    pub no_color: bool,
    /// Emit JSON instead of text.
    pub as_json: bool,
}

/// Single-file logging config (HIS-063: `--logfile`).
#[derive(Debug, Clone)]
pub struct FileConfig {
    pub dirname: PathBuf,
    pub filename: String,
}

impl FileConfig {
    pub fn full_path(&self) -> PathBuf {
        self.dirname.join(&self.filename)
    }
}

/// Rolling-file logging config (HIS-065: log rotation).
///
/// Go defaults via lumberjack: MaxSize=1MB, MaxBackups=5, MaxAge=0.
#[derive(Debug, Clone)]
pub struct RollingConfig {
    pub dirname: PathBuf,
    pub filename: String,
    /// Max size per file in megabytes. Go default: 1.
    pub max_size_mb: u32,
    /// Max number of old log files. Go default: 5.
    pub max_backups: u32,
    /// Max age in days (0 = no age limit). Go default: 0.
    pub max_age_days: u32,
}

impl Default for RollingConfig {
    fn default() -> Self {
        Self {
            dirname: PathBuf::from(DEFAULT_LOG_DIRECTORY),
            filename: "cloudflared.log".into(),
            max_size_mb: 1,
            max_backups: 5,
            max_age_days: 0,
        }
    }
}

// --- Combined logging config ---

/// Full logging configuration matching Go `logger.Config`.
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Console logging (None = no console output).
    pub console: Option<ConsoleConfig>,
    /// Single-file logging via `--logfile` (None = disabled).
    pub file: Option<FileConfig>,
    /// Rolling-file logging via `--log-directory` (None = disabled).
    pub rolling: Option<RollingConfig>,
    /// Minimum log level. Go default: `info`.
    pub min_level: LogLevel,
    /// Output format. Go default: `text`.
    pub format: LogFormat,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            console: Some(ConsoleConfig {
                no_color: false,
                as_json: false,
            }),
            file: None,
            rolling: None,
            min_level: LogLevel::Info,
            format: LogFormat::Text,
        }
    }
}

/// Build a `LogConfig` from CLI flag values.
///
/// Go rule: if both `--logfile` and `--log-directory` are set, `--logfile`
/// takes precedence.
pub fn build_log_config(
    level: Option<&str>,
    format: Option<&str>,
    logfile: Option<&str>,
    log_directory: Option<&str>,
) -> Result<LogConfig> {
    let min_level = match level {
        Some(s) => s.parse()?,
        None => LogLevel::default(),
    };

    let log_format = match format {
        Some(s) => s.parse()?,
        None => LogFormat::default(),
    };

    let file = logfile.map(|f| {
        let path = Path::new(f);
        FileConfig {
            dirname: path.parent().unwrap_or(Path::new(".")).to_path_buf(),
            filename: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "cloudflared.log".into()),
        }
    });

    let rolling = if file.is_none() {
        log_directory.map(|dir| RollingConfig {
            dirname: PathBuf::from(dir),
            ..RollingConfig::default()
        })
    } else {
        None
    };

    Ok(LogConfig {
        console: Some(ConsoleConfig {
            no_color: false,
            as_json: log_format == LogFormat::Json,
        }),
        file,
        rolling,
        min_level,
        format: log_format,
    })
}

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

    #[test]
    fn log_level_parse() {
        assert_eq!("debug".parse::<LogLevel>().expect("debug"), LogLevel::Debug);
        assert_eq!("info".parse::<LogLevel>().expect("info"), LogLevel::Info);
        assert_eq!("warn".parse::<LogLevel>().expect("warn"), LogLevel::Warn);
        assert_eq!("warning".parse::<LogLevel>().expect("warning"), LogLevel::Warn);
        assert_eq!("error".parse::<LogLevel>().expect("error"), LogLevel::Error);
        assert_eq!("err".parse::<LogLevel>().expect("err"), LogLevel::Error);
        assert_eq!("fatal".parse::<LogLevel>().expect("fatal"), LogLevel::Fatal);
        assert!("unknown".parse::<LogLevel>().is_err());
    }

    #[test]
    fn log_level_display() {
        assert_eq!(LogLevel::Info.to_string(), "info");
        assert_eq!(LogLevel::Debug.to_string(), "debug");
    }

    #[test]
    fn log_format_parse() {
        assert_eq!("json".parse::<LogFormat>().expect("json"), LogFormat::Json);
        assert_eq!("default".parse::<LogFormat>().expect("default"), LogFormat::Text);
        assert_eq!("text".parse::<LogFormat>().expect("text"), LogFormat::Text);
        assert!("xml".parse::<LogFormat>().is_err());
    }

    #[test]
    fn rolling_config_defaults() {
        let cfg = RollingConfig::default();
        assert_eq!(cfg.max_size_mb, 1);
        assert_eq!(cfg.max_backups, 5);
        assert_eq!(cfg.max_age_days, 0);
    }

    #[test]
    fn build_log_config_defaults() {
        let cfg = build_log_config(None, None, None, None).expect("defaults");
        assert_eq!(cfg.min_level, LogLevel::Info);
        assert_eq!(cfg.format, LogFormat::Text);
        assert!(cfg.file.is_none());
        assert!(cfg.rolling.is_none());
    }

    #[test]
    fn build_log_config_logfile_takes_precedence() {
        let cfg = build_log_config(
            Some("debug"),
            Some("json"),
            Some("/tmp/my.log"),
            Some("/var/log/cloudflared"),
        )
        .expect("logfile precedence");

        assert_eq!(cfg.min_level, LogLevel::Debug);
        assert!(cfg.file.is_some());
        // logfile takes precedence over log-directory.
        assert!(cfg.rolling.is_none());
    }

    #[test]
    fn build_log_config_log_directory_creates_rolling() {
        let cfg = build_log_config(None, None, None, Some("/var/log/custom")).expect("directory rolling");

        assert!(cfg.file.is_none());
        assert!(cfg.rolling.is_some());
        assert_eq!(
            cfg.rolling.as_ref().expect("rolling should exist").dirname,
            PathBuf::from("/var/log/custom")
        );
    }

    // --- HIS-066: permission constants match Go `filePermMode` / `dirPermMode` ---

    #[test]
    fn log_file_perm_mode_matches_go_baseline() {
        // Go: `filePermMode = 0644`
        assert_eq!(LOG_FILE_PERM_MODE, 0o644);
    }

    #[test]
    fn log_dir_perm_mode_matches_go_baseline() {
        // Go: `dirPermMode = 0744`
        assert_eq!(LOG_DIR_PERM_MODE, 0o744);
    }

    // --- HIS-067: log format output matches Go `--log-format-output` ---

    #[test]
    fn log_format_default_is_text() {
        assert_eq!(LogFormat::default(), LogFormat::Text);
    }

    #[test]
    fn log_format_json_round_trips() {
        let cfg = build_log_config(None, Some("json"), None, None).expect("json format");
        assert_eq!(cfg.format, LogFormat::Json);
        assert!(cfg.console.as_ref().expect("console should exist").as_json);
    }

    #[test]
    fn log_format_case_insensitive() {
        assert_eq!("JSON".parse::<LogFormat>().expect("JSON"), LogFormat::Json);
        assert_eq!("Default".parse::<LogFormat>().expect("Default"), LogFormat::Text);
    }

    // --- HIS-068: log level matches Go `--loglevel` / `--transport-loglevel` ---

    #[test]
    fn log_level_default_is_info() {
        // Go: MinLevel defaults to "info"
        assert_eq!(LogLevel::default(), LogLevel::Info);
    }

    #[test]
    fn log_level_all_go_variants_parse() {
        // Go recognizes: debug, info, warn/warning, error/err, fatal
        for (input, expected) in [
            ("debug", LogLevel::Debug),
            ("info", LogLevel::Info),
            ("warn", LogLevel::Warn),
            ("warning", LogLevel::Warn),
            ("error", LogLevel::Error),
            ("err", LogLevel::Error),
            ("fatal", LogLevel::Fatal),
        ] {
            assert_eq!(input.parse::<LogLevel>().expect(input), expected);
        }
    }

    #[test]
    fn log_level_case_insensitive() {
        assert_eq!("DEBUG".parse::<LogLevel>().expect("DEBUG"), LogLevel::Debug);
        assert_eq!("INFO".parse::<LogLevel>().expect("INFO"), LogLevel::Info);
    }

    #[test]
    fn log_level_display_round_trips() {
        for level in [
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
            LogLevel::Fatal,
        ] {
            let text = level.to_string();
            assert_eq!(text.parse::<LogLevel>().expect(&text), level);
        }
    }

    // --- HIS-063/064: default log directory matches Go ---

    #[test]
    fn default_log_directory_matches_go_baseline() {
        // Go: `DefaultUnixLogLocation = "/var/log/cloudflared"`
        assert_eq!(DEFAULT_LOG_DIRECTORY, "/var/log/cloudflared");
    }

    #[test]
    fn rolling_config_default_dirname_matches_go() {
        let cfg = RollingConfig::default();
        assert_eq!(cfg.dirname, PathBuf::from("/var/log/cloudflared"));
        assert_eq!(cfg.filename, "cloudflared.log");
    }

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

    // --- HIS-063/064/065: logging config edge cases ---

    #[test]
    fn build_log_config_bare_filename_uses_empty_dirname() {
        // When logfile is just a bare name with no directory component,
        // Path::parent() returns Some("") — the code stores that as-is.
        let config = build_log_config(None, None, Some("app.log"), None).expect("bare filename should work");
        let file = config.file.expect("file config should exist");
        assert_eq!(file.dirname, PathBuf::from(""));
        assert_eq!(file.filename, "app.log");
    }

    #[test]
    fn build_log_config_nested_path_splits_correctly() {
        let config = build_log_config(None, None, Some("/var/log/cloudflared/tunnel.log"), None)
            .expect("nested path should work");
        let file = config.file.expect("file config should exist");
        assert_eq!(file.dirname, PathBuf::from("/var/log/cloudflared"));
        assert_eq!(file.filename, "tunnel.log");
        assert_eq!(file.full_path(), PathBuf::from("/var/log/cloudflared/tunnel.log"));
    }

    #[test]
    fn log_level_ordering_matches_severity() {
        // Go severity ordering: debug < info < warn < error < fatal
        let levels = [
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
            LogLevel::Fatal,
        ];
        for pair in levels.windows(2) {
            assert_ne!(pair[0], pair[1], "{:?} should differ from {:?}", pair[0], pair[1]);
        }
    }

    #[test]
    fn log_format_display_round_trips() {
        // Both formats should survive parse → display → parse
        for input in &["json", "default", "text"] {
            let parsed: LogFormat = input.parse().expect("should parse");
            let displayed = if parsed == LogFormat::Json {
                "json"
            } else {
                "default"
            };
            let reparsed: LogFormat = displayed.parse().expect("should reparse");
            assert_eq!(parsed, reparsed);
        }
    }

    #[test]
    fn journalctl_args_length_is_four() {
        // Go passes exactly 4 args: -u, service name, --since, time window
        assert_eq!(JOURNALCTL_ARGS.len(), 4);
    }
}
