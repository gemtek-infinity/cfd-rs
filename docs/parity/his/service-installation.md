# HIS Feature-Group Audit: Service Installation

## Purpose

This document audits Linux service installation and uninstall behavior
against the frozen Go baseline in `baseline-2026.2.0/old-impl/`.

This is a host-interaction surface (HIS) — it creates, modifies, and removes
files on the local filesystem and interacts with the init system.

## Frozen Baseline Source

Primary file: `cmd/cloudflared/linux_service.go`

Supporting files:

- `cmd/cloudflared/common_service.go` — token-based install args
- `postinst.sh` — package manager post-install script
- `postrm.sh` — package manager post-remove script
- `cmd/cloudflared/tunnel/subcommands.go` — token parsing

## CLI Entry Points

```text
cloudflared service install                       # config-based
cloudflared service install --token <TOKEN>        # token-based
cloudflared service install --no-update-service    # skip update timer
cloudflared service uninstall
```

The `service` command is registered in `runApp()` (build-tagged for Linux)
with two subcommands: `install` and `uninstall`.

## Init System Detection

```go
func isSystemd() bool {
    if _, err := os.Stat("/run/systemd/system"); err == nil {
        return true
    }
    return false
}
```

Detection checks `/run/systemd/system` existence. If absent, falls back to
SysV init scripts.

## Install Flow

### Config-Based Install (0 arguments)

1. read user's config file via `config.ReadConfigFile()`
2. validate required keys: `tunnel` and `credentials-file`
3. if user config path differs from `/etc/cloudflared/config.yml` and service
   config already exists, return conflict error
4. if no conflict, copy user config to `/etc/cloudflared/config.yml`
5. build args: `["--config", "/etc/cloudflared/config.yml", "tunnel", "run"]`

### Token-Based Install (1+ arguments)

1. parse tunnel token via `tunnel.ParseToken(token)`
2. build args: `["tunnel", "run", "--token", token]`

### Systemd Install

Files written:

| File | Template |
| --- | --- |
| `/etc/systemd/system/cloudflared.service` | main service unit |
| `/etc/systemd/system/cloudflared-update.service` | update unit (unless `--no-update-service`) |
| `/etc/systemd/system/cloudflared-update.timer` | daily timer (unless `--no-update-service`) |

Commands run in order:

1. `systemctl enable cloudflared.service`
2. `systemctl daemon-reload`
3. `systemctl start cloudflared.service`
4. start update timer (if generated)

### Systemd Unit Templates

**cloudflared.service:**

```ini
[Unit]
Description=cloudflared
After=network-online.target
Wants=network-online.target

[Service]
TimeoutStartSec=15
Type=notify
ExecStart={{ .Path }} --no-autoupdate{{ range .ExtraArgs }} {{ . }}{{ end }}
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

**cloudflared-update.service:**

```ini
[Unit]
Description=Update cloudflared
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=/bin/bash -c '{{ .Path }} update; code=$?; if [ $code -eq 11 ]; then systemctl restart cloudflared; exit 0; fi; exit $code'
```

**cloudflared-update.timer:**

```ini
[Unit]
Description=Update cloudflared

[Timer]
OnCalendar=daily

