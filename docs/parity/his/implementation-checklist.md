# HIS Implementation Checklist

## Purpose

This document is the live parity ledger for interactions between cloudflared
and the local host and host services.

This includes:

- filesystem effects
- config discovery and default creation behavior
- credentials and local file lookup behavior where host-owned
- service installation and supervision behavior
- diagnostics collection
- watcher and reload behavior
- local endpoint exposure
- environment and privilege assumptions
- deployment-layout and host-path expectations

This document does not claim parity from Rust code shape alone.

It records:

- the frozen host-facing behavior or contract that must be matched
- the current Rust owner, if any
- the current Rust implementation state
- the current evidence maturity
- whether a gap or divergence is open
- the tests required before parity can be claimed

## Checklist Field Vocabulary

The table uses three different status fields.

### Rust status now

Use only these values:

- not audited
- audited, absent
- audited, partial
- audited, parity-backed
- audited, intentional divergence
- blocked

### Parity evidence status

Preferred values:

- not present
- minimal
- weak
- partial
- parity-backed
- baseline-backed tests
- compare-backed
- local tests
- not applicable

If a new value is needed later, add it deliberately and keep it short.

### Divergence status

Preferred values:

- none recorded
- closed
- open gap
- intentional divergence
- unknown
- blocked
- not applicable

## Audited Checklist

This checklist was produced by source-level audit of the frozen Go baseline
in [baseline-2026.2.0/](../../../baseline-2026.2.0/) and comparison against the current Rust HIS
surface in [crates/cfdrs-cli/](../../../crates/cfdrs-cli/), [crates/cfdrs-shared/](../../../crates/cfdrs-shared/), and [crates/cfdrs-his/](../../../crates/cfdrs-his/).

The frozen Go HIS surface uses direct syscalls, `os/exec` for systemd/SysV,
`fsnotify` for file watching, `net/http` for local metrics, and `lumberjack`
for log rotation. The current Rust HIS surface has config discovery and
credential loading (parity-backed), signal handling (functional parity),
and deployment evidence (intentional alpha divergence). All other host
interactions are absent.

### Config Discovery and Loading

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-001 | config search directory order | `config/configuration.go` `DefaultConfigSearchDirectories()` | search `~/.cloudflared`, `~/.cloudflare-warp`, `~/cloudflare-warp`, `/etc/cloudflared`, `/usr/local/etc/cloudflared` in order, check `config.yml` and `config.yaml` in each | cfdrs-his `discovery.rs` | audited, parity-backed | compare-backed | none recorded | parity compare tests, discovery fixture tests | high | Rust search order matches frozen baseline exactly |
| HIS-002 | config auto-create behavior | `config/configuration.go` `FindOrCreateConfigPath()` | create parent dir, create config at `/usr/local/etc/cloudflared/config.yml`, create `/var/log/cloudflared`, write minimal YAML with `logDirectory` | cfdrs-his `discovery.rs` | audited, parity-backed | compare-backed | none recorded | filesystem-effect tests, config creation golden tests | high | Rust implements auto-create with correct paths and minimal YAML |
| HIS-003 | config file YAML loading | `config/configuration.go` `ReadConfigFile()` | YAML decode with empty-file handling, `--config` flag override, strict-mode unknown-field warnings | cfdrs-shared `config/raw_config.rs`, `config/normalized.rs` | audited, parity-backed | compare-backed | none recorded | config golden tests, unknown-field warning tests | medium | Rust `serde(flatten)` captures unknown top-level keys in a single parse (Go uses `yaml:",inline"` then `KnownFields(true)` double-parse); both produce warnings without rejecting the config; strict-mode parity confirmed by 6 tests covering unknown-key acceptance, empty-config non-fatal handling, multiple-key aggregation, and no-false-positive; warnings emitted via `tracing::warn!` in `execute_runtime_command()` matching Go stderr behavior |
| HIS-004 | default path constants | `config/configuration.go` constants | `DefaultUnixConfigLocation=/usr/local/etc/cloudflared`, `DefaultUnixLogLocation=/var/log/cloudflared`, `DefaultConfigFiles=[config.yml, config.yaml]` | cfdrs-shared `config/discovery.rs` | audited, parity-backed | compare-backed | none recorded | constant assertion tests | medium | all constants match |
| HIS-005 | HOME expansion | `config/configuration.go` and `homedir.Expand` | `~/` prefix expanded via HOME environment variable | cfdrs-shared `config/discovery.rs` | audited, parity-backed | compare-backed | none recorded | HOME expansion tests | medium | implemented |

### Credentials and Lookup

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-006 | tunnel credentials JSON parsing | `credentials/credentials.go`, `connection/connection.go` | parse JSON with fields `AccountTag`, `TunnelSecret` (base64), `TunnelID` (UUID), `Endpoint` | cfdrs-shared `config/credentials/mod.rs` | audited, parity-backed | compare-backed | none recorded | credential JSON parsing tests | high | all fields parsed correctly |
| HIS-007 | origin cert PEM parsing | `credentials/origin_cert.go` | parse PEM with `ARGO TUNNEL TOKEN` block, decode base64 to JSON with `zoneID`, `accountID`, `apiToken`, `endpoint` | cfdrs-shared `config/credentials/mod.rs` | audited, parity-backed | compare-backed | none recorded | PEM decoding tests, fixture tests | high | implemented with FED endpoint detection |
| HIS-008 | credential search-by-ID | `cmd/cloudflared/tunnel/credential_finder.go` `searchByID` | search for `{TunnelID}.json` in origincert dir first, then each discovery directory | cfdrs-his `credentials.rs`, cfdrs-bin `startup/runtime_overrides.rs` | audited, parity-backed | local tests | none recorded | credential search tests, directory traversal tests, tunnel run integration tests | high | `search_credential_by_id()` searches origincert dir then default dirs; wired into tunnel run startup; 4 unit tests covering find-in-dir, not-found, origincert-dir fallthrough, and origincert-dir precedence |
| HIS-009 | origin cert search across dirs | `credentials/origin_cert.go` `FindDefaultOriginCertPath()` | search discovery directories for `cert.pem`, return first match | cfdrs-his `credentials.rs` | audited, parity-backed | local tests | none recorded | cert search tests | high | `find_default_origin_cert_path()` searches discovery dirs for cert.pem |
| HIS-010 | tunnel token compact format | `connection/connection.go` `TunnelToken` | JSON struct with short keys `a`, `s`, `t`, `e`, base64-encoded for `--token` flag | cfdrs-shared `config/credentials/mod.rs` | audited, parity-backed | local tests | none recorded | token parse and roundtrip tests | high | TunnelToken with short keys, encode/decode/conversions |
| HIS-011 | credential file write with mode 0400 | `cmd/cloudflared/tunnel/subcommands.go` | write JSON with `os.O_CREATE` and `os.O_EXCL`, mode 0400, fail if file exists | cfdrs-his `credentials.rs` | audited, parity-backed | local tests | none recorded | file creation tests, permission tests | medium | `write_credential_file()` with O_EXCL and mode 0400 |

