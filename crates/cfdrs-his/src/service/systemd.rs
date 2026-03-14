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

/// Write a systemd unit to `/etc/systemd/system/<name>`.
fn write_unit(name: &str, content: &str) -> Result<()> {
    let path = format!("{SYSTEMD_DIR}/{name}");
    fs::write(&path, content).map_err(|e| ConfigError::write_file(Path::new(&path), e))?;
    Ok(())
}

/// Remove a systemd unit file.
fn remove_unit(name: &str) -> Result<()> {
    let path = format!("{SYSTEMD_DIR}/{name}");
    if Path::new(&path).exists() {
        fs::remove_file(&path)
            .map_err(|e| ConfigError::invariant(format!("failed to remove {path}: {e}")))?;
    }
    Ok(())
}

/// HIS-014, HIS-015: install systemd units and enable the service.
pub fn install(args: &ServiceTemplateArgs, auto_update: bool, runner: &dyn CommandRunner) -> Result<()> {
    // Write main service unit.
    write_unit(CLOUDFLARED_SERVICE, &render_service_unit(args))?;

    // HIS-018: optionally write update service and timer.
    if auto_update {
        write_unit(CLOUDFLARED_UPDATE_SERVICE, &render_update_service_unit(args))?;
        write_unit(CLOUDFLARED_UPDATE_TIMER, &render_update_timer_unit())?;
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
    // Stop and disable in a best-effort order matching Go.
    let _ = runner.run("systemctl", &["disable", CLOUDFLARED_SERVICE]);
    let _ = runner.run("systemctl", &["stop", CLOUDFLARED_SERVICE]);
    let _ = runner.run("systemctl", &["stop", CLOUDFLARED_UPDATE_TIMER]);

    remove_unit(CLOUDFLARED_SERVICE)?;
    remove_unit(CLOUDFLARED_UPDATE_SERVICE)?;
    remove_unit(CLOUDFLARED_UPDATE_TIMER)?;

    runner.run("systemctl", &["daemon-reload"])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

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
}
