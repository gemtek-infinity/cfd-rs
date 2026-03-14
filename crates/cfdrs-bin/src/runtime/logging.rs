use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use cfdrs_his::logging::{
    LOG_DIR_PERM_MODE, LOG_FILE_PERM_MODE, LogConfig, LogFormat, LogLevel, RollingConfig,
};
use tracing::Level;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::writer::MakeWriter;

use super::RUNTIME_LOGGING;

#[derive(Clone)]
struct RuntimeLogWriter {
    file: Option<Arc<Mutex<File>>>,
}

struct CompositeWriter {
    stderr: io::Stderr,
    file: Option<Arc<Mutex<File>>>,
}

impl Write for CompositeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stderr.write_all(buf)?;

        if let Some(file) = self.file.as_ref() {
            let mut guard = file
                .lock()
                .map_err(|_| io::Error::other("runtime log file mutex poisoned"))?;

            guard.write_all(buf)?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stderr.flush()?;

        if let Some(file) = self.file.as_ref() {
            let mut guard = file
                .lock()
                .map_err(|_| io::Error::other("runtime log file mutex poisoned"))?;

            guard.flush()?;
        }

        Ok(())
    }
}

impl<'a> MakeWriter<'a> for RuntimeLogWriter {
    type Writer = CompositeWriter;

    fn make_writer(&'a self) -> Self::Writer {
        CompositeWriter {
            stderr: io::stderr(),
            file: self.file.clone(),
        }
    }
}

pub(crate) fn install_runtime_logging(log_config: &LogConfig, transport_level: Option<LogLevel>) {
    let file = open_runtime_log_file(log_config)
        .inspect_err(|error| eprintln!("runtime logging fallback to stderr only: {error}"))
        .ok()
        .flatten()
        .map(|file| Arc::new(Mutex::new(file)));

    let writer = RuntimeLogWriter { file };
    let level = resolve_global_level(log_config.min_level, transport_level);

    RUNTIME_LOGGING.get_or_init(|| match log_config.format {
        LogFormat::Json => {
            let subscriber = fmt()
                .with_writer(writer)
                .with_max_level(level)
                .with_target(false)
                .without_time()
                .json()
                .finish();

            let _ = tracing::subscriber::set_global_default(subscriber);
        }
        LogFormat::Text => {
            let subscriber = fmt()
                .with_writer(writer)
                .with_max_level(level)
                .with_target(false)
                .without_time()
                .with_ansi(false)
                .compact()
                .finish();

            let _ = tracing::subscriber::set_global_default(subscriber);
        }
    });
}

fn open_runtime_log_file(log_config: &LogConfig) -> io::Result<Option<File>> {
    let path = match runtime_log_path(log_config) {
        Some(path) => path,
        None => return Ok(None),
    };

    ensure_parent_directory(&path)?;

    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .mode(LOG_FILE_PERM_MODE)
        .open(&path)?;

    let permissions = fs::Permissions::from_mode(LOG_FILE_PERM_MODE);
    fs::set_permissions(&path, permissions)?;

    Ok(Some(file))
}

fn runtime_log_path(log_config: &LogConfig) -> Option<PathBuf> {
    if let Some(file) = log_config.file.as_ref() {
        return Some(file.full_path());
    }

    log_config.rolling.as_ref().map(rolling_log_path)
}

fn rolling_log_path(config: &RollingConfig) -> PathBuf {
    config.dirname.join(&config.filename)
}

fn ensure_parent_directory(path: &Path) -> io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    fs::create_dir_all(parent)?;
    let permissions = fs::Permissions::from_mode(LOG_DIR_PERM_MODE);
    fs::set_permissions(parent, permissions)?;

    Ok(())
}

fn resolve_global_level(app_level: LogLevel, transport_level: Option<LogLevel>) -> Level {
    let selected = match transport_level {
        Some(candidate) if verbosity_rank(candidate) < verbosity_rank(app_level) => candidate,
        _ => app_level,
    };

    tracing_level(selected)
}

fn verbosity_rank(level: LogLevel) -> u8 {
    match level {
        LogLevel::Debug => 0,
        LogLevel::Info => 1,
        LogLevel::Warn => 2,
        LogLevel::Error | LogLevel::Fatal => 3,
    }
}

fn tracing_level(level: LogLevel) -> Level {
    match level {
        LogLevel::Debug => Level::DEBUG,
        LogLevel::Info => Level::INFO,
        LogLevel::Warn => Level::WARN,
        LogLevel::Error | LogLevel::Fatal => Level::ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_log_path_prefers_explicit_file() {
        let config = LogConfig {
            file: Some(cfdrs_his::logging::FileConfig {
                dirname: PathBuf::from("/tmp"),
                filename: "cloudflared.log".to_owned(),
            }),
            ..LogConfig::default()
        };

        assert_eq!(
            runtime_log_path(&config),
            Some(PathBuf::from("/tmp/cloudflared.log"))
        );
    }

    #[test]
    fn runtime_log_path_uses_rolling_target_when_present() {
        let config = LogConfig {
            console: None,
            file: None,
            rolling: Some(RollingConfig {
                dirname: PathBuf::from("/var/log/cloudflared"),
                filename: "cloudflared.log".to_owned(),
                ..RollingConfig::default()
            }),
            ..LogConfig::default()
        };

        assert_eq!(
            runtime_log_path(&config),
            Some(PathBuf::from("/var/log/cloudflared/cloudflared.log"))
        );
    }

    #[test]
    fn transport_level_can_raise_global_verbosity() {
        assert_eq!(
            resolve_global_level(LogLevel::Warn, Some(LogLevel::Debug)),
            Level::DEBUG
        );
        assert_eq!(
            resolve_global_level(LogLevel::Info, Some(LogLevel::Error)),
            Level::INFO
        );
    }
}