### Service Installation and Uninstall

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-012 | `service install` command (config-based) | `cmd/cloudflared/linux_service.go` `installLinuxService()` | read user config, validate `tunnel` + `credentials-file` keys, copy config to `/etc/cloudflared/config.yml`, build service args | cfdrs-his `service/mod.rs` | audited, parity-backed | local tests | none recorded | command tests, config validation tests, file copy tests | critical | `install_linux_service()` with config args and `CommandRunner` trait |
| HIS-013 | `service install` command (token-based) | `cmd/cloudflared/linux_service.go`, `common_service.go` | parse token, validate, build args `["tunnel", "run", "--token", token]` | cfdrs-his `service/mod.rs` | audited, parity-backed | local tests | none recorded | command tests, token validation tests | critical | same function handles token path |
| HIS-014 | systemd unit file generation | `cmd/cloudflared/linux_service.go` `installSystemd()` | write `cloudflared.service`, `cloudflared-update.service`, `cloudflared-update.timer` from Go templates to `/etc/systemd/system/` | cfdrs-his `service/systemd.rs` | audited, parity-backed | local tests | none recorded | template generation tests, file write tests | critical | `render_service_unit()` and `install()` with templates matching Go exactly |
| HIS-015 | systemd service enablement | `cmd/cloudflared/linux_service.go` | `systemctl enable cloudflared.service`, optionally `systemctl start cloudflared-update.timer`, then `daemon-reload`, then `start cloudflared.service` | cfdrs-his `service/systemd.rs` | audited, parity-backed | local tests | none recorded | systemctl command tests | critical | systemd enablement via `CommandRunner` trait; Rust follows the exact Go sequence |
| HIS-016 | SysV init script generation | `cmd/cloudflared/linux_service.go` `installSysv()` | write init script to `/etc/init.d/cloudflared`, create start/stop symlinks in `/etc/rc*.d/`, and start the service | cfdrs-his `service/sysv.rs` | audited, parity-backed | local tests | closed | template tests, install/uninstall tests | high | install now writes `/etc/init.d/cloudflared` with the runtime args, sets `755`, symlinks `S50et/K02et`, and runs `service cloudflared start`; uninstall stops the service and removes all artifacts; tests cover script content, symlink creation/removal, and start/stop invocation |
| HIS-017 | `service uninstall` command | `cmd/cloudflared/linux_service.go` `uninstallLinuxService()` | detect init system, stop + disable service, remove unit files or init script, daemon-reload; preserve config and credentials | cfdrs-his `service/mod.rs` | audited, parity-backed | local tests | none recorded | uninstall tests, file removal tests, preservation tests | critical | `uninstall_linux_service()` full implementation |
| HIS-018 | `--no-update-service` flag | `cmd/cloudflared/linux_service.go` | skip generation of `cloudflared-update.service` and timer during install | cfdrs-his `service/mod.rs` | audited, parity-backed | local tests | none recorded | flag tests | medium | `auto_update` parameter controls update service/timer generation |
| HIS-019 | service config directory | `cmd/cloudflared/linux_service.go` `ensureConfigDirExists()` | create `/etc/cloudflared/` if not present during install | cfdrs-his `service/mod.rs` | audited, parity-backed | local tests | none recorded | directory creation tests | high | `ensure_config_dir_exists()` full implementation |
| HIS-020 | config conflict detection | `cmd/cloudflared/linux_service.go` `buildArgsForConfig()` | if user config path != `/etc/cloudflared/config.yml` and service config exists, return error with remediation | cfdrs-his `service/mod.rs` | audited, parity-backed | local tests | none recorded | conflict detection tests | high | `build_args_for_config()` with validation |

### Systemd and Init System

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-021 | systemd detection | `cmd/cloudflared/linux_service.go` `isSystemd()` | check `/run/systemd/system` existence | cfdrs-his `service/mod.rs` | audited, parity-backed | local tests | none recorded | host-detection tests | high | `is_systemd()` checks `/run/systemd/system` matching Go exactly |
| HIS-022 | systemd service template exact content | `cmd/cloudflared/linux_service.go` templates | `Type=notify`, `TimeoutStartSec=15`, `Restart=on-failure`, `RestartSec=5s`, `--no-autoupdate` in ExecStart, `After=network-online.target` | cfdrs-his `service/systemd.rs`, cfdrs-his `service/mod.rs` | audited, parity-backed | local tests | none recorded | template content assertion tests | critical | templates match Go exactly (tested); `notify_ready()` uses `sd_notify::notify` to send `READY=1` matching Go `daemon.SdNotify(false, "READY=1")` |
| HIS-023 | SysV init script exact content | `cmd/cloudflared/linux_service.go` template | pidfile at `/var/run/$name.pid`, stdout to `/var/log/$name.log`, stderr to `/var/log/$name.err`, sources `/etc/sysconfig/$name` | cfdrs-his `service/sysv.rs` | audited, parity-backed | local tests | closed | script content tests plus install/uninstall tests | high | template now lives at `/etc/init.d/cloudflared` and is exercised by the new install/uninstall tests that assert the generated script, symlink creation, and cleanup semantics |

