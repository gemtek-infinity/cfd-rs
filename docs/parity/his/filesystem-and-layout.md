# HIS Feature-Group Audit: Filesystem and Layout

## Purpose

This document audits filesystem path expectations, config discovery, credential
lookup, and host-path assumptions against the frozen Go baseline in
[baseline-2026.2.0/](../../../baseline-2026.2.0/).

These paths define the interface between cloudflared and the host filesystem.
Operators and deployment tooling depend on these paths being predictable.

## Frozen Baseline Source

Primary files:

- [config/configuration.go](../../../baseline-2026.2.0/config/configuration.go) — config discovery, default paths, search order
- [credentials/origin_cert.go](../../../baseline-2026.2.0/credentials/origin_cert.go) — origin cert lookup
- [credentials/credentials.go](../../../baseline-2026.2.0/credentials/credentials.go) — tunnel credential types
- [cmd/cloudflared/tunnel/credential_finder.go](../../../baseline-2026.2.0/cmd/cloudflared/tunnel/credential_finder.go) — credential search logic
- [cmd/cloudflared/linux_service.go](../../../baseline-2026.2.0/cmd/cloudflared/linux_service.go) — service directory constants

## Config Discovery Search Order

The frozen baseline searches these directories in order, checking for both
`config.yml` and `config.yaml` in each:

| Priority | Directory | Category |
| --- | --- | --- |
| 1 | `~/.cloudflared/` | user home |
| 2 | `~/.cloudflare-warp/` | legacy user home |
| 3 | `~/cloudflare-warp/` | legacy user home |
| 4 | `/etc/cloudflared/` | system |
| 5 | `/usr/local/etc/cloudflared/` | primary Unix system |

`FindDefaultConfigPath()` returns the first match. If no config is found,
`FindOrCreateConfigPath()` creates a minimal config at the primary path.

## Default Path Constants

| Constant | Value | Source |
| --- | --- | --- |
| `DefaultUnixConfigLocation` | `/usr/local/etc/cloudflared` | `config/configuration.go` |
| `DefaultUnixLogLocation` | `/var/log/cloudflared` | `config/configuration.go` |
| `DefaultConfigFiles` | `["config.yml", "config.yaml"]` | `config/configuration.go` |
| `serviceConfigDir` | `/etc/cloudflared` | `linux_service.go` |
| `serviceConfigFile` | `config.yml` | `linux_service.go` |
| `serviceCredentialFile` | `cert.pem` | `linux_service.go` |
| `serviceConfigPath` | `/etc/cloudflared/config.yml` | `linux_service.go` |

## Config Auto-Create Behavior

When `FindOrCreateConfigPath()` finds no config:

1. creates parent directory of `DefaultConfigPath()` via `mkdir -p`
2. creates the config file at `/usr/local/etc/cloudflared/config.yml`
3. creates log directory at `/var/log/cloudflared` via `mkdir -p`
4. writes minimal YAML with `logDirectory: /var/log/cloudflared`

## Config File Loading

`ReadConfigFile()`:

1. checks `--config` CLI flag first
2. if not set, finds config via discovery
3. opens and YAML-decodes the file
4. empty files are handled gracefully (returns empty config)
5. re-opens in strict mode to detect unknown fields and emit warnings

## Credential File Lookup

### Origin Certificate

`FindDefaultOriginCertPath()` searches the same directories as config for
`cert.pem`. Returns the first match.

`FindOriginCert(path)` validates that the cert exists at the given path and
expands `~` via `homedir.Expand()`.

### Origin Cert Format

PEM-encoded JSON block:

```text
-----BEGIN ARGO TUNNEL TOKEN-----
<base64-encoded JSON>
-----END ARGO TUNNEL TOKEN-----
```

JSON payload:

```json
{
  "zoneID": "...",
  "accountID": "...",
  "apiToken": "...",
  "endpoint": "..."
}
```

### Tunnel Credentials

