use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use cfdrs_shared::{LOG_DIR_PERM_MODE, LOG_FILE_PERM_MODE, LogConfig, LogFormat, LogLevel, RollingConfig};
use tracing::Level;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::writer::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;

use super::RUNTIME_LOGGING;

const SECS_PER_DAY: u64 = 86_400;

#[derive(Clone)]
struct RuntimeLogWriter {
    file: Option<Arc<Mutex<RuntimeLogSink>>>,
}

struct CompositeWriter {
    stderr: io::Stderr,
    file: Option<Arc<Mutex<RuntimeLogSink>>>,
}

struct RuntimeLogSink {
    target: SinkTarget,
}

enum SinkTarget {
    Plain(File),
    Rotating(RotatingFile),
}

struct RotatingFile {
    config: RollingConfig,
    path: PathBuf,
    file: File,
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
    let file = open_runtime_log_sink(log_config)
        .inspect_err(|error| eprintln!("runtime logging fallback to stderr only: {error}"))
        .ok()
        .flatten()
        .map(|file| Arc::new(Mutex::new(file)));

    let writer = RuntimeLogWriter { file };
    let level = resolve_global_level(log_config.min_level, transport_level);

    // Optional journald layer — active when JOURNAL_STREAM is set (indicates
    // the process was started by systemd with journal output).
    let journal_layer = journal_layer();

    RUNTIME_LOGGING.get_or_init(|| match log_config.format {
        LogFormat::Json => {
            let fmt_layer = fmt::layer()
                .with_writer(writer)
                .with_target(false)
                .without_time()
                .json();

            let subscriber = tracing_subscriber::Registry::default()
                .with(tracing_subscriber::filter::LevelFilter::from_level(level))
                .with(fmt_layer)
                .with(journal_layer);

            let _ = tracing::subscriber::set_global_default(subscriber);
        }
        LogFormat::Text => {
            let fmt_layer = fmt::layer()
                .with_writer(writer)
                .with_target(false)
                .without_time()
                .with_ansi(false)
                .compact();

            let subscriber = tracing_subscriber::Registry::default()
                .with(tracing_subscriber::filter::LevelFilter::from_level(level))
                .with(fmt_layer)
                .with(journal_layer);

            let _ = tracing::subscriber::set_global_default(subscriber);
        }
    });
}

/// Create a journald layer if JOURNAL_STREAM is set (systemd-launched process).
///
/// HIS-063, HIS-064, HIS-065: when running as a systemd service,
/// Go relies on stderr being captured by the journal, but a direct
/// journald layer gives structured fields and avoids double-logging.
fn journal_layer() -> Option<tracing_journald::Layer> {
    std::env::var_os("JOURNAL_STREAM")?;

    tracing_journald::layer()
        .inspect_err(|error| {
            eprintln!("journald layer unavailable: {error}");
        })
        .ok()
}