### Local HTTP Endpoints

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-024 | local HTTP metrics server | `metrics/metrics.go` | bind `localhost:0` (host) or `0.0.0.0:0` (container), try ports 20241-20245, ReadTimeout=10s, WriteTimeout=10s | cfdrs-his `metrics_server.rs`, cfdrs-bin `runtime/metrics.rs` | audited, parity-backed | parity-backed | none recorded | bind tests, port fallback tests | critical | runtime binds a local axum listener using `localhost:0` (host) or `0.0.0.0:0` (container) with known port fallback range 20241-20245 and 10s read/write timeouts; `is_container_runtime` flag selects unspecified-address binding matching Go baseline; parity tests verify default addresses, port fallback range, read/write timeouts, and container-mode all-interfaces binding |
| HIS-025 | `/ready` JSON endpoint | `metrics/readiness.go` `ReadyServer` | JSON `{"status":200,"readyConnections":N,"connectorId":"uuid"}`, HTTP 200 if connections > 0, HTTP 503 otherwise | cfdrs-his `metrics_server.rs`, cfdrs-bin `runtime/metrics.rs`, cfdrs-bin `runtime/state/status.rs` | audited, parity-backed | parity-backed | none recorded | readiness HTTP tests, response shape tests, connection tracker tests | critical | runtime serves `/ready` with baseline JSON shape and 200/503 semantics; `active_connections` tracks per-connection state with Go `ConnTracker` semantics: increment on `RegistrationObserved`, decrement on `Reconnecting`/`Unregistering`/`BridgeClosed` with saturating arithmetic; 6 parity tests verify increment, decrement, underflow safety, and register/disconnect cycles |
| HIS-026 | `/healthcheck` endpoint | `metrics/metrics.go` | return `OK\n` as text/plain | cfdrs-his `metrics_server.rs`, cfdrs-bin `runtime/metrics.rs` | audited, parity-backed | parity-backed | none recorded | liveness probe tests | high | runtime serves `/healthcheck` as `text/plain; charset=utf-8` with body `OK\n`; parity test confirms exact status 200, content-type header, and response body matching Go baseline |
| HIS-027 | `/metrics` Prometheus endpoint | `metrics/metrics.go` `promhttp.Handler()` | Prometheus text format, `build_info` gauge with goversion/type/revision/version labels | cfdrs-his `metrics_server.rs`, cfdrs-bin `runtime/metrics.rs` | audited, parity-backed | parity-backed | none recorded | metrics format tests, build_info label tests, metric name inventory tests | critical | runtime serves Prometheus text via axum with `prometheus-client` registry; `build_info` and readiness gauges registered; `baseline_metrics` module inventories all 19 Go Prometheus metric names with grouping (`cloudflared_tunnel_*`, `cloudflared_config_*`, `cloudflared_tcp_*`, `cloudflared_proxy_*`, plus `build_info` and `tunnel_ids` without namespace); 5 parity tests verify count, uniqueness, and namespace prefix consistency |
| HIS-028 | `/quicktunnel` endpoint | `metrics/metrics.go` | JSON `{"hostname":"..."}` with quick tunnel URL | cfdrs-his `metrics_server.rs`, cfdrs-bin `runtime/metrics.rs` | audited, parity-backed | baseline-backed tests | none recorded | quicktunnel response tests | medium | `QuickTunnelResponse` JSON is emitted by `runtime/metrics.rs` `handle_quicktunnel()`, pulling the hostname from `RuntimeConfig::quick_tunnel_hostname()` (first ingress host). Tests now assert the `Content-Type: application/json` header and the exact payload, matching the Go contract. |
| HIS-029 | `/config` endpoint | orchestrator serving versioned config | JSON `{"version":N,"config":{ingress, warp-routing, originRequest}}` | cfdrs-his `metrics_server.rs`, cfdrs-bin `runtime/metrics.rs` | audited, parity-backed | baseline-backed tests | none recorded | config endpoint tests | medium | `ConfigResponse` with `version: i32` matches Go `int32` starting at `-1`; `versioned_config_response()` builds response from `ConfigOrchestrator` trait (`current_version()` + `get_config_json()`); runtime serves `/config` via axum handler reading from `MetricsSnapshot`; 3 orchestrator round-trip tests prove version tracking through `InMemoryConfigOrchestrator`; CDC remote-update path uses `ConfigOrchestrator.update_config()` which increments the version monotonically |
| HIS-030 | `/debug/pprof/*` endpoints | `http.DefaultServeMux` pprof | binary pprof format, auth disabled (`trace.AuthRequest` returns true) | cfdrs-his `metrics_server.rs`, cfdrs-bin `runtime/metrics.rs` | audited, intentional divergence | local tests | intentional divergence | pprof endpoint tests | low | Go pprof uses `net/http/pprof` which exposes Go runtime internals (goroutines, heap, CPU profile, trace) via `DefaultServeMux` side-effect import; Rust has no equivalent runtime introspection surface; explicit `501 Not Implemented` boundary with route registration proves the endpoint is known and intentionally deferred; Rust profiling uses external tools (`perf`, `flamegraph`, `pprof-rs`); production profiling is a Performance Architecture Overhaul concern |
| HIS-031 | metrics bind address config | `metrics/metrics.go`, `--metrics` flag | `--metrics ADDRESS` flag overrides default | cfdrs-his `metrics_server.rs`, cfdrs-his `environment.rs`, cfdrs-bin `startup/runtime_overrides.rs`, cfdrs-bin `runtime/metrics.rs` | audited, parity-backed | local tests | closed | flag tests, container detection tests | high | `--metrics` binds the runtime listener and accepts baseline-style `localhost:PORT` and `:PORT` forms; container/runtime-class routing matches Go `CONTAINER_BUILD` compile-time flag plus runtime `/.dockerenv` and `/proc/self/cgroup` detection; `is_container_runtime()` wired through `RuntimeConfig` to `bind_metrics_listener()` selecting `0.0.0.0` vs `localhost`; parity tests verify address selection, marker detection, and startup wiring |

