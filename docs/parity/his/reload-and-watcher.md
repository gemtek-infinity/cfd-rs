# HIS Feature-Group Audit: Reload and Watcher Behavior

## Purpose

This document audits config reload, file watching, and updater behavior
against the frozen Go baseline in [baseline-2026.2.0/](../../../baseline-2026.2.0/).

These surfaces interact with the host filesystem (watching files, restarting
processes) and with external services (update check endpoint).

## Frozen Baseline Source

Primary files:

- [watcher/file.go](../../../baseline-2026.2.0/watcher/file.go) — fsnotify file watcher
- [overwatch/app_manager.go](../../../baseline-2026.2.0/overwatch/app_manager.go) — service lifecycle manager
- [orchestration/orchestrator.go](../../../baseline-2026.2.0/orchestration/orchestrator.go) — remote config update handling
- [cmd/cloudflared/app_service.go](../../../baseline-2026.2.0/cmd/cloudflared/app_service.go) — config update action loop
- [cmd/cloudflared/updater/update.go](../../../baseline-2026.2.0/cmd/cloudflared/updater/update.go) — auto-update logic
- [cmd/cloudflared/updater/workers_service.go](../../../baseline-2026.2.0/cmd/cloudflared/updater/workers_service.go) — update check HTTP client

## File Watcher

### Mechanism

Uses `fsnotify.Watcher` (inotify on Linux):

```go
type File struct {
    watcher  *fsnotify.Watcher
    shutdown chan struct{}
}
```

- `NewFile()` creates the watcher
- `Add(filepath)` registers files for monitoring
- triggers only on `fsnotify.Write` events (ignores CHMOD, REMOVE, etc.)
- `Start(Notification)` runs a blocking event loop
- `Shutdown()` sends a non-blocking shutdown signal

### Notification Interface

```go
type Notification interface {
    WatcherItemDidChange(string)
    WatcherDidError(error)
}
```

Delegates to `ConfigManager` which parses the changed file and forwards
parsed config to `AppService`.

## Config Reload Flow

### Local File Reload

1. `watcher.NewFile()` creates fsnotify watcher
2. `config.NewFileManager()` wraps watcher with config parser
3. `configManager.Start(AppService)` registers the config file
4. file watcher detects Write event
5. calls `AppService.ConfigDidUpdate(config.Root)`
6. posts to `configUpdateChan`
7. `actionLoop()` processes the update:
   - creates/updates services matching new config
   - removes services no longer in config
   - uses content hash for change detection (skip if hash unchanged)

### Service Management on Reload

```go
func (m *AppManager) Add(service Service) {
    if current, ok := m.services[service.Name()]; ok {
        if current.Hash() == service.Hash() {
            return  // no change
        }
        current.Shutdown()  // stop old
    }
    m.services[service.Name()] = service
    go m.serviceRun(service)  // start new
}
```

Service interface:

- `Name() string` — unique identifier
- `Type() string` — service category
- `Hash() string` — content hash for change detection
- `Shutdown()` — graceful stop
- `Run() error` — blocking run loop

### Remote Config Update

The orchestrator handles config pushed from the Cloudflare edge:

1. version check: only apply if `version > currentVersion`
2. parse and validate new config JSON
3. start new origins concurrently (before closing old ones)
4. update flow limiter and origin dialer
5. swap proxy via `atomic.Value` (lock-free reads, locked writes)
6. close old proxy shutdown channel

Atomic guarantees:

- version is monotonically increasing (no downgrade)
- new origins started before old closed (minimize connection drop window)
- proxy replaced atomically
- local warp-routing overrides remote values

## Error and Recovery Semantics

| Condition | Behavior |
| --- | --- |
| file watch error | logged via `WatcherDidError()`, watching continues |
| config parse error | service not added, other services unaffected |
| reload failure | old service remains active (no rollback) |
| shutdown during reload | non-blocking channel handler prevents hang |
| version downgrade attempt | rejected (current version preserved) |

## Updater

### Entry Points

| Path | Trigger |
| --- | --- |
| `cloudflared update` | manual CLI command |
| `AutoUpdater.Run(ctx)` | periodic timer (default 24h) |
| `VersionWarningChecker.StartWarningCheck()` | startup, non-blocking |

### Update Check

HTTP call to `https://update.argotunnel.com` (or staging endpoint):

Request parameters:

- `os`: runtime.GOOS
- `arch`: runtime.GOARCH
- `beta`: bool flag
- `version`: specific version if forced
- `clientVersion`: current running version

### CLI Flags

| Flag | Purpose |
| --- | --- |
| `--beta` | check beta channel |
| `--staging` | use staging update URL |
| `--force` | force update |
| `--version X.Y.Z` | target specific version |
| `--autoupdate-freq DURATION` | check interval (default 24h) |
| `--no-autoupdate` | disable auto-update |

### Auto-Update Restrictions

| Condition | Behavior |
| --- | --- |
| Windows | auto-update disabled |
| running from terminal (`isTerminal()`) | auto-update disabled |
| `.installedFromPackageManager` marker exists | auto-update disabled |
| `BuiltForPackageManager` build tag set | auto-update disabled |

