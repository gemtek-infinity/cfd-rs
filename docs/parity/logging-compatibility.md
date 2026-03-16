# Logging Compatibility

This document is the cross-domain preparation contract for logging parity.
It is a production-alpha blocker.

Authoritative rows: `CLI-003`, `CLI-023`, `CLI-024`, `CDC-023`, `CDC-024`,
`CDC-026`, `CDC-038`, `HIS-036`, `HIS-050`, `HIS-063`, `HIS-064`, `HIS-065`,
`HIS-067`, `HIS-068`.

## Ownership

- `cfdrs-shared` owns log configuration types (`LogLevel`, `LogFormat`, `LogConfig`, `RollingConfig`, `FileConfig`, `ConsoleConfig`, `build_log_config`, permission constants) — see ADR-0007
- `cfdrs-cli` owns logging flags, aliases, env bindings, help text, and user-visible CLI entry semantics
- `cfdrs-his` owns stderr/file sinks, rolling rotation, journald/systemd behavior, host log collection, and `LogSink` trait
- `cfdrs-cdc` owns management-token scope, `/logs` authentication, WebSocket protocol, filters, sampling, close codes, host-details upstream flow, and wire-protocol `LogLevel`

## CLI Contract

Required CLI-visible logging surface:

- `--loglevel` with baseline default `info`
- `--transport-loglevel` as a distinct transport-layer verbosity control
- `--logfile` for exact-path file logging
- `--log-directory` for host-managed log root selection
- `--log-format-output` for text or JSON formatting
- management and tail entry bindings for `TUNNEL_MANAGEMENT_TOKEN`
- hidden management-host routing and token-resource semantics where the frozen baseline exposes them

The CLI contract is not satisfied by internal tracing configuration alone.
The visible flags, envs, defaults, aliases, help text, and failure behavior must
match the frozen baseline.

## Local Sink Contract

Required local behavior:

- stderr remains a supported sink
- `--logfile` creates the requested file path
- `--log-directory` integrates with the baseline host layout and default `/var/log/cloudflared`
- rolling rotation matches the frozen baseline expectations from `logger/create.go`
  - max size: 1 MB
  - max backups: 5
  - max age: 0
- file and directory permissions follow the baseline logging contract
- systemd and SysV expectations remain explicit and are not approximated with ad hoc file placement

Current Rust slice:

- stderr remains the default runtime sink
- `--logfile` now opens and appends to the requested file while still mirroring stderr
- `--log-directory` and config `logDirectory` now select a local `cloudflared.log` target
- local rolling rotation enforces the admitted max-size/max-backups/max-age surface with backup-count enforcement tests
- `--log-format-output` now switches the runtime subscriber between text and JSON output
- `--loglevel` now drives runtime log filtering, and `--transport-loglevel` can widen the effective verbosity
- conditional `tracing_journald` layer activates when `JOURNAL_STREAM` is set (systemd-launched)
- `sd_notify::notify` sends `READY=1` matching Go `daemon.SdNotify`
- management `/logs` WebSocket streaming remains pending (CDC-026)

Local format divergence:

- local log output uses `tracing_subscriber` formatters (`.compact()` for text, `.json()` for JSON)
- this intentionally differs from Go's zerolog field names (`time`, `level`, `message`)
- local format is not part of the upstream contract — upstream format parity is tracked under CDC-026
- backup file naming uses numeric suffixes (`.1`, `.2`) instead of lumberjack timestamp naming — local-only concern

## Host Collection Contract

Required host-collection behavior:

- journald/systemd collection is root-gated and baseline-compatible
- the host collector must follow the same decision boundary as the frozen baseline:
  - root plus systemd: `journalctl -u cloudflared.service --since "2 weeks ago"`
  - otherwise: user log path or fallback error log path
- host-details collection must preserve the baseline field shape needed by the management surface

## Upstream Contract

Required upstream behavior:

- management token scope values remain exactly `logs`, `admin`, and `host_details`
- management routes require `access_token` query auth
- `/logs` upgrades to WebSocket and preserves the baseline command/event protocol
- client control messages remain `start_streaming` and `stop_streaming`
- server log payloads preserve `{time, level, message, event, fields}` shape
- filters remain baseline-compatible:
  - events: `cloudflared`, `http`, `tcp`, `udp`
  - levels: `debug`, `info`, `warn`, `error`
  - sampling: `0.0` to `1.0`
- close codes remain baseline-compatible: `4001`, `4002`, `4003`

## Promotion Rule

Logging compatibility is not a soft follow-up task.
Production-alpha is blocked until the local sink, journald/systemd, host
collection, and upstream `/logs` behavior are all evidenced against the frozen
baseline.