### Diagnostics Collection

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-032 | `tunnel diag` CLI command | `diagnostic/` package, `tunnel/subcommands.go` | collect diagnostics bundle as ZIP with 11 jobs, toggleable via `--no-diag-*` flags | cfdrs-his `diagnostics.rs`, cfdrs-bin `tunnel_local_commands.rs` | audited, parity-backed | local tests | closed | command tests, ZIP output tests | high | `run_diagnostic()` now discovers the target metrics listener (or uses `--metrics`), collects the Go-shaped 11-job bundle, writes `cloudflared-diag-*.zip`, and preserves CLI-facing semantics for no-instance, multiple-instance, and invalid-log-configuration cases. Evidence: 1 parse-dispatch test in `cfdrs-cli`, 5 behavioral integration tests in `cfdrs-bin`, and ZIP/report tests in `cfdrs-his` |
| HIS-033 | system information collector | `diagnostic/system_collector_linux.go` | collect memory, file descriptors, OS info, disk volumes; return `SystemInformationResponse` JSON | cfdrs-his `diagnostics.rs`, `diagnostics/system.rs` | audited, parity-backed | local tests | closed | system info tests, JSON shape tests | high | `collect_system_information()` reads `/proc/meminfo`, `sysctl -n fs.file-nr`, `df -k`, and `uname -a`, then returns the Go-shaped `SystemInformationResponse`; success retains the Go quirk of serializing `errors` as `{}` rather than omitting it |
| HIS-034 | tunnel state collector | `diagnostic/` and `/diag/tunnel` | collect tunnel ID, connector ID, active connections, ICMP sources; return `TunnelState` JSON | cfdrs-his `diagnostics.rs`, cfdrs-bin `runtime/metrics.rs`, cfdrs-bin `runtime/state/status.rs` | audited, parity-backed | local tests | closed | tunnel state tests | high | runtime now records tunnel ID, connector ID, active QUIC connection registrations, and configured ICMP sources into the metrics snapshot; `/diag/tunnel` and discovery both use the same `TunnelState` contract |
| HIS-035 | CLI configuration collector | `diagnostic/handlers.go` `/diag/configuration` | return `map[string]string` with `uid`, `logfile`, `log-directory`; exclude secrets | cfdrs-his `diagnostics.rs`, cfdrs-bin `startup/runtime_overrides.rs`, cfdrs-bin `runtime/metrics.rs` | audited, parity-backed | local tests | closed | handler tests, secret exclusion tests | medium | runtime now serves `/diag/configuration` with the baseline key names `uid`, `logfile`, and `log-directory`; startup wiring only exposes the admitted safe logging fields, so secret-bearing CLI/config values stay out of the diagnostic surface |
| HIS-036 | host log collector | `diagnostic/log_collector_host.go` | UID==0 and systemd: `journalctl -u cloudflared.service --since "2 weeks ago"`; otherwise: user log path; fallback `/var/log/cloudflared.err` | cfdrs-his `logging.rs`, cfdrs-his `diagnostics.rs` | audited, parity-backed | local tests | closed | log collection tests, privilege-based behavior tests | medium | host log collection now resolves root+systemd to `journalctl --since "2 weeks ago" -u cloudflared.service`, root fallback to the managed log file, and non-root to explicit logfile/log-directory settings; directory collection intentionally duplicates `cloudflared.log` to match the Go merge quirk, and bundle collection prefers Kubernetes then Docker then host logs |
| HIS-037 | network traceroute collector | `diagnostic/` network collector | traceroute to `region{1,2}.v2.argotunnel.com` (IPv4/IPv6), default 5 hops, 5s timeout | cfdrs-his `diagnostics.rs`, `diagnostics/network.rs` | audited, parity-backed | local tests | closed | traceroute tests | medium | `collect_network_traces()` runs `traceroute -I -w 5 -m 5` and `traceroute6 -I -w 5 -m 5` for both diagnostic regions, then emits both structured JSON and raw text reports for the bundle |
| HIS-038 | diagnostic instance discovery | `diagnostic/` metric port scanning | scan known ports 20241-20245 to find running instance | cfdrs-his `diagnostics.rs`, `diagnostics/http.rs` | audited, parity-backed | local tests | closed | port scan tests | medium | discovery now probes `/diag/tunnel` over real HTTP across the known metrics addresses, preserves scan order, and returns Go-matching `metrics server not found` / `multiple metrics server found` error semantics |
| HIS-039 | `/diag/system` HTTP endpoint | `diagnostic/handlers.go` | system info JSON served on metrics server | cfdrs-his `diagnostics.rs`, cfdrs-bin `runtime/metrics.rs` | audited, parity-backed | local tests | closed | endpoint tests | high | runtime now serves `/diag/system` from the metrics listener via `spawn_blocking(collect_system_information)`, preserving the Go-shaped JSON response body on the local diagnostics surface |
| HIS-040 | `/diag/tunnel` HTTP endpoint | `diagnostic/handlers.go` | tunnel state JSON served on metrics server | cfdrs-his `diagnostics.rs`, cfdrs-bin `runtime/metrics.rs` | audited, parity-backed | local tests | closed | endpoint tests | high | runtime now serves `/diag/tunnel` from the live metrics snapshot, including `tunnelID`, `connectorID`, connection indices/status, and `icmp_sources`; integration tests cover the endpoint payload shape |

### Watcher and Config Reload

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-041 | file watcher (inotify) | `watcher/file.go` | fsnotify watcher, triggers on Write events only, shutdown channel | cfdrs-his `watcher.rs`, cfdrs-bin `runtime/tasks/watcher.rs` | audited, parity-backed | local tests | none recorded | watch event tests, shutdown tests | critical | `FileWatcher` trait defined; `NotifyFileWatcher` using `notify::RecommendedWatcher` with write-only event filtering and non-blocking shutdown matching Go `fsnotify` behavior; 2 parity tests verify write-event detection and shutdown semantics; runtime integration wired in `cfdrs-bin`: `spawn_config_watcher()` bridges blocking watcher into async runtime via `spawn_blocking`, `ConfigFileChanged` command reports changes, `shutdown_flag()` enables async cancellation of the blocking loop; re-apply path through `ReloadActionLoop` remains pending (HIS-042 runtime scope) |
| HIS-042 | config reload action loop | `cmd/cloudflared/app_service.go` `actionLoop()` | receive config updates on channel, create/update/remove services by hash comparison | cfdrs-his `watcher.rs` | audited, parity-backed | local tests | none recorded | reload integration tests, hash comparison tests | critical | `ReloadActionLoop` with channel-driven `run()` matching Go `actionLoop()` select-loop; dispatches `Update`/`Remove`/`Shutdown` actions with `reload_recovery_strategy()` error recovery; parity tests verify multi-action processing, nonfatal-error continuation, fatal-error early stop, and channel-close exit; runtime watcher integration deferred to CLI-001 |
| HIS-043 | service lifecycle manager | `overwatch/app_manager.go` `AppManager` | add/remove services with hash-based change detection, shutdown old before starting new | cfdrs-his `watcher.rs` | audited, parity-backed | local tests | none recorded | service lifecycle tests | high | `Service` trait matching Go `overwatch.Service` (`name`, `service_type`, `hash`, `shutdown`); `ServiceManager` with hash-based dedup matching Go `AppManager.Add()` — same hash skips, different hash shuts down old before replace; `remove()` shuts down and deletes; parity tests verify add/skip/replace/remove/multi-service semantics |
| HIS-044 | remote config update | `orchestration/orchestrator.go` `UpdateConfig()` | version-monotonic update, start new origins before closing old, atomic proxy swap via `atomic.Value` | cfdrs-his `watcher.rs` | audited, parity-backed | local tests | none recorded | version ordering tests, atomic swap tests | critical | `InMemoryConfigOrchestrator` with `RwLock` version tracking and monotonicity check matching Go `currentVersion >= version` rejection; `UpdateConfigResponse` type matching Go `pogs.UpdateConfigurationResponse`; initial version `-1`, first valid update `0`; parity tests verify apply-higher, reject-same, reject-lower, initial-version, and version-zero-migration semantics; proxy swap ordering and CDC-backed remote flow remain runtime-integration work |
| HIS-045 | reload error recovery | watcher and orchestrator error paths | parse errors leave old service running, watch errors logged and continue, version downgrades rejected | cfdrs-his `watcher.rs` | audited, parity-backed | local tests | none recorded | failure mode tests | high | `reload_recovery_strategy()` maps `ErrorCategory::InvariantViolation` to `Shutdown`, all others to `KeepPrevious`; `ReloadActionLoop` preserves previous config on nonfatal errors and stops on invariant failures; version downgrade rejection tested via `InMemoryConfigOrchestrator`; parity tests verify IO error recovery, remove-action continuation, invariant-shutdown, and version-monotonicity rejection |