`CredFinder` interface with two implementations:

1. **staticPath** — user-specified via `--credentials-file` flag
2. **searchByID** — auto-search for `{TunnelID}.json` in:
   - `dirname(origincert)` (if `--origincert` set)
   - each directory in `DefaultConfigSearchDirectories()`

Credentials are written with file mode `0400` (read-only owner) after
`tunnel create`.

### Tunnel Credentials JSON Format

```json
{
  "AccountTag": "...",
  "TunnelSecret": "<base64>",
  "TunnelID": "<uuid>",
  "Endpoint": "..."
}
```

### Tunnel Token Format (compact, for `--token` flag)

```json
{
  "a": "<account-tag>",
  "s": "<base64-secret>",
  "t": "<uuid>",
  "e": "<endpoint>"
}
```

Base64-encoded for transport.

## Logging File Artifacts

### Baseline Behavior

**Source:** [logger/create.go](../../../baseline-2026.2.0/logger/create.go), [logger/configuration.go](../../../baseline-2026.2.0/logger/configuration.go)

The frozen Go baseline supports three log output modes:

| Mode | Target | Flags |
| --- | --- | --- |
| console | stderr (default) | — |
| file | specified path | `--logfile PATH` |
| rolling | directory with rotation | `--log-directory DIR` |

**Rolling rotation** uses lumberjack.v2:

- MaxSize: 1 MB per file
- MaxBackups: 5 retained files
- MaxAge: 0 (no age limit, backups kept forever)

#### File permissions

- log files: mode 0644 (rw-r--r--)
- log directories: mode 0744 (rwxr--r--)

#### Log format flags

- `--loglevel LEVEL` — default `info`, controls global level
- `--transport-loglevel LEVEL` — separate transport layer level
- `--log-format-output FORMAT` — JSON or text output format
- `--log-level-ssh` — SSH server log level (deprecated feature context)

### Rust State

Config discovery creates the log directory (`/var/log/cloudflared`) via
`fs::create_dir_all()`. The `log_directory` config field is parsed and honored
during auto-create. However, no file-based log writer or rolling rotation
exists — the runtime writes exclusively to stderr via `tracing_subscriber::fmt()`.

No `--logfile`, `--loglevel`, `--transport-loglevel`, or `--log-format-output`
CLI flags are exposed.

## Token Lock Files

### Baseline Behavior

**Source:** [token/token.go](../../../baseline-2026.2.0/token/token.go)

During token acquisition, the Go baseline creates a lock file at
`<token-path>.lock` with mode 0600. The lock prevents concurrent token
fetch races across multiple processes (AUTH-1736). If the lock file exists,
the process polls with exponential backoff for up to 7 iterations. SIGINT
and SIGTERM handlers call `deleteLockFile()` to clean up stale locks.

### Lock File Path

The lock file is co-located with the token file in the credentials directory,
typically `~/.cloudflared/<token>.lock`.

### Rust State

Not implemented. No file-based token locking exists in any crate.

## Complete Filesystem Path Inventory

### Directories

| Path | Purpose | Created By |
| --- | --- | --- |
| `/usr/local/etc/cloudflared/` | primary Unix config directory | config auto-create, postinst.sh |
| `/etc/cloudflared/` | service config directory | service install |
| `/var/log/cloudflared/` | default log directory | config auto-create |
| `~/.cloudflared/` | user config and credentials | user-created |

### Files

