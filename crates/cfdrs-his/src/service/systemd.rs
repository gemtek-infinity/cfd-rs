//! Systemd unit template generation and install/uninstall.
//!
//! Covers HIS-014, HIS-015, HIS-018, HIS-022.

use std::fs;
use std::path::Path;

use cfdrs_shared::{ConfigError, Result};

use super::{CommandRunner, ServiceTemplateArgs};

// --- Unit names ---

pub const CLOUDFLARED_SERVICE: &str = "cloudflared.service";
pub const CLOUDFLARED_UPDATE_SERVICE: &str = "cloudflared-update.service";
pub const CLOUDFLARED_UPDATE_TIMER: &str = "cloudflared-update.timer";

const SYSTEMD_DIR: &str = "/etc/systemd/system";

// --- HIS-022: exact systemd template content ---

/// Render `cloudflared.service`.
///
/// Go template fields: `Type=notify`, `TimeoutStartSec=15`,
/// `Restart=on-failure`, `RestartSec=5s`, `--no-autoupdate`,
/// `After=network-online.target`.
pub fn render_service_unit(args: &ServiceTemplateArgs) -> String {
    let path = args.path.display();
    let extra = if args.extra_args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.extra_args.join(" "))
    };

    format!(
        "[Unit]\nDescription=cloudflared\nAfter=network-online.target\nWants=network-online.target\n\\
         n[Service]\nTimeoutStartSec=15\nType=notify\nExecStart={path} \
         --no-autoupdate{extra}\nRestart=on-failure\nRestartSec=5s\n\n[Install]\nWantedBy=multi-user.target\\
         \
         n"
    )
}

/// Render `cloudflared-update.service`.
///
/// Exit code 11 from `cloudflared update` signals a successful binary
/// replacement that requires a service restart. The shell wrapper maps
/// it to a `systemctl restart` + clean exit so systemd does not treat
/// the update as a failure.
pub fn render_update_service_unit(args: &ServiceTemplateArgs) -> String {
    let path = args.path.display();

    format!(
        "[Unit]\nDescription=Update \
         cloudflared\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nExecStart=/bin/\
         bash -c '{path} update; code=$?; if [ $code -eq 11 ]; then systemctl restart cloudflared; exit 0; \
         fi; exit $code'\n"
    )
}

/// Render `cloudflared-update.timer`.
pub fn render_update_timer_unit() -> String {
    "[Unit]\nDescription=Update \
     cloudflared\n\n[Timer]\nOnCalendar=daily\n\n[Install]\nWantedBy=timers.target\n"
        .to_owned()
}

/// Write a systemd unit to `<dir>/<name>`.
fn write_unit_to(dir: &str, name: &str, content: &str) -> Result<()> {
    let path = format!("{dir}/{name}");
    fs::write(&path, content).map_err(|e| ConfigError::write_file(Path::new(&path), e))?;
    Ok(())
}

/// Remove a systemd unit file from `<dir>/<name>`.
fn remove_unit_from(dir: &str, name: &str) -> Result<()> {
    let path = format!("{dir}/{name}");
    if Path::new(&path).exists() {
        fs::remove_file(&path)
            .map_err(|e| ConfigError::invariant(format!("failed to remove {path}: {e}")))?;
    }
    Ok(())
}

/// HIS-014, HIS-015: install systemd units and enable the service.
pub fn install(args: &ServiceTemplateArgs, auto_update: bool, runner: &dyn CommandRunner) -> Result<()> {
    install_to_dir(SYSTEMD_DIR, args, auto_update, runner)
}

/// Inner install targeting an arbitrary directory (enables testing without
/// root).
fn install_to_dir(
    dir: &str,
    args: &ServiceTemplateArgs,
    auto_update: bool,
    runner: &dyn CommandRunner,
) -> Result<()> {
    // Write main service unit.
    write_unit_to(dir, CLOUDFLARED_SERVICE, &render_service_unit(args))?;

    // HIS-018: optionally write update service and timer.
    if auto_update {
        write_unit_to(dir, CLOUDFLARED_UPDATE_SERVICE, &render_update_service_unit(args))?;
        write_unit_to(dir, CLOUDFLARED_UPDATE_TIMER, &render_update_timer_unit())?;
    }

    runner.run("systemctl", &["enable", CLOUDFLARED_SERVICE])?;

    if auto_update {
        runner.run("systemctl", &["start", CLOUDFLARED_UPDATE_TIMER])?;
    }

    runner.run("systemctl", &["daemon-reload"])?;
    runner.run("systemctl", &["start", CLOUDFLARED_SERVICE])?;

    Ok(())
}

/// HIS-017: uninstall systemd units.
pub fn uninstall(runner: &dyn CommandRunner) -> Result<()> {
    uninstall_from_dir(SYSTEMD_DIR, runner)
}