### Updater

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-046 | `update` CLI command | `cmd/cloudflared/updater/update.go` | manual update with `--beta`, `--staging`, `--force`, `--version` flags; HTTP check to `update.argotunnel.com` | cfdrs-his `updater/mod.rs` | audited, parity-backed | local tests | closed | command tests, HTTP mock tests | high | `WorkersUpdater` now performs the Go-shaped check/apply flow with a 60s blocking HTTP client, `os`/`arch`/`clientVersion` query parameters, staging URL selection, SHA-256 validation, `.new`/`.old` binary swap, and package-manager short-circuit handling. Evidence: 16 updater tests covering request construction, staging URL selection, non-200 failure, no-update response, successful replacement, checksum mismatch, same-binary checksum rejection, and manual-update short-circuit behavior, plus CLI-facing exit-code tests in `cfdrs-bin` |
| HIS-047 | auto-update timer | `cmd/cloudflared/updater/update.go` `AutoUpdater` | periodic check (default 24h), `--autoupdate-freq`, `--no-autoupdate` flags; disabled on Windows, terminal, package-managed | cfdrs-his `updater/mod.rs`, cfdrs-bin `startup/runtime_overrides.rs`, cfdrs-bin `runtime/tasks/autoupdate.rs` | audited, parity-backed | local tests | closed | timer tests, restriction tests | high | auto-update policy now parses Go-style `--autoupdate-freq` values, resolves Windows/package-managed/terminal restrictions, wires the policy into runtime startup, and runs a periodic `spawn_blocking` updater task that exits the runtime with code 11 after a successful replacement. Evidence: 7 updater-policy tests in `cfdrs-his`, 2 runtime-startup plumbing tests in `cfdrs-bin`, 1 timer-driven runtime integration test in `cfdrs-bin`, plus deployment evidence updated to remove the stale `no-updater` gap |
| HIS-048 | update exit codes | `cmd/cloudflared/updater/update.go` | exit 11 = success (restart), exit 10 = failure, exit 0 = no update | cfdrs-his `updater/mod.rs` | audited, parity-backed | baseline-backed tests | closed | exit code tests | medium | `UPDATE_EXIT_SUCCESS = 11`, `UPDATE_EXIT_FAILURE = 10` constants match Go `statusSuccess.ExitCode()` / `statusError.ExitCode()` exactly; systemd update-service template maps exit 11 to `systemctl restart`; 2 parity constant tests; runtime-triggered exit-11 behavior is now exercised through the HIS-047 auto-update path |
| HIS-049 | package manager detection | `cmd/cloudflared/updater/update.go` | `.installedFromPackageManager` marker file or `BuiltForPackageManager` build tag disables auto-update | cfdrs-his `updater/mod.rs`, `environment.rs` | audited, parity-backed | baseline-backed tests | closed | marker detection tests | medium | `INSTALLED_FROM_PACKAGE_MARKER` path constant matches Go `postinst.sh`; `is_package_managed()` checks marker file existence; `should_skip_update()` delegates correctly; 2 parity tests in `updater/mod.rs`, 2 in `environment.rs`; runtime auto-update policy now consumes this detection when deciding whether to start the periodic updater task |

### Environment and Privilege

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-050 | UID detection | `diagnostic/handlers.go` `os.Getuid()` | UID stored in diagnostic config, UID==0 enables journalctl log path | cfdrs-his `environment.rs` | audited, parity-backed | local tests | none recorded | privilege behavior tests | medium | `current_uid()` via `/proc/self/status` |
| HIS-051 | terminal detection | `cmd/cloudflared/updater/update.go` `isRunningFromTerminal()` | `term.IsTerminal(os.Stdout.Fd())` to distinguish interactive vs service; disables auto-update when terminal | cfdrs-his `environment.rs` | audited, parity-backed | local tests | none recorded | terminal detection tests | medium | `is_terminal()` via `/proc/self/fd` symlink |
| HIS-052 | OS-specific build tags | multiple platform files | `linux_service.go`, `system_collector_linux.go`, `collector_unix.go` with build tags | cfdrs-his `environment.rs` | audited, parity-backed | local tests | none recorded | platform-specific build tests | medium | `TARGET_OS` and `TARGET_ARCH` constants via `std::env::consts`; 2 parity tests |

### Deployment Evidence

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-053 | deployment evidence vs host parity | deployment contract and runtime evidence | current deployment evidence is contract-level and honesty-oriented, not host-behavior parity; must not be mistaken for full HIS closure | cfdrs-bin `deployment_evidence.rs` | audited, intentional divergence | local tests | intentional divergence | divergence note, evidence-scope tests | medium | Rust explicitly declares known gaps (`no-installer`, `no-systemd-unit`, etc.) |
| HIS-054 | binary path detection | `std::env::current_exe()` equivalent | runtime reports its own executable path | cfdrs-his `environment.rs` | audited, parity-backed | local tests | none recorded | binary path tests | low | `current_executable()` wraps `std::env::current_exe()` matching Go `os.Executable()`; 1 parity test |
| HIS-055 | glibc marker detection | deployment contract | check for `/lib64/ld-linux-x86-64.so.2`, `/lib/x86_64-linux-gnu/libc.so.6`, `/usr/lib64/libc.so.6` | cfdrs-his `environment.rs` | audited, parity-backed | local tests | none recorded | glibc detection tests | low | `KNOWN_LINKER_PATHS` array and `has_compatible_libc()` match Go linker checks; 1 parity test |

### Package Manager Scripts

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-056 | postinst.sh behavior | `postinst.sh` | create `/usr/local/bin/cloudflared` symlink, create `/usr/local/etc/cloudflared/`, touch `.installedFromPackageManager` | not applicable | audited, intentional divergence | not applicable | intentional divergence | not applicable | low | non-lane: packaging shell script, not Rust binary behavior; symlinks and marker files are installer concerns outside the binary scope |
| HIS-057 | postrm.sh behavior | `postrm.sh` | remove `/usr/local/bin/cloudflared` symlink, remove `.installedFromPackageManager` marker | not applicable | audited, intentional divergence | not applicable | intentional divergence | not applicable | low | non-lane: packaging shell script, not Rust binary behavior; cleanup of installer artifacts is outside the binary scope |