[Install]
WantedBy=timers.target
```

### SysV Install

Files written:

| File | Type |
| --- | --- |
| `/etc/init.d/cloudflared` | init script from template |
| `/etc/rc{2,3,4,5}.d/S50et` | start symlinks → `/etc/init.d/cloudflared` |
| `/etc/rc{0,1,6}.d/K02et` | stop symlinks → `/etc/init.d/cloudflared` |

Commands run:

1. `service cloudflared start`

The SysV init script supports `start`, `stop`, `restart`, `status` actions.
It creates a pidfile at `/var/run/$name.pid`, writes stdout to
`/var/log/$name.log` and stderr to `/var/log/$name.err`, and optionally
sources `/etc/sysconfig/$name` for environment overrides.

## Uninstall Flow

### Systemd Uninstall

1. `systemctl disable cloudflared.service`
2. `systemctl stop cloudflared.service`
3. stop update timer if installed
4. remove all service template files
5. `systemctl daemon-reload`

### SysV Uninstall

1. `service cloudflared stop`
2. remove `/etc/init.d/cloudflared`
3. remove all symlinks from `/etc/rc*.d/`

### Preservation

- config file at `/etc/cloudflared/config.yml` is NOT deleted
- credential files are NOT deleted
- user-created files are NOT touched

## Package Manager Integration

### postinst.sh (runs after package install)

```bash
#!/bin/bash
set -eu
ln -sf /usr/bin/cloudflared /usr/local/bin/cloudflared
mkdir -p /usr/local/etc/cloudflared/
touch /usr/local/etc/cloudflared/.installedFromPackageManager || true
```

### postrm.sh (runs after package uninstall)

```bash
#!/bin/bash
set -eu
rm -f /usr/local/bin/cloudflared
rm -f /usr/local/etc/cloudflared/.installedFromPackageManager
```

The `.installedFromPackageManager` marker file is used by the updater to
detect package-managed installations and skip self-update.

## Service Config Directory

The service config directory is `/etc/cloudflared/`, distinct from the
user-facing discovery paths. `ensureConfigDirExists()` creates the directory
during install if it does not exist.

## Error Handling

| Condition | Behavior |
| --- | --- |
| service template already exists | error with `serviceAlreadyExistsWarn` message |
| config conflict (different source vs existing service config) | error with remediation guidance |
| config copy fails | error returned |
| required config keys missing (`tunnel:`, `credentials-file:`) | error listing requirements |
| invalid token | validation error |
| template generation failure | error logged and returned |
| systemctl command failure | error with command output |
| uninstall when not installed | warning logged, does not fail |

## Filesystem Side Effects Summary

### Created by install

- `/etc/cloudflared/` directory
- `/etc/cloudflared/config.yml` (copied from user config, config-based only)
- `/etc/systemd/system/cloudflared.service` (systemd)
- `/etc/systemd/system/cloudflared-update.service` (systemd, optional)
- `/etc/systemd/system/cloudflared-update.timer` (systemd, optional)
- `/etc/init.d/cloudflared` (SysV)
- `/etc/rc{2,3,4,5}.d/S50et` symlinks (SysV)
- `/etc/rc{0,1,6}.d/K02et` symlinks (SysV)

### Created by package manager

- `/usr/local/bin/cloudflared` symlink
- `/usr/local/etc/cloudflared/.installedFromPackageManager` marker

### Removed by uninstall

- systemd unit files or SysV init script and symlinks
- does NOT remove config or credentials

### Removed by package uninstall

- `/usr/local/bin/cloudflared` symlink
- `/usr/local/etc/cloudflared/.installedFromPackageManager` marker

## Current Rust State

Service install and uninstall are entirely absent in the current Rust
codebase. The runtime deployment evidence in
`crates/cloudflared-cli/src/runtime/state/deployment_evidence.rs` honestly
declares `no-installer` and `no-systemd-unit` as known gaps.

Systemd detection exists in `crates/cloudflared-cli/src/runtime/deployment.rs`
but only for evidence reporting, not for service management.

## Gap Summary

| Gap | Severity | Notes |
| --- | --- | --- |
| `service install` command absent | critical | lane-required for Linux production-alpha |
| `service uninstall` command absent | critical | lane-required |
| systemd unit template generation absent | critical | main service path |
| SysV init script generation absent | high | fallback path |
| config copy to service directory absent | high | install prerequisite |
| config validation for service install absent | high | tunnel + credentials-file check |
| token-based install absent | high | common install path |
| update service and timer absent | medium | depends on updater surface |
| postinst/postrm package scripts not applicable | low | packaging concern, not binary behavior |
| `--no-update-service` flag absent | medium | install option |