Terminal detection: `term.IsTerminal(int(os.Stdout.Fd()))`

### Exit Codes

| Code | Meaning |
| --- | --- |
| 11 | update successful, restart needed |
| 10 | update failed |
| 0 | no update or normal exit |

### Service Integration

**Systemd**: update service runs `cloudflared update`, and on exit code 11
restarts the main service via `systemctl restart cloudflared`.

**SysV**: process self-restarts via `gracenet.Net.StartProcess()` (fork new
process, old process exits).

## Environment and Privilege Assumptions

### systemd Detection

```go
func isSystemd() bool {
    if _, err := os.Stat("/run/systemd/system"); err == nil {
        return true
    }
    return false
}
```

### UID Detection

`os.Getuid()` used in diagnostic configuration handler. UID==0 enables
journalctl log extraction.

### Terminal Detection

`term.IsTerminal(int(os.Stdout.Fd()))` determines interactive vs service
mode. Affects auto-update behavior.

### OS-Specific Build Tags

Platform-specific files:

- `linux_service.go` — Linux service commands
- `macos_service.go` — macOS service commands
- `windows_service.go` — Windows service commands
- `system_collector_linux.go` — Linux system info
- `system_collector_macos.go` — macOS system info
- `collector_unix.go` — Unix network diagnostics
- `collector_windows.go` — Windows network diagnostics

## Current Rust State

### What exists

- systemd detection via environment variables (`INVOCATION_ID`,
  `NOTIFY_SOCKET`, `JOURNAL_STREAM`) in deployment evidence
- explicit declaration that config reload is not supported
- SIGHUP detection exists but handler returns "not supported" error
- restart budget tracking in failure evidence
- SIGTERM/SIGINT signal handling via `tokio::signal::unix` in
    [crates/cfdrs-bin/src/runtime/tasks/bridges.rs](../../../crates/cfdrs-bin/src/runtime/tasks/bridges.rs): sends `RuntimeCommand::ShutdownRequested`
  with signal name, conditionally enabled (disabled in tests)
- graceful shutdown with child task draining in [crates/cfdrs-bin/src/runtime/tasks/drain.rs](../../../crates/cfdrs-bin/src/runtime/tasks/drain.rs):
  waits for children with grace period, aborts timed-out tasks
- runtime shutdown grace period defaulting to 30s via cfdrs-his and accepting parsed `--grace-period` overrides
- pidfile helpers wired on runtime service-ready and cleanup through
        [crates/cfdrs-bin/src/runtime/command_dispatch/handlers.rs](../../../crates/cfdrs-bin/src/runtime/command_dispatch/handlers.rs)
- reload recovery strategy and action handling in [crates/cfdrs-his/src/watcher.rs](../../../crates/cfdrs-his/src/watcher.rs):
    `ReloadActionLoop` keeps the previous config on nonfatal update errors and stops on invariant failures;
    channel-driven `run()` processes actions from `mpsc::Receiver<ReloadAction>` matching Go `actionLoop()` select semantics
- service lifecycle manager in [crates/cfdrs-his/src/watcher.rs](../../../crates/cfdrs-his/src/watcher.rs):
    `Service` trait matching Go `overwatch.Service` (`name`, `service_type`, `hash`, `shutdown`);
    `ServiceManager` with hash-based change detection matching Go `AppManager.Add()` — same hash skips, different hash shuts down old before replacing
- versioned config orchestrator in [crates/cfdrs-his/src/watcher.rs](../../../crates/cfdrs-his/src/watcher.rs):
    `InMemoryConfigOrchestrator` with version-monotonic `update_config(version, config)` matching Go `Orchestrator.UpdateConfig()` — rejects `current_version >= version`, initial version `-1`, returns `UpdateConfigResponse`

### What is missing

- file watcher runtime integration (HIS-041 `NotifyFileWatcher` exists but is not connected to the reload loop in cfdrs-bin)
- auto-update mechanism
- update CLI command
- update check HTTP client
- graceful process restart (fork/exec)
- package manager detection (`.installedFromPackageManager`)
- double-signal immediate shutdown (second signal bypass grace period)
- gracenet socket inheritance for restart

## Signal Handling

### Baseline Behavior

**Source:** [signal/safe_signal.go](../../../baseline-2026.2.0/signal/safe_signal.go), [cmd/cloudflared/tunnel/signal.go](../../../baseline-2026.2.0/cmd/cloudflared/tunnel/signal.go)

The frozen Go baseline listens for SIGTERM and SIGINT:

1. first signal closes `graceShutdownC` channel
2. tunnel stops accepting new requests
3. in-progress requests drain for `--grace-period` duration (default 30s)
4. `GracefulShutdown()` is called on HTTP/2 RPC client connections
5. second signal during grace period forces immediate exit

Token lock acquisition in [token/token.go](../../../baseline-2026.2.0/token/token.go) also intercepts SIGINT/SIGTERM to
call `deleteLockFile()` before exit, preventing stale lock files.

### Rust State