/// Inner uninstall targeting an arbitrary directory.
fn uninstall_from_dir(dir: &str, runner: &dyn CommandRunner) -> Result<()> {
    // Stop and disable in a best-effort order matching Go.
    let _ = runner.run("systemctl", &["disable", CLOUDFLARED_SERVICE]);
    let _ = runner.run("systemctl", &["stop", CLOUDFLARED_SERVICE]);
    let _ = runner.run("systemctl", &["stop", CLOUDFLARED_UPDATE_TIMER]);

    remove_unit_from(dir, CLOUDFLARED_SERVICE)?;
    remove_unit_from(dir, CLOUDFLARED_UPDATE_SERVICE)?;
    remove_unit_from(dir, CLOUDFLARED_UPDATE_TIMER)?;

    runner.run("systemctl", &["daemon-reload"])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use super::*;

    /// Mock that records every invocation for sequence verification.
    struct RecordingRunner {
        calls: RefCell<Vec<String>>,
    }

    impl RecordingRunner {
        fn new() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl CommandRunner for RecordingRunner {
        fn run(&self, program: &str, args: &[&str]) -> cfdrs_shared::Result<()> {
            self.calls
                .borrow_mut()
                .push(format!("{program} {}", args.join(" ")));
            Ok(())
        }
    }

    fn test_args() -> ServiceTemplateArgs {
        ServiceTemplateArgs {
            path: PathBuf::from("/usr/bin/cloudflared"),
            extra_args: vec![
                "--config".into(),
                "/etc/cloudflared/config.yml".into(),
                "tunnel".into(),
                "run".into(),
            ],
        }
    }

    #[test]
    fn service_unit_matches_go_template() {
        let content = render_service_unit(&test_args());

        assert!(content.contains("Type=notify"));
        assert!(content.contains("TimeoutStartSec=15"));
        assert!(content.contains("Restart=on-failure"));
        assert!(content.contains("RestartSec=5s"));
        assert!(content.contains("--no-autoupdate"));
        assert!(content.contains("After=network-online.target"));
        assert!(content.contains("WantedBy=multi-user.target"));
        assert!(content.contains(
            "/usr/bin/cloudflared --no-autoupdate --config /etc/cloudflared/config.yml tunnel run"
        ));
    }

    #[test]
    fn update_service_contains_restart_logic() {
        let content = render_update_service_unit(&test_args());
        assert!(content.contains("systemctl restart cloudflared"));
        assert!(content.contains("exit 0"));
    }

    #[test]
    fn update_timer_is_daily() {
        let content = render_update_timer_unit();
        assert!(content.contains("OnCalendar=daily"));
        assert!(content.contains("WantedBy=timers.target"));
    }

    /// HIS-015: verify the exact Go baseline systemd command sequence.
    ///
    /// Go order: enable → [optional timer start] → daemon-reload → start
    /// service.
    #[test]
    fn install_with_auto_update_follows_go_sequence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let runner = RecordingRunner::new();

        install_to_dir(dir.path().to_str().expect("utf8"), &test_args(), true, &runner).expect("install");

        assert_eq!(
            runner.calls(),
            vec![
                "systemctl enable cloudflared.service",
                "systemctl start cloudflared-update.timer",
                "systemctl daemon-reload",
                "systemctl start cloudflared.service",
            ]
        );

        // Verify all three unit files were written.
        assert!(dir.path().join(CLOUDFLARED_SERVICE).exists());
        assert!(dir.path().join(CLOUDFLARED_UPDATE_SERVICE).exists());
        assert!(dir.path().join(CLOUDFLARED_UPDATE_TIMER).exists());
    }

    /// HIS-015: without auto_update, timer start is skipped.
    #[test]
    fn install_without_auto_update_skips_timer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let runner = RecordingRunner::new();

        install_to_dir(dir.path().to_str().expect("utf8"), &test_args(), false, &runner).expect("install");

        assert_eq!(
            runner.calls(),
            vec![
                "systemctl enable cloudflared.service",
                "systemctl daemon-reload",
                "systemctl start cloudflared.service",
            ]
        );

        // Only the main service unit should exist.
        assert!(dir.path().join(CLOUDFLARED_SERVICE).exists());
        assert!(!dir.path().join(CLOUDFLARED_UPDATE_SERVICE).exists());
        assert!(!dir.path().join(CLOUDFLARED_UPDATE_TIMER).exists());
    }

    /// HIS-017: uninstall removes unit files and runs daemon-reload.
    #[test]
    fn uninstall_removes_units_and_reloads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let runner = RecordingRunner::new();

        // Pre-create unit files to verify removal.
        for name in [
            CLOUDFLARED_SERVICE,
            CLOUDFLARED_UPDATE_SERVICE,
            CLOUDFLARED_UPDATE_TIMER,
        ] {
            fs::write(dir.path().join(name), "placeholder").expect("write");
        }

        uninstall_from_dir(dir.path().to_str().expect("utf8"), &runner).expect("uninstall");

        assert_eq!(
            runner.calls(),
            vec![
                "systemctl disable cloudflared.service",
                "systemctl stop cloudflared.service",
                "systemctl stop cloudflared-update.timer",
                "systemctl daemon-reload",
            ]
        );

        // All unit files removed.
        assert!(!dir.path().join(CLOUDFLARED_SERVICE).exists());
        assert!(!dir.path().join(CLOUDFLARED_UPDATE_SERVICE).exists());
        assert!(!dir.path().join(CLOUDFLARED_UPDATE_TIMER).exists());
    }
}
