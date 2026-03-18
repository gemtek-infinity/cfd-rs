//! SysV init script generation and install/uninstall.
//!
//! HIS-016, HIS-023 — deferred in roadmap-index to Proof Closure / Command
//! Family Closure. Templates are provided here for completeness but the
//! operational entry points return `ConfigError::deferred`.

use cfdrs_shared::{ConfigError, Result};
use std::env;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

const SYSV_ROOT_ENV: &str = "CFDRS_SYSV_ROOT";
const START_RUNLEVELS: &[&str] = &["2", "3", "4", "5"];
const STOP_RUNLEVELS: &[&str] = &["0", "1", "6"];
const START_TARGET: &str = "S50et";
const STOP_TARGET: &str = "K02et";
const AUTO_UPDATE_ARG: &str = "--autoupdate-freq 24h0m0s";
const NO_AUTO_UPDATE_ARG: &str = "--no-autoupdate";

#[cfg(test)]
static SYSV_ROOT_OVERRIDE: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);
#[cfg(test)]
static SYSV_TEST_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

use super::{CommandRunner, ServiceTemplateArgs};

/// HIS-023: paths used by the SysV init script.
pub const SYSV_INIT_PATH: &str = "/etc/init.d/cloudflared";

/// Render the SysV init script.
///
/// Provided for template parity; not invoked from the operational path yet.
pub fn render_init_script(args: &ServiceTemplateArgs) -> String {
    let path = args.path.display();
    let extra = if args.extra_args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.extra_args.join(" "))
    };

    format!(
        r##"#!/bin/sh
# For RedHat and cousins:
# chkconfig: 2345 99 01
# description: cloudflared
# processname: {path}
### BEGIN INIT INFO
# Provides:          {path}
# Required-Start:
# Required-Stop:
# Default-Start:     2 3 4 5
# Default-Stop:      0 1 6
# Short-Description: cloudflared
# Description:       cloudflared agent
### END INIT INFO
name=$(basename $(readlink -f $0))
cmd="{path} --pidfile /var/run/$name.pid{extra}"
pid_file="/var/run/$name.pid"
stdout_log="/var/log/$name.log"
stderr_log="/var/log/$name.err"
[ -e /etc/sysconfig/$name ] && . /etc/sysconfig/$name
get_pid() {{
    cat "$pid_file"
}}
is_running() {{
    [ -f "$pid_file" ] && ps $(get_pid) > /dev/null 2>&1
}}
case "$1" in
    start)
        if is_running; then
            echo "Already started"
        else
            echo "Starting $name"
            $cmd >> "$stdout_log" 2>> "$stderr_log" &
            echo $! > "$pid_file"
        fi
    ;;
    stop)
        if is_running; then
            echo -n "Stopping $name.."
            kill $(get_pid)
            for i in {{1..10}}
            do
                if ! is_running; then
                    break
                fi
                echo -n "."
                sleep 1
            done
            echo
            if is_running; then
                echo "Not stopped; may still be shutting down or shutdown may have failed"
                exit 1
            else
                echo "Stopped"
                if [ -f "$pid_file" ]; then
                    rm "$pid_file"
                fi
            fi
        else
            echo "Not running"
        fi
    ;;
    restart)
        $0 stop
        if is_running; then
            echo "Unable to stop, will not attempt to start"
            exit 1
        fi
        $0 start
    ;;
    status)
        if is_running; then
            echo "Running"
        else
            echo "Stopped"
            exit 1
        fi
    ;;
    *)
    echo "Usage: $0 {{start|stop|restart|status}}"
    exit 1
    ;;
esac
exit 0
"##
    )
}