| Path | Purpose | Created By |
| --- | --- | --- |
| `/usr/local/etc/cloudflared/config.yml` | primary default config | config auto-create |
| `/etc/cloudflared/config.yml` | service config | service install copy |
| `~/.cloudflared/cert.pem` | user origin cert | `cloudflared login` |
| `~/.cloudflared/{uuid}.json` | tunnel credentials | `cloudflared tunnel create` |
| `~/.cloudflared/<token>.lock` | token fetch lock | token acquisition |
| `/usr/local/etc/cloudflared/.installedFromPackageManager` | package marker | postinst.sh |
| `/usr/local/bin/cloudflared` | binary symlink | postinst.sh |
| `/var/run/$name.pid` | SysV pidfile | init script |
| `/var/log/$name.log` | SysV stdout log | init script |
| `/var/log/$name.err` | SysV stderr log | init script |
| `/var/log/cloudflared/cloudflared.log` | rolling log file | logger (file/rolling mode) |
| `<user-specified>.pid` | PID file (`--pidfile`) | tunnel after connect |

### Systemd Unit Files

| Path | Purpose |
| --- | --- |
| `/etc/systemd/system/cloudflared.service` | main service |
| `/etc/systemd/system/cloudflared-update.service` | update unit |
| `/etc/systemd/system/cloudflared-update.timer` | daily update timer |
| `/run/systemd/system` | systemd detection probe |

### Init Script Files

| Path | Purpose |
| --- | --- |
| `/etc/init.d/cloudflared` | SysV init script |
| `/etc/rc{2,3,4,5}.d/S50et` | start symlinks |
| `/etc/rc{0,1,6}.d/K02et` | stop symlinks |
| `/etc/sysconfig/cloudflared` | optional SysV env overrides |

## Current Rust State

### What exists

- config discovery search order matches frozen baseline exactly (5 directories,
  2 filenames, same priority)
- config auto-create behavior implemented with correct paths and minimal YAML
- credential search-by-ID across discovery directories plus origin-cert-dir
  precedence
- origin cert search across default discovery directories
- origin cert PEM parsing implemented (ARGO TUNNEL TOKEN blocks)
- tunnel token compact format parsing for `--token`
- tunnel credentials JSON parsing implemented (all 4 fields)
- credential file creation with mode `0400`
- token lock file creation, exponential-backoff cleanup, and signal-safe
  removal
- pidfile write/remove helpers with `~` expansion and connected-signal timing
- service config directory handling plus service-install filesystem writes
  (systemd and SysV) in the owned HIS service surface
- local logging filesystem surface: `--logfile`, `--log-directory`, rolling
  rotation, default `/var/log/cloudflared`, and Go-matching permission modes
- default path constants match: `/usr/local/etc/cloudflared/config.yml` and
  `/var/log/cloudflared`

### What is missing

- package-manager shell-script behavior (`postinst` / `postrm`) remains
  non-lane and outside the Rust binary surface
- repo-owned preview artifacts still do not provide package-manager installers
  or package scripts; operators use the binary plus `service install`

## Lane Classification

| Path or Behavior | Lane-required | Notes |
| --- | --- | --- |
| config discovery search order | yes | already implemented |
| config auto-create | yes | already implemented |
| credential JSON parsing | yes | already implemented |
| origin cert PEM parsing | yes | already implemented |
| credential search-by-ID | yes | needed for `tunnel run` without `--credentials-file` |
| origin cert search across directories | yes | needed for login-based flows |
| service config directory (`/etc/cloudflared/`) | yes | service install prerequisite |
| systemd unit file paths | yes | service install |
| SysV init script paths | yes | fallback service install |
| log directory creation | yes | auto-create dependency |
| log file creation (`--logfile`) | yes | operator-expected log output |
| rolling log rotation | yes | production log management |
| `--loglevel` flag | yes | operator observability control |
| token lock file | yes | concurrent fetch safety |
| PID file (`--pidfile`) | medium | optional service integration |
| `~/.cloudflared/` for user credentials | yes | user credential storage |
| package manager paths | deployment | not binary behavior |

## Gap Summary

| Gap | Severity | Notes |
| --- | --- | --- |
| package-manager shell scripts | low | non-lane package-manager cleanup/install behavior lives outside the Rust binary |
| package-managed artifact delivery | deployment | preview tarballs exist, but deb/rpm-style installers remain outside this repo slice |