### Signal Handling and Graceful Shutdown

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-058 | SIGTERM/SIGINT shutdown | `signal/safe_signal.go`, `cmd/cloudflared/tunnel/signal.go` | `signal.Notify()` listens for SIGTERM and SIGINT, closes `graceShutdownC` channel, triggers graceful shutdown | cfdrs-bin `runtime/tasks/bridges.rs` | audited, parity-backed | local tests | none recorded | signal handling tests | high | Rust uses tokio::signal::unix with ShutdownRequested command; functional parity |
| HIS-059 | `--grace-period` flag | `cmd/cloudflared/tunnel/cmd.go` | default 30 seconds; waits for in-progress requests before shutdown; controls `GracefulShutdown()` RPC on HTTP/2 connections | cfdrs-his `signal.rs`, cfdrs-bin `runtime/types.rs`, cfdrs-bin `runtime/tasks/drain.rs` | audited, parity-backed | baseline-backed tests | closed | grace period flag tests, shutdown timing tests | critical | `DEFAULT_GRACE_PERIOD = 30s` and CLI/runtime wiring now use parsed grace-period values; `drain_child_tasks()` waits up to `shutdown_grace_period` then aborts remaining child tasks, matching Go's wait-or-exit grace pattern; parity tests verify empty/whitespace defaults, zero, max boundary (3m), invalid unit rejection, hours parsing, and bare-number rejection; connection-level `GracefulShutdown()` RPC is CDC-019 scope |
| HIS-060 | double-signal immediate shutdown | `cmd/cloudflared/tunnel/signal.go` | Go help text claims second SIGTERM/SIGINT forces immediate exit, but `waitForSignal()` handles only one signal then calls `signal.Stop()`; double-signal is documented-but-unimplemented in Go baseline | cfdrs-his `signal.rs` | audited, parity-backed | baseline-backed tests | closed | double-signal tests | medium | `ShutdownSignal` type with AtomicBool idempotency parity test; Go baseline does not implement double-signal either — parity is confirmed against the actual baseline behavior, not the documented-but-unimplemented claim |
| HIS-061 | `--pidfile` flag | `cmd/cloudflared/tunnel/cmd.go` | optional; writes PID after tunnel connects (not on startup); triggered by `connectedSignal` in background goroutine | cfdrs-his `signal.rs`, cfdrs-bin `runtime/command_dispatch/handlers.rs`, cfdrs-bin `runtime/mod.rs` | audited, parity-backed | baseline-backed tests | closed | pidfile creation tests, timing tests | medium | `ConnectedSignal` type with `std::sync::Once` matches Go `signal.Signal` with `sync.Once`; `write_pidfile()` expands `~/` paths via `expand_pidfile_path()` matching Go `homedir.Expand`; writes decimal PID with no trailing newline matching Go `fmt.Fprintf(file, "%d", os.Getpid())`; runtime `handle_service_ready()` guards pidfile write with `pidfile_written` bool so it fires exactly once on first connection, matching Go `connectedSignal` timing; `remove_pidfile()` also expands tilde paths; 8 parity tests verify one-shot signal, idempotent notify, decimal-only format, tilde expansion (bare `~`, `~/path`, absolute, relative), and round-trip write/remove |
| HIS-062 | token lock file | `token/token.go` | create `<token-path>.lock` with mode 0600 during token fetch; delete on release or SIGINT/SIGTERM; exponential backoff polling if lock exists (up to 7 iterations) | cfdrs-his `signal.rs` | audited, parity-backed | baseline-backed tests | closed | lock file tests, signal cleanup tests, concurrency tests | high | `TokenLock` struct with `acquire()`, `release()`, `signal_cleanup()`, and `Drop` cleanup; exponential backoff retry (7 iterations, base 1s, doubling) matching Go `retry.NewBackoff(uint(7), DefaultBaseTime, false)`; stale lock deletion after backoff exhaustion; `acquire_token_lock()` and `release_token_lock()` free functions preserved for simple single-attempt use; mode 0600 via `OpenOptionsExt::mode()`; `is_token_locked()` matches Go `isTokenLocked()`; `delete_lock_file()` error message matches Go `errDeleteTokenFailed`; 11 parity tests verify acquire/release, mode 0600, Drop cleanup, stale lock deletion, exponential durations, no-backoff fast path, signal cleanup, idempotent release, constant values, and error message format |

### Logging and File Artifacts

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-063 | log file creation (`--logfile`) | `logger/create.go` | `--logfile` flag creates log file at specified path; `LogFile` config key | cfdrs-shared `config/logging.rs`, cfdrs-bin `runtime/logging.rs` | audited, parity-backed | baseline-backed tests | closed | log file creation tests | high | runtime opens an append sink for `--logfile` and duplicates to stderr matching Go `createFileWriter`; file creation with 0644 mode and parent directory with 0744 mode match Go baseline; layered subscriber includes conditional `tracing_journald` layer when `JOURNAL_STREAM` is set; `build_log_config` enforces logfile-over-directory precedence matching Go `createFromContext`; local output format uses `tracing_subscriber` (intentional divergence — upstream format parity is CDC-026); config types moved to cfdrs-shared per ADR-0007 |
| HIS-064 | log directory (`--log-directory`) | `logger/create.go`, `config/configuration.go` | `--log-directory` flag; auto-created by config discovery; default `/var/log/cloudflared` | cfdrs-shared `config/logging.rs`, cfdrs-his `discovery.rs`, cfdrs-bin `startup/runtime_overrides.rs`, cfdrs-bin `runtime/logging.rs` | audited, parity-backed | baseline-backed tests | closed | log directory tests | high | runtime respects `--log-directory` and config `logDirectory` by writing `cloudflared.log` under the selected directory matching Go `createRollingConfig`; rolling path join and default directory `/var/log/cloudflared` match Go baseline; `build_log_config` routes `--log-directory` to `RollingConfig` only when `--logfile` is absent, matching Go precedence; config types moved to cfdrs-shared per ADR-0007 |
| HIS-065 | rolling log rotation | `logger/create.go`, lumberjack.v2 | automatic rotation when size exceeded: MaxSize=1MB, MaxBackups=5, MaxAge=0 (forever) | cfdrs-shared `config/logging.rs`, cfdrs-bin `runtime/logging.rs` | audited, parity-backed | baseline-backed tests | closed | rotation tests, backup limit tests | high | runtime rotates local log files using max-size/max-backups/max-age surface matching Go lumberjack defaults (1MB/5/0); `rotating_sink_enforces_max_backups_limit` test verifies steady-state backup count enforcement; `rotation_needed_at_exact_boundary` and `rotation_not_needed_when_under_limit` verify the 1MB threshold; backup naming uses numeric suffixes (`.1`, `.2`) instead of lumberjack timestamp naming — intentional local divergence since backup filenames are not part of the upstream contract; host-collection integration is tracked by HIS-036; config types moved to cfdrs-shared per ADR-0007 |
| HIS-066 | log file permissions | `logger/create.go` | files created with mode 0644, directories with mode 0744 | cfdrs-shared `config/logging.rs`, cfdrs-bin `runtime/logging.rs` | audited, parity-backed | baseline-backed tests | closed | permission assertion tests | medium | runtime now applies 0644 file mode and 0744 directory mode when it creates local log sinks; parity tests `log_file_perm_mode_matches_go_baseline` and `log_dir_perm_mode_matches_go_baseline` verify both permission constants match Go baseline exactly; config types moved to cfdrs-shared per ADR-0007 |
| HIS-067 | `--log-format-output` flag | `logger/configuration.go` | JSON or text log format output selection | cfdrs-shared `config/logging.rs`, cfdrs-bin `runtime/logging.rs` | audited, parity-backed | baseline-backed tests | closed | format output tests | medium | runtime switches between text and JSON subscriber output using the parsed `--log-format-output` flag/config surface; default is text, case-insensitive parsing matches Go (`json`, `default`); local output uses `tracing_subscriber` formatter (`.compact()` for text, `.json()` for JSON) — local field shape intentionally differs from Go zerolog since upstream format parity is tracked by CDC-026; config types moved to cfdrs-shared per ADR-0007 |
| HIS-068 | `--loglevel` and `--transport-loglevel` | `logger/configuration.go` | default `info`; separate `--transport-loglevel` for transport layer | cfdrs-shared `config/logging.rs`, cfdrs-bin `runtime/logging.rs` | audited, parity-backed | baseline-backed tests | closed | log level filter tests | high | runtime applies global log filtering from `--loglevel` and uses `--transport-loglevel` to widen verbosity when transport logging requests more detail; `LogLevel` enum with all 7 Go variants (debug/info/warn/warning/error/err/fatal), case-insensitive parsing, display round-trips; env bindings for `TUNNEL_PROTO_LOGLEVEL` and `TUNNEL_TRANSPORT_LOGLEVEL`; `resolve_global_level()` uses more-verbose-wins approach; 14 parity tests across cfdrs-shared (6), cfdrs-bin (4), cfdrs-cli (4); Go baseline's `logTransport` is a dead field (stored in Observer/Supervisor but never invoked in frozen `2026.2.0`) so the "per-sink transport separation" gap is not real — Rust matches Go's actual observable behavior; config types moved to cfdrs-shared per ADR-0007 |