fn sysv_root() -> PathBuf {
    #[cfg(test)]
    {
        if let Some(root) = SYSV_ROOT_OVERRIDE
            .lock()
            .expect("sysv test override lock should not be poisoned")
            .as_ref()
            .cloned()
        {
            return root;
        }
    }

    env::var(SYSV_ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

fn init_script_path() -> PathBuf {
    sysv_root().join("etc").join("init.d").join("cloudflared")
}

fn rc_dir(level: &str) -> PathBuf {
    sysv_root().join("etc").join(format!("rc{}.d", level))
}

fn create_rc_symlink(target: &Path, level: &str, name: &str) -> Result<()> {
    let dir = rc_dir(level);
    fs::create_dir_all(&dir).map_err(|e| ConfigError::create_directory(&dir, e))?;
    let link = dir.join(name);
    let _ = fs::remove_file(&link);
    symlink(target, &link)
        .map_err(|e| ConfigError::invariant(format!("failed to create {}: {e}", link.display())))?;
    Ok(())
}

fn remove_rc_symlink(level: &str, name: &str) {
    let link = rc_dir(level).join(name);
    let _ = fs::remove_file(link);
}

/// HIS-016: install SysV init script, create runlevel symlinks, and start.
pub fn install(args: &ServiceTemplateArgs, auto_update: bool, runner: &dyn CommandRunner) -> Result<()> {
    let mut template_args = args.clone();
    let update_arg = if auto_update {
        AUTO_UPDATE_ARG
    } else {
        NO_AUTO_UPDATE_ARG
    };
    template_args.extra_args.insert(0, update_arg.to_string());

    let script = render_init_script(&template_args);
    let script_path = init_script_path();

    if let Some(parent) = script_path.parent() {
        fs::create_dir_all(parent).map_err(|e| ConfigError::create_directory(parent, e))?;
    }
    fs::write(&script_path, script).map_err(|e| ConfigError::write_file(&script_path, e))?;
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))
        .map_err(|e| ConfigError::invariant(format!("failed to chmod {}: {e}", script_path.display())))?;

    for level in START_RUNLEVELS {
        create_rc_symlink(&script_path, level, START_TARGET)?;
    }
    for level in STOP_RUNLEVELS {
        create_rc_symlink(&script_path, level, STOP_TARGET)?;
    }

    runner.run("service", &["cloudflared", "start"])?;
    Ok(())
}

/// HIS-023: uninstall SysV init script and remove runlevel symlinks.
pub fn uninstall(runner: &dyn CommandRunner) -> Result<()> {
    runner.run("service", &["cloudflared", "stop"])?;
    remove_rc_symlinks();

    let script_path = init_script_path();
    if script_path.exists() {
        fs::remove_file(&script_path).map_err(|e| {
            ConfigError::invariant(format!("failed to remove {}: {e}", script_path.display()))
        })?;
    }

    Ok(())
}