impl RuntimeLogSink {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match &mut self.target {
            SinkTarget::Plain(file) => file.write_all(buf),
            SinkTarget::Rotating(file) => file.write_all(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.target {
            SinkTarget::Plain(file) => file.flush(),
            SinkTarget::Rotating(file) => file.flush(),
        }
    }
}

impl RotatingFile {
    fn new(config: RollingConfig) -> io::Result<Self> {
        let path = rolling_log_path(&config);
        let file = open_log_file(&path)?;

        Ok(Self { config, path, file })
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.rotate_if_needed(buf.len())?;
        self.file.write_all(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }

    fn rotate_if_needed(&mut self, incoming_len: usize) -> io::Result<()> {
        if !rotation_needed(self.file.metadata()?.len(), incoming_len, self.config.max_size_mb) {
            return Ok(());
        }

        self.rotate_backups()?;
        self.file = open_log_file(&self.path)?;

        Ok(())
    }

    fn rotate_backups(&self) -> io::Result<()> {
        if self.config.max_backups == 0 {
            remove_if_exists(&self.path)?;
            return Ok(());
        }

        remove_if_exists(&backup_path(&self.path, self.config.max_backups))?;

        for index in (1..self.config.max_backups).rev() {
            let source = backup_path(&self.path, index);
            let destination = backup_path(&self.path, index + 1);

            if source.exists() {
                fs::rename(source, destination)?;
            }
        }

        if self.path.exists() {
            fs::rename(&self.path, backup_path(&self.path, 1))?;
        }

        self.prune_old_backups()
    }

    fn prune_old_backups(&self) -> io::Result<()> {
        if self.config.max_age_days == 0 {
            return Ok(());
        }

        let max_age = Duration::from_secs(u64::from(self.config.max_age_days) * SECS_PER_DAY);
        let now = SystemTime::now();

        for index in 1..=self.config.max_backups {
            let path = backup_path(&self.path, index);
            let Ok(metadata) = fs::metadata(&path) else {
                continue;
            };
            let Ok(modified) = metadata.modified() else {
                continue;
            };
            let Ok(age) = now.duration_since(modified) else {
                continue;
            };

            if age > max_age {
                remove_if_exists(&path)?;
            }
        }

        Ok(())
    }
}

fn open_runtime_log_sink(log_config: &LogConfig) -> io::Result<Option<RuntimeLogSink>> {
    if let Some(file) = log_config.file.as_ref() {
        return Ok(Some(RuntimeLogSink {
            target: SinkTarget::Plain(open_log_file(&file.full_path())?),
        }));
    }

    if let Some(rolling) = log_config.rolling.as_ref() {
        return Ok(Some(RuntimeLogSink {
            target: SinkTarget::Rotating(RotatingFile::new(rolling.clone())?),
        }));
    }

    Ok(None)
}

#[cfg(test)]
fn runtime_log_path(log_config: &LogConfig) -> Option<PathBuf> {
    if let Some(file) = log_config.file.as_ref() {
        return Some(file.full_path());
    }

    log_config.rolling.as_ref().map(rolling_log_path)
}

fn rolling_log_path(config: &RollingConfig) -> PathBuf {
    config.dirname.join(&config.filename)
}

fn open_log_file(path: &Path) -> io::Result<File> {
    ensure_parent_directory(path)?;

    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .mode(LOG_FILE_PERM_MODE)
        .open(path)?;

    let permissions = fs::Permissions::from_mode(LOG_FILE_PERM_MODE);
    fs::set_permissions(path, permissions)?;

    Ok(file)
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

fn backup_path(path: &Path, index: u32) -> PathBuf {
    PathBuf::from(format!("{}.{}", path.display(), index))
}

fn remove_if_exists(path: &Path) -> io::Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }

    Ok(())
}

fn rotation_needed(current_size: u64, incoming_len: usize, max_size_mb: u32) -> bool {
    current_size + incoming_len as u64 > u64::from(max_size_mb) * 1024 * 1024
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
            file: Some(cfdrs_shared::FileConfig {
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

    #[test]
    fn backup_paths_use_numeric_suffixes() {
        assert_eq!(
            backup_path(Path::new("/tmp/cloudflared.log"), 2),
            PathBuf::from("/tmp/cloudflared.log.2")
        );
    }

    #[test]
    fn rotation_threshold_uses_megabyte_limit() {
        assert!(!rotation_needed(64, 128, 1));
        assert!(rotation_needed(1024 * 1024, 1, 1));
    }

    #[test]
    fn rotating_sink_creates_first_backup_after_limit() {
        let root = std::env::temp_dir().join(format!("cfdrs-runtime-rotation-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp dir should exist");

        let mut sink = RotatingFile::new(RollingConfig {
            dirname: root.clone(),
            filename: "cloudflared.log".to_owned(),
            max_size_mb: 1,
            max_backups: 2,
            max_age_days: 0,
        })
        .expect("rotating file should open");

        sink.write_all(&vec![b'a'; 1024 * 1024])
            .expect("first write should succeed");
        sink.write_all(b"b").expect("second write should rotate");

        assert!(root.join("cloudflared.log").exists());
        assert!(root.join("cloudflared.log.1").exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rotating_sink_enforces_max_backups_limit() {
        let root = std::env::temp_dir().join(format!("cfdrs-backup-limit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp dir should exist");

        let mut sink = RotatingFile::new(RollingConfig {
            dirname: root.clone(),
            filename: "cloudflared.log".to_owned(),
            max_size_mb: 1,
            max_backups: 3,
            max_age_days: 0,
        })
        .expect("rotating file should open");

        // Trigger 4 rotations — with max_backups=3, oldest is pruned.
        let payload = vec![b'x'; 1024 * 1024];

        for _ in 0..4 {
            sink.write_all(&payload).expect("write should succeed");
            sink.write_all(b"y").expect("overflow write should rotate");
        }

        assert!(root.join("cloudflared.log").exists());
        assert!(root.join("cloudflared.log.1").exists());
        assert!(root.join("cloudflared.log.2").exists());
        assert!(root.join("cloudflared.log.3").exists());
        assert!(
            !root.join("cloudflared.log.4").exists(),
            "backup count should not exceed max_backups=3"
        );

        let _ = fs::remove_dir_all(root);
    }

    // --- HIS-065: backup naming is numeric, not lumberjack timestamps ---

    #[test]
    fn backup_naming_uses_numeric_suffixes_not_lumberjack_timestamps() {
        // Go lumberjack: cloudflared-2024-01-15T10-30-00.000.log
        // Rust: cloudflared.log.1, cloudflared.log.2, ...
        // Intentional local divergence — backup filenames are not sent
        // upstream. Rotation behavior (size, count, age) matches Go.
        assert_eq!(
            backup_path(Path::new("/var/log/cloudflared/cloudflared.log"), 1),
            PathBuf::from("/var/log/cloudflared/cloudflared.log.1")
        );
        assert_eq!(
            backup_path(Path::new("/var/log/cloudflared/cloudflared.log"), 5),
            PathBuf::from("/var/log/cloudflared/cloudflared.log.5")
        );
    }

    // --- HIS-063: --logfile takes precedence over --log-directory ---

    #[test]
    fn runtime_log_path_returns_none_when_no_file_config() {
        let config = LogConfig::default();
        assert!(runtime_log_path(&config).is_none());
    }

    // --- HIS-064: --log-directory defaults ---

    #[test]
    fn rolling_log_path_joins_dirname_and_filename() {
        let config = RollingConfig {
            dirname: PathBuf::from("/var/log/cloudflared"),
            filename: "cloudflared.log".into(),
            ..RollingConfig::default()
        };
        assert_eq!(
            rolling_log_path(&config),
            PathBuf::from("/var/log/cloudflared/cloudflared.log")
        );
    }

    // --- HIS-065: rotation defaults match Go lumberjack ---

    #[test]
    fn rotation_not_needed_when_under_limit() {
        // 512KB current + 1 byte incoming < 1MB limit
        assert!(!rotation_needed(512 * 1024, 1, 1));
    }

    #[test]
    fn rotation_needed_at_exact_boundary() {
        // 1MB current + 1 byte incoming > 1MB limit
        assert!(rotation_needed(1024 * 1024, 1, 1));
    }

    // --- HIS-066: file permissions ---

    #[test]
    fn open_log_file_creates_parent_directory() {
        let root = std::env::temp_dir().join(format!("cfdrs-logfile-parent-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);

        let log_path = root.join("subdir").join("cloudflared.log");
        let _file = open_log_file(&log_path).expect("should create parent and file");

        assert!(log_path.exists());
        assert!(root.join("subdir").exists());

        // Verify file permission is 0644
        let metadata = fs::metadata(&log_path).expect("metadata");
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o644, "file should have 0644 permissions");

        // Verify parent directory permission is 0744
        let dir_metadata = fs::metadata(root.join("subdir")).expect("dir metadata");
        let dir_mode = dir_metadata.permissions().mode() & 0o777;
        assert_eq!(dir_mode, 0o744, "directory should have 0744 permissions");

        let _ = fs::remove_dir_all(root);
    }

    // --- HIS-068: transport level widening ---

    #[test]
    fn resolve_global_level_uses_app_level_when_transport_is_less_verbose() {
        // App=Info, Transport=Error → use Info (more verbose)
        assert_eq!(
            resolve_global_level(LogLevel::Info, Some(LogLevel::Error)),
            Level::INFO
        );
    }

    #[test]
    fn resolve_global_level_widens_to_transport_when_more_verbose() {
        // App=Error, Transport=Debug → use Debug (more verbose)
        assert_eq!(
            resolve_global_level(LogLevel::Error, Some(LogLevel::Debug)),
            Level::DEBUG
        );
    }

    #[test]
    fn resolve_global_level_uses_app_when_transport_absent() {
        assert_eq!(resolve_global_level(LogLevel::Warn, None), Level::WARN);
    }
}