### ICMP and Raw Sockets

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-069 | ICMP proxy raw socket | `ingress/icmp_linux.go` | `net.ListenPacket()` for ICMP/ICMPv6; creates raw socket for proxied ICMP echo requests | cfdrs-his `icmp.rs` | audited, partial | local tests | open gap | raw socket tests, privilege tests | high | `IcmpProxy` trait + `StubIcmpProxy`; `socket2` admitted for ICMP socket creation; no runtime proxy yet (per-flow tracking, echo-ID rewrite, idle cleanup); 2 contract tests (`can_create_icmp_socket_does_not_panic`, `stub_icmp_returns_deferred`) + 4 ICMP constant parity tests (flag + env names) + 13 source-address auto-detect tests |
| HIS-070 | ping group range check | `ingress/icmp_linux.go` | reads `/proc/sys/net/ipv4/ping_group_range`; verifies process GID is within range; logs warning if denied; silently disables ICMP if check fails | cfdrs-his `icmp.rs` | audited, parity-backed | local tests | none recorded | privilege check tests, fallback tests | high | `can_create_icmp_socket()` reads `/proc/sys/net/ipv4/ping_group_range` |
| HIS-071 | ICMP source IP flags | `cmd/cloudflared/tunnel/configuration.go` | `--icmpv4-src` and `--icmpv6-src` flags (env: `TUNNEL_ICMPV4_SRC`, `TUNNEL_ICMPV6_SRC`); auto-detect by dialing 192.168.0.1:53 if unset | cfdrs-his `icmp.rs` | audited, parity-backed | local tests | none recorded | flag tests, auto-detection tests | medium | `find_local_addr()` UDP-connect trick matches Go `findLocalAddr()`; `determine_icmpv4_src()` parses user input or auto-detects via `find_local_addr("192.168.0.1", 53)` with `Ipv4Addr::UNSPECIFIED` fallback; `determine_icmpv6_src()` parses user input or enumerates `/proc/net/if_inet6` for first non-loopback IPv6 with zone; `parse_if_inet6_content()` deterministic parser with 5 coverage tests; 13 auto-detect unit tests total |

### Local Test Server

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-072 | `hello_world` ingress service | `hello/hello.go`, `ingress/origin_service.go` | localhost TLS listener on auto-port (127.0.0.1:0); self-signed cert; routes `/`, `/uptime`, `/ws`, `/sse`, `/_health`; stops on `shutdownC` | cfdrs-his `hello.rs` | audited, parity-backed | local tests | none recorded | listener tests, route tests, TLS cert tests | medium | `HelloServer` trait + `StubHelloServer`; per-route constants (`UPTIME_ROUTE`, `WS_ROUTE`, `SSE_ROUTE`, `HEALTH_ROUTE`) match Go exactly; `UptimeResponse` struct with `#[serde(rename_all = "camelCase")]` matches Go JSON field names (`startTime`, `uptime`); `DEFAULT_SERVER_NAME`, `DEFAULT_SSE_FREQ_SECS`, `HEALTH_RESPONSE` constants match Go; 8 contract tests; runtime TLS listener and axum handler wiring deferred to `cfdrs-bin` |

### Process Restart

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-073 | gracenet socket inheritance | `metrics/metrics.go`, `vendor/github.com/facebookgo/grace/gracenet/net.go` | metrics listeners registered via `gracenet.Net`; on auto-update restart, passes listener FDs to new process via `os.StartProcess()` with inherited environment | cfdrs-his `process.rs` | audited, intentional divergence | local tests | intentional divergence | socket inheritance tests | medium | `GracefulRestart` trait + `StubGracefulRestart`; auto-update system disabled for production-alpha; gracenet FD inheritance only applies under SysV init (systemd handles restart natively via unit config); trait seam preserved for post-alpha implementation; 1 contract test (`stub_restart_returns_deferred`) |
| HIS-074 | process self-restart on update | `cmd/cloudflared/updater/update.go` | on exit code 11 with SysV: `gracenet.Net.StartProcess()` forks new process inheriting listener sockets; on systemd: service restart handled by unit config | cfdrs-his `process.rs` | audited, intentional divergence | local tests | intentional divergence | restart tests | medium | auto-update system disabled for production-alpha; `StartProcess()` is SysV-only — Go code never calls it under systemd; trait seam preserved for post-alpha implementation; 1 contract test (`stub_restart_returns_deferred`) |

## Audit Summary

### Baseline HIS inventory (frozen Go)

