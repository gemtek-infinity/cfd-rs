//! SysV init script generation and install/uninstall.
//!
//! HIS-016, HIS-023 — deferred in roadmap-index to Proof Closure / Command
//! Family Closure. Templates are provided here for completeness but the
//! operational entry points return `ConfigError::deferred`.

use cfdrs_shared::{ConfigError, Result};

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

/// SysV install — deferred per roadmap-index.
pub fn install(_args: &ServiceTemplateArgs, _auto_update: bool, _runner: &dyn CommandRunner) -> Result<()> {
    Err(ConfigError::deferred("service install (SysV)"))
}

/// SysV uninstall — deferred per roadmap-index.
pub fn uninstall(_runner: &dyn CommandRunner) -> Result<()> {
    Err(ConfigError::deferred("service uninstall (SysV)"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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
}