Signal handling is implemented in [crates/cfdrs-bin/src/runtime/tasks/bridges.rs](../../../crates/cfdrs-bin/src/runtime/tasks/bridges.rs)
using `tokio::signal::unix::signal(SignalKind::terminate())` and `signal(SignalKind::interrupt())`.

Received signals send `RuntimeCommand::ShutdownRequested` with the signal name.
Signals are conditionally registered (`enable_signals: true`) and disabled in
test harnesses.

**Parity:** Rust uses the 30s default and parsed `--grace-period` values.
The runtime `drain_child_tasks()` waits up to `shutdown_grace_period` then
aborts remaining child tasks, matching Go's wait-or-exit grace pattern.
Connection-level `GracefulShutdown()` RPC is tracked under CDC-019.

## PID Files

### Baseline Behavior

**Source:** [cmd/cloudflared/tunnel/cmd.go](../../../baseline-2026.2.0/cmd/cloudflared/tunnel/cmd.go)

Optional `--pidfile <path>` flag writes the process PID to the specified file
**after** the tunnel connects (not on startup). Triggered by `connectedSignal`
in a background goroutine via `writePidFile()`.

### Rust State

Runtime service-ready now writes the configured pidfile and removes it during
shutdown using [crates/cfdrs-his/src/signal/](../../../crates/cfdrs-his/src/signal/)
and [crates/cfdrs-bin/src/runtime/command_dispatch/handlers.rs](../../../crates/cfdrs-bin/src/runtime/command_dispatch/handlers.rs).
`ConnectedSignal` one-shot type with `std::sync::Once` matches Go `signal.Signal` + `sync.Once`.
Once-only pidfile guard in `handle_service_ready()` matches Go `writePidFile` timing.

## Token Lock Files

### Baseline Behavior

**Source:** [token/token.go](../../../baseline-2026.2.0/token/token.go)

During token fetch, creates a lock file at `<token-path>.lock` with mode 0600.
If lock exists, polls with exponential backoff (up to 7 iterations). Signal
handlers (SIGINT/SIGTERM) call `deleteLockFile()` to clean up. Lock is deleted
on successful token acquisition or error.

Purpose: prevent concurrent token fetch races (AUTH-1736).

### Rust State

Implemented in [crates/cfdrs-his/src/signal/](../../../crates/cfdrs-his/src/signal/)
with O_EXCL lock creation, signal-safe cleanup helpers, and local tests.

## Process Restart (Gracenet)

### Baseline Behavior

**Source:** [vendor/github.com/facebookgo/grace/gracenet/net.go](../../../baseline-2026.2.0/vendor/github.com/facebookgo/grace/gracenet/net.go), [metrics/metrics.go](../../../baseline-2026.2.0/metrics/metrics.go),
[cmd/cloudflared/updater/update.go](../../../baseline-2026.2.0/cmd/cloudflared/updater/update.go)

The Go baseline uses Facebook's gracenet library for socket inheritance:

- metrics listeners are registered via `gracenet.Net` instead of raw `net.Listen`
- on auto-update restart: `gracenet.Net.StartProcess()` calls `os.StartProcess()`
  inheriting listener file descriptors via `GODEBUGGRACEFUL=1` environment
- new process begins serving immediately on inherited sockets
- old process exits after new process confirms ready

This only applies to SysV (non-systemd) restarts. Under systemd, the service
unit handles restart via `systemctl restart`.

### Rust State

Not implemented. Rust uses async runtime (tokio) without OS-level fork/exec
or socket inheritance. May intentionally diverge since systemd service restart
is the primary Linux path.

## Lane Classification

| Surface | Lane-required | Notes |
| --- | --- | --- |
| local config file watching | yes | operator expects config reload |
| config reload action loop | yes | reload is observable behavior |
| remote config update from edge | yes | Cloudflare-pushed config changes |
| service lifecycle manager | yes | reload depends on it |
| `cloudflared update` command | yes | operator self-update path |
| auto-update timer | yes | service auto-update |
| terminal detection | yes | affects update behavior |
| SIGTERM/SIGINT shutdown | yes | already implemented |
| `--grace-period` flag (30s default) | yes | critical shutdown behavior |
| `--pidfile` flag | medium | optional systemd/service integration |
| token lock file | yes | prevents concurrent token fetch |
| gracenet socket inheritance | medium | SysV restart path |
| double-signal immediate exit | medium | operator escape hatch |
| package manager detection | medium | prevents conflicting updates |
| UID detection | medium | diagnostic privilege behavior |
| graceful process restart | medium | SysV update path |

## Gap Summary

| Gap | Severity | Notes |
| --- | --- | --- |
| file watcher runtime integration absent | critical | `NotifyFileWatcher` exists but not wired to reload loop |
| config reload flow absent | critical | operator-expected behavior |
| remote config update handler absent | critical | edge-pushed config |
| `update` CLI command absent | high | operator self-update |
| auto-update mechanism absent | high | service auto-update |
| update check HTTP client absent | high | update endpoint |
| graceful restart absent | medium | SysV update path |
| gracenet socket inheritance absent | medium | SysV restart path |