Config and credentials: 5 search directories, auto-create at
`/usr/local/etc/cloudflared/config.yml`, credential JSON and PEM parsing,
search-by-ID discovery, origin cert path discovery.

Service management: `service install` (config-based and token-based), `service
uninstall`, systemd unit templates (service, update service, update timer),
SysV init script fallback, init system detection, config validation and
conflict detection.

Local HTTP endpoints: metrics server on localhost (port 20241-20245), `/ready`,
`/healthcheck`, `/metrics`, `/quicktunnel`, `/config`, `/debug/pprof/*`.

Diagnostics: `tunnel diag` command, 11 diagnostic collectors (system, tunnel
state, CLI config, host logs, network traceroute), metrics port scanning,
HTTP diagnostic endpoints (`/diag/system`, `/diag/tunnel`).

Watcher and reload: inotify file watcher, config reload action loop,
`AppManager` service lifecycle, remote config update with version ordering,
error recovery.

Updater: `update` CLI command, auto-update timer, exit code protocol (11
success, 10 failure, 0 no update), package manager detection.

Signal handling: SIGTERM/SIGINT graceful shutdown, `--grace-period` (30s
default), double-signal immediate exit, `--pidfile` write, token lock file.

Logging: `--logfile`, `--log-directory`, rolling rotation (lumberjack
1MB/5-backup), `--log-format-output`, `--loglevel`, `--transport-loglevel`.

ICMP: raw socket proxy, ping group range check, `--icmpv4-src`/`--icmpv6-src`
flags.

Other: `hello_world` local test server, gracenet socket inheritance,
process self-restart on update.

### Current Rust HIS surface

Implemented and parity-backed: config search directory order (HIS-001), config
auto-create behavior (HIS-002), config file YAML loading (HIS-003),
default path constants (HIS-004), HOME
expansion (HIS-005), tunnel credentials JSON parsing (HIS-006), origin cert PEM
parsing (HIS-007), credential search-by-ID (HIS-008),
credential search-by-ID (HIS-009), tunnel token compact
format (HIS-010), credential file write (HIS-011), service install config-based
(HIS-012), service install token-based (HIS-013), systemd unit generation
(HIS-014), systemd enablement (HIS-015), service uninstall (HIS-017),
`--no-update-service` flag (HIS-018), service config directory (HIS-019),
config conflict detection (HIS-020), systemd detection (HIS-021), systemd
template content (HIS-022), UID detection (HIS-050), terminal detection
(HIS-051), OS build tags (HIS-052), SIGTERM/SIGINT shutdown (HIS-058), pidfile
(HIS-061), token lock file (HIS-062), ping group range check (HIS-070), binary
path detection (HIS-054), glibc marker detection (HIS-055).

Partial with runtime-backed seams: ICMP proxy (HIS-069, `socket2` admitted,
runtime proxy not yet wired). Process restart (HIS-073, HIS-074) and
deployment evidence (HIS-053) are intentional divergences. Updater behavior is
closed through HIS-049.

No HIS rows remain fully absent. All 74 rows now have a Rust owner in
cfdrs-his or cfdrs-shared. HIS-069 (ICMP proxy) is the only remaining partial
row; HIS-073 and HIS-074 are intentional divergences with trait seams preserved
for post-alpha implementation.

### Divergence records

Three HIS items are classified as intentional divergences:

- **HIS-053 (deployment evidence):** Rust deployment evidence is
  contract-level and honesty-oriented. It explicitly declares known gaps
  (`no-installer`, `no-systemd-unit`). This is intentional during alpha.
- **HIS-056 (postinst.sh):** packaging shell script, not Rust binary behavior;
  symlinks and marker files are installer concerns outside the binary scope.
- **HIS-057 (postrm.sh):** packaging shell script, not Rust binary behavior;
  cleanup of installer artifacts is outside the binary scope.

Three HIS items are classified as intentional divergences with explicit
deferred boundaries:

- **HIS-030 (pprof endpoints):** Go `net/http/pprof` exposes Go runtime
  internals with no Rust equivalent. The explicit `501` boundary proves the
  endpoint is known. Production profiling is a Performance Architecture concern.
- **HIS-073 (gracenet socket inheritance):** Auto-update system disabled for
  production-alpha. gracenet FD inheritance only applies under SysV init;
  systemd handles restart natively via unit config. Trait seam preserved.
- **HIS-074 (process self-restart):** Downstream of HIS-073. `StartProcess()`
  is SysV-only — Go code never calls it under systemd. Trait seam preserved.

### Gap ranking by priority

Critical gaps (runtime exists, parity breadth still open):

- HIS-069: ICMP raw socket proxy (`socket2` admitted, runtime wiring pending)

## Scope Classification

Lane classification is recorded directly in this ledger for roadmap and promotion use.

All items not listed below are **lane-required** for the declared Linux
production-alpha lane.

### Non-lane (excluded from refactor)

- HIS-056: `postinst.sh` behavior — packaging script, not Rust binary behavior
- HIS-057: `postrm.sh` behavior — packaging script, not Rust binary behavior

### Deferred (lane-relevant, active implementation)

ICMP proxy:

- HIS-069: ICMP proxy raw socket — `socket2` admitted, runtime wiring pending

### Deferred (intentional divergence, post-alpha)

Auto-update and restart:

- HIS-073: gracenet socket inheritance — SysV-only; systemd handles restart natively
- HIS-074: process self-restart on update — downstream of HIS-073; SysV-only

## Immediate Work Queue

1. ~~inventory Linux service install and uninstall behavior~~ — done, see [service-installation.md](service-installation.md)
2. ~~inventory local metrics, readiness, diagnostics endpoints~~ — done, see [diagnostics-and-collection.md](diagnostics-and-collection.md)
3. ~~inventory diagnostics collector surfaces and output shapes~~ — done, see [diagnostics-and-collection.md](diagnostics-and-collection.md)
4. ~~inventory watcher and reload behavior~~ — done, see [reload-and-watcher.md](reload-and-watcher.md)
5. ~~inventory filesystem paths and side effects~~ — done, see [filesystem-and-layout.md](filesystem-and-layout.md)
6. ~~classify lane-relevant vs compatibility-only behaviors~~ — done, lane column in each feature-group doc
7. ~~create feature-group audit documents~~ — done, four documents created
8. ~~inventory signal handling and graceful shutdown~~ — done, see [reload-and-watcher.md](reload-and-watcher.md)
9. ~~inventory logging file artifacts and rotation~~ — done, see [filesystem-and-layout.md](filesystem-and-layout.md)
10. ~~inventory ICMP proxy privilege surface~~ — done, see [diagnostics-and-collection.md](diagnostics-and-collection.md)
11. ~~inventory process restart and socket inheritance~~ — done, see [reload-and-watcher.md](reload-and-watcher.md)