fn remove_rc_symlinks() {
    for level in START_RUNLEVELS {
        remove_rc_symlink(level, START_TARGET);
    }
    for level in STOP_RUNLEVELS {
        remove_rc_symlink(level, STOP_TARGET);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::{Mutex, MutexGuard};

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn sysv_template_contains_expected_markers() {
        let args = ServiceTemplateArgs {
            path: PathBuf::from("/usr/bin/cloudflared"),
            extra_args: vec![],
        };
        let script = render_init_script(&args);

        assert!(script.contains("#!/bin/sh"));
        assert!(script.contains("chkconfig: 2345 99 01"));
        assert!(script.contains("/var/run/$name.pid"));
        assert!(script.contains("/var/log/$name.log"));
        assert!(script.contains("/var/log/$name.err"));
        assert!(script.contains("/etc/sysconfig/$name"));
        assert!(script.contains("start|stop|restart|status"));
    }

    /// HIS-016/023: SysV template contains the binary path argument.
    #[test]
    fn sysv_template_contains_binary_path() {
        let args = ServiceTemplateArgs {
            path: PathBuf::from("/opt/custom/cloudflared"),
            extra_args: vec!["--token".to_string(), "abc".to_string()],
        };
        let script = render_init_script(&args);
        assert!(script.contains("/opt/custom/cloudflared"));
        assert!(script.contains("--token abc"));
    }

    struct RecordingRunner {
        calls: Mutex<Vec<String>>,
    }

    impl RecordingRunner {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls
                .lock()
                .expect("recording runner lock should not be poisoned")
                .clone()
        }
    }

    impl CommandRunner for RecordingRunner {
        fn run(&self, program: &str, args: &[&str]) -> Result<()> {
            self.calls
                .lock()
                .expect("recording runner lock should not be poisoned")
                .push(format!("{program} {}", args.join(" ")));
            Ok(())
        }
    }

    struct SysvRootGuard {
        _serial: MutexGuard<'static, ()>,
    }

    impl SysvRootGuard {
        fn new(path: &Path) -> Self {
            let serial = SYSV_TEST_GUARD
                .lock()
                .expect("sysv test guard lock should not be poisoned");
            set_sysv_root_override(path);
            Self { _serial: serial }
        }
    }

    impl Drop for SysvRootGuard {
        fn drop(&mut self) {
            clear_sysv_root_override();
        }
    }

    #[cfg(test)]
    fn set_sysv_root_override(path: &Path) {
        let mut override_lock = SYSV_ROOT_OVERRIDE
            .lock()
            .expect("sysv test override lock should not be poisoned");
        *override_lock = Some(path.to_path_buf());
    }

    #[cfg(test)]
    fn clear_sysv_root_override() {
        let mut override_lock = SYSV_ROOT_OVERRIDE
            .lock()
            .expect("sysv test override lock should not be poisoned");
        *override_lock = None;
    }

    #[test]
    fn install_writes_script_and_symlinks() {
        let temp = TempDir::new().expect("tempdir");
        let _guard = SysvRootGuard::new(temp.path());

        let args = ServiceTemplateArgs {
            path: temp.path().join("bin/cloudflared"),
            extra_args: vec![],
        };
        let runner = RecordingRunner::new();

        install(&args, false, &runner).expect("install succeeds");

        let script_path = init_script_path();
        assert!(script_path.exists());
        let contents = fs::read_to_string(&script_path).expect("read script");
        assert!(contents.contains("--no-autoupdate"));

        for level in START_RUNLEVELS {
            assert!(rc_dir(level).join(START_TARGET).exists());
        }
        for level in STOP_RUNLEVELS {
            assert!(rc_dir(level).join(STOP_TARGET).exists());
        }

        assert_eq!(runner.calls(), vec!["service cloudflared start".to_string()]);
    }

    #[test]
    fn install_with_auto_update_sets_autoupdate_arg() {
        let temp = TempDir::new().expect("tempdir");
        let _guard = SysvRootGuard::new(temp.path());

        let args = ServiceTemplateArgs {
            path: temp.path().join("bin/cloudflared"),
            extra_args: vec!["tunnel".to_string(), "run".to_string()],
        };
        let runner = RecordingRunner::new();

        install(&args, true, &runner).expect("install succeeds");

        let script_path = init_script_path();
        assert!(script_path.exists());
        let contents = fs::read_to_string(&script_path).expect("read script");
        assert!(contents.contains("--autoupdate-freq 24h0m0s tunnel run"));
        assert!(!contents.contains("--no-autoupdate"));

        for level in START_RUNLEVELS {
            assert!(rc_dir(level).join(START_TARGET).exists());
        }
        for level in STOP_RUNLEVELS {
            assert!(rc_dir(level).join(STOP_TARGET).exists());
        }

        assert_eq!(runner.calls(), vec!["service cloudflared start".to_string()]);
    }

    #[test]
    fn uninstall_removes_script_and_links() {
        let temp = TempDir::new().expect("tempdir");
        let _guard = SysvRootGuard::new(temp.path());

        let script_path = init_script_path();
        if let Some(parent) = script_path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(&script_path, "binary").expect("write script");

        for level in START_RUNLEVELS {
            create_rc_symlink(&script_path, level, START_TARGET).expect("create start link");
        }
        for level in STOP_RUNLEVELS {
            create_rc_symlink(&script_path, level, STOP_TARGET).expect("create stop link");
        }

        let runner = RecordingRunner::new();
        uninstall(&runner).expect("uninstall succeeds");

        assert!(!script_path.exists());
        for level in START_RUNLEVELS {
            assert!(!rc_dir(level).join(START_TARGET).exists());
        }
        for level in STOP_RUNLEVELS {
            assert!(!rc_dir(level).join(STOP_TARGET).exists());
        }

        assert_eq!(runner.calls(), vec!["service cloudflared stop".to_string()]);
    }
}
