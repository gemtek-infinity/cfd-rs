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
- compare-backed
- local tests

If a new value is needed later, add it deliberately and keep it short.

### Divergence status

Preferred values:

- none recorded
- open gap
- intentional divergence
- unknown
- blocked

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
| HIS-003 | config file YAML loading | `config/configuration.go` `ReadConfigFile()` | YAML decode with empty-file handling, `--config` flag override, strict-mode unknown-field warnings | cfdrs-shared `config/raw_config.rs`, `config/normalized.rs` | audited, partial | compare-backed | open gap | config golden tests, unknown-field warning tests | medium | Rust loads YAML and tracks unknown top-level keys during normalization via `NormalizationWarning::UnknownTopLevelKeys`; warnings now emitted via `tracing::warn!` in `execute_runtime_command()` matching Go stderr warning behavior; strict-mode double-parse not confirmed but unknown-key detection is functional |
| HIS-004 | default path constants | `config/configuration.go` constants | `DefaultUnixConfigLocation=/usr/local/etc/cloudflared`, `DefaultUnixLogLocation=/var/log/cloudflared`, `DefaultConfigFiles=[config.yml, config.yaml]` | cfdrs-shared `config/discovery.rs` | audited, parity-backed | compare-backed | none recorded | constant assertion tests | medium | all constants match |
| HIS-005 | HOME expansion | `config/configuration.go` and `homedir.Expand` | `~/` prefix expanded via HOME environment variable | cfdrs-shared `config/discovery.rs` | audited, parity-backed | compare-backed | none recorded | HOME expansion tests | medium | implemented |

### Credentials and Lookup

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-006 | tunnel credentials JSON parsing | `credentials/credentials.go`, `connection/connection.go` | parse JSON with fields `AccountTag`, `TunnelSecret` (base64), `TunnelID` (UUID), `Endpoint` | cfdrs-shared `config/credentials/mod.rs` | audited, parity-backed | compare-backed | none recorded | credential JSON parsing tests | high | all fields parsed correctly |
| HIS-007 | origin cert PEM parsing | `credentials/origin_cert.go` | parse PEM with `ARGO TUNNEL TOKEN` block, decode base64 to JSON with `zoneID`, `accountID`, `apiToken`, `endpoint` | cfdrs-shared `config/credentials/mod.rs` | audited, parity-backed | compare-backed | none recorded | PEM decoding tests, fixture tests | high | implemented with FED endpoint detection |
| HIS-008 | credential search-by-ID | `cmd/cloudflared/tunnel/credential_finder.go` `searchByID` | search for `{TunnelID}.json` in origincert dir first, then each discovery directory | cfdrs-his `credentials.rs`, cfdrs-bin `startup/runtime_overrides.rs` | audited, parity-backed | local tests | none recorded | credential search tests, directory traversal tests, tunnel run integration tests | high | `search_credential_by_id()` searches origincert dir then default dirs; wired into tunnel run startup; 2 unit tests |
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
| HIS-016 | SysV init script generation | `cmd/cloudflared/linux_service.go` `installSysv()` | write init script to `/etc/init.d/cloudflared`, create start/stop symlinks in `/etc/rc*.d/` | cfdrs-his `service/sysv.rs` | audited, partial | local tests | open gap | template tests, symlink tests | high | template renders correctly; install/uninstall are deferred stubs |
| HIS-017 | `service uninstall` command | `cmd/cloudflared/linux_service.go` `uninstallLinuxService()` | detect init system, stop + disable service, remove unit files or init script, daemon-reload; preserve config and credentials | cfdrs-his `service/mod.rs` | audited, parity-backed | local tests | none recorded | uninstall tests, file removal tests, preservation tests | critical | `uninstall_linux_service()` full implementation |
| HIS-018 | `--no-update-service` flag | `cmd/cloudflared/linux_service.go` | skip generation of `cloudflared-update.service` and timer during install | cfdrs-his `service/mod.rs` | audited, parity-backed | local tests | none recorded | flag tests | medium | `auto_update` parameter controls update service/timer generation |
| HIS-019 | service config directory | `cmd/cloudflared/linux_service.go` `ensureConfigDirExists()` | create `/etc/cloudflared/` if not present during install | cfdrs-his `service/mod.rs` | audited, parity-backed | local tests | none recorded | directory creation tests | high | `ensure_config_dir_exists()` full implementation |
| HIS-020 | config conflict detection | `cmd/cloudflared/linux_service.go` `buildArgsForConfig()` | if user config path != `/etc/cloudflared/config.yml` and service config exists, return error with remediation | cfdrs-his `service/mod.rs` | audited, parity-backed | local tests | none recorded | conflict detection tests | high | `build_args_for_config()` with validation |

### Systemd and Init System

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-021 | systemd detection | `cmd/cloudflared/linux_service.go` `isSystemd()` | check `/run/systemd/system` existence | cfdrs-his `service/mod.rs` | audited, parity-backed | local tests | none recorded | host-detection tests | high | `is_systemd()` checks `/run/systemd/system` matching Go exactly |
| HIS-022 | systemd service template exact content | `cmd/cloudflared/linux_service.go` templates | `Type=notify`, `TimeoutStartSec=15`, `Restart=on-failure`, `RestartSec=5s`, `--no-autoupdate` in ExecStart, `After=network-online.target` | cfdrs-his `service/systemd.rs` | audited, parity-backed | local tests | none recorded | template content assertion tests | critical | templates match Go exactly (tested) |
| HIS-023 | SysV init script exact content | `cmd/cloudflared/linux_service.go` template | pidfile at `/var/run/$name.pid`, stdout to `/var/log/$name.log`, stderr to `/var/log/$name.err`, sources `/etc/sysconfig/$name` | cfdrs-his `service/sysv.rs` | audited, partial | local tests | open gap | script content tests | high | template renders correctly; install/uninstall deferred |

### Local HTTP Endpoints

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-024 | local HTTP metrics server | `metrics/metrics.go` | bind `localhost:0` (host) or `0.0.0.0:0` (container), try ports 20241-20245, ReadTimeout=10s, WriteTimeout=10s | cfdrs-his `metrics_server.rs`, cfdrs-bin `runtime/metrics.rs` | audited, partial | local tests | open gap | bind tests, port fallback tests | critical | runtime now binds a local listener using the host default address and known port fallback list with 10s read/write timeouts; baseline-backed parity tests now verify default addresses, port fallback range 20241-20245, and read/write timeouts against Go constants; container bind behavior and startup-delay parity remain open |
| HIS-025 | `/ready` JSON endpoint | `metrics/readiness.go` `ReadyServer` | JSON `{"status":200,"readyConnections":N,"connectorId":"uuid"}`, HTTP 200 if connections > 0, HTTP 503 otherwise | cfdrs-his `metrics_server.rs`, cfdrs-bin `runtime/metrics.rs` | audited, partial | local tests | open gap | readiness HTTP tests, response shape tests | critical | runtime serves `/ready` with the baseline JSON shape and 200/503 semantics; parity tests now verify exact Go camelCase field names (`status`, `readyConnections`, `connectorId`) and deserialize from Go JSON shape; full connection-tracker semantics still open |
| HIS-026 | `/healthcheck` endpoint | `metrics/metrics.go` | return `OK\n` as text/plain | cfdrs-his `metrics_server.rs`, cfdrs-bin `runtime/metrics.rs` | audited, partial | local tests | open gap | liveness probe tests | high | runtime serves `/healthcheck` as text/plain with `OK\n`; parity test confirms exact response body matches Go baseline; broader diagnostic route coverage is still pending |
| HIS-027 | `/metrics` Prometheus endpoint | `metrics/metrics.go` `promhttp.Handler()` | Prometheus text format, `build_info` gauge with goversion/type/revision/version labels | cfdrs-his `metrics_server.rs`, cfdrs-bin `runtime/metrics.rs` | audited, partial | local tests | open gap | metrics format tests, build_info label tests | critical | runtime serves Prometheus text with `build_info` and readiness gauges; config response shape parity test confirms expected serialization contract; parity for the full baseline registry remains open |
| HIS-028 | `/quicktunnel` endpoint | `metrics/metrics.go` | JSON `{"hostname":"..."}` with quick tunnel URL | cfdrs-his `metrics_server.rs` | audited, partial | local tests | blocked | quicktunnel response tests | medium | `QuickTunnelResponse` type with serialization tests; 1 parity test (`quick_tunnel_serializes`); no HTTP endpoint |
| HIS-029 | `/config` endpoint | orchestrator serving versioned config | JSON `{"version":N,"config":{ingress, warp-routing, originRequest}}` | cfdrs-his `metrics_server.rs`, cfdrs-bin `runtime/metrics.rs` | audited, partial | local tests | open gap | config endpoint tests | medium | runtime now serves `/config` with versioned JSON derived from the current normalized config; CDC-backed orchestrator semantics and remote-update parity remain open |
| HIS-030 | `/debug/pprof/*` endpoints | `http.DefaultServeMux` pprof | binary pprof format, auth disabled (`trace.AuthRequest` returns true) | cfdrs-his `metrics_server.rs`, cfdrs-bin `runtime/metrics.rs` | audited, partial | local tests | open gap | pprof endpoint tests | low | runtime now exposes an explicit deferred `501` boundary for `/debug/pprof/*`; real profiling payloads remain open |
| HIS-031 | metrics bind address config | `metrics/metrics.go`, `--metrics` flag | `--metrics ADDRESS` flag overrides default | cfdrs-his `metrics_server.rs`, cfdrs-bin `startup/runtime_overrides.rs`, cfdrs-bin `runtime/metrics.rs` | audited, partial | local tests | open gap | flag tests | high | `--metrics` now binds the runtime listener and accepts baseline-style `localhost:PORT` and `:PORT` forms; parity tests verify `:PORT` → localhost binding, `localhost:PORT` resolution, and explicit IP pass-through; container/runtime-class routing remains open |

### Diagnostics Collection

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-032 | `tunnel diag` CLI command | `diagnostic/` package, `tunnel/subcommands.go` | collect diagnostics bundle as ZIP with 11 jobs, toggleable via `--no-diag-*` flags | cfdrs-his `diagnostics.rs` | audited, partial | local tests | open gap | command tests, ZIP output tests | high | `DiagnosticHandler` trait + `StubDiagnosticHandler`; types defined, no runtime collection; 1 contract test (`stub_handler_returns_deferred`) |
| HIS-033 | system information collector | `diagnostic/system_collector_linux.go` | collect memory, file descriptors, OS info, disk volumes; return `SystemInformationResponse` JSON | cfdrs-his `diagnostics.rs` | audited, partial | local tests | open gap | system info tests, JSON shape tests | high | `SystemInformation`, `DiskVolumeInformation`, and `SystemInformationResponse` types match Go camelCase JSON tags with omitempty; 3 parity tests verify key names, omitempty behavior, and response wrapper shape; no runtime collection |
| HIS-034 | tunnel state collector | `diagnostic/` and `/diag/tunnel` | collect tunnel ID, connector ID, active connections, ICMP sources; return `TunnelState` JSON | cfdrs-his `diagnostics.rs` | audited, partial | local tests | open gap | tunnel state tests | high | `TunnelState` and `IndexedConnectionInfo` types match Go JSON tags including `tunnelID`, `connectorID`, `icmp_sources` casing; 3 parity tests verify key names, omitempty, and empty-object serialization; no runtime state collection |
| HIS-035 | CLI configuration collector | `diagnostic/handlers.go` `/diag/configuration` | return `map[string]string` with `uid`, `log_file`, `log_directory`; exclude secrets | cfdrs-his `diagnostics.rs`, cfdrs-bin `startup/runtime_overrides.rs`, cfdrs-bin `runtime/metrics.rs` | audited, partial | local tests | open gap | handler tests, secret exclusion tests | medium | runtime now serves `/diag/configuration` with UID and active local log file/directory hints; broader CLI-flag coverage and secret filtering parity remain open |
| HIS-036 | host log collector | `diagnostic/log_collector_host.go` | UID==0 and systemd: `journalctl -u cloudflared.service --since "2 weeks ago"`; otherwise: user log path; fallback `/var/log/cloudflared.err` | cfdrs-his `diagnostics.rs` | audited, partial | local tests | open gap | log collection tests, privilege-based behavior tests | medium | types defined; parity test confirms journalctl command, args, and fallback log path match Go baseline constants; no journalctl or log-file collection runtime |
| HIS-037 | network traceroute collector | `diagnostic/` network collector | traceroute to `region{1,2}.v2.argotunnel.com` (IPv4/IPv6), default 5 hops, 5s timeout | cfdrs-his `diagnostics.rs` | audited, partial | local tests | open gap | traceroute tests | medium | `DIAGNOSTIC_REGIONS` constant matches Go baseline; parity test verifies both region hostnames; no traceroute collection |
| HIS-038 | diagnostic instance discovery | `diagnostic/` metric port scanning | scan known ports 20241-20245 to find running instance | cfdrs-his `diagnostics.rs` | audited, partial | local tests | open gap | port scan tests | medium | `AddressableTunnelState` wraps (state, address). `DiscoveryError` enum: `MetricsServerNotFound`, `MultipleMetricsServersFound` matching Go error messages. `known_metrics_addresses(is_virtual)` builds host/virtual port lists. `find_metrics_server(addresses, probe)` implements Go `FindMetricsServer` logic with injectable probe. 8 tests: address generation (host + virtual), 0/1/N instance scenarios, scan order, error display, type fields. Real HTTP probe pending. |
| HIS-039 | `/diag/system` HTTP endpoint | `diagnostic/handlers.go` | system info JSON served on metrics server | cfdrs-his `diagnostics.rs` | audited, partial | local tests | open gap | endpoint tests | high | types defined with Go-matching JSON shape; 3 JSON shape parity tests (`system_info_json_keys_match_go_baseline`, `system_info_response_json_shape_matches_go`, `disk_volume_json_keys_match_go_baseline`); no `/diag/system` HTTP handler |
| HIS-040 | `/diag/tunnel` HTTP endpoint | `diagnostic/handlers.go` | tunnel state JSON served on metrics server | cfdrs-his `diagnostics.rs` | audited, partial | local tests | open gap | endpoint tests | high | types defined with Go-matching JSON shape; 3 JSON shape parity tests (`tunnel_state_json_keys_match_go_baseline`, `indexed_connection_info_json_keys_match_go_baseline`, `tunnel_state_omitempty_matches_go`); no `/diag/tunnel` HTTP handler |

### Watcher and Config Reload

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-041 | file watcher (inotify) | `watcher/file.go` | fsnotify watcher, triggers on Write events only, shutdown channel | cfdrs-his `watcher.rs` | audited, partial | minimal | blocked | watch event tests, shutdown tests | critical | `FileWatcher` trait defined; no inotify runtime (needs notify crate) |
| HIS-042 | config reload action loop | `cmd/cloudflared/app_service.go` `actionLoop()` | receive config updates on channel, create/update/remove services by hash comparison | cfdrs-his `watcher.rs` | audited, partial | local tests | open gap | reload integration tests, hash comparison tests | critical | `ReloadActionLoop` now models update/remove/shutdown handling with restart-or-keep-previous recovery; `notify` wiring and service-hash comparison remain open |
| HIS-043 | service lifecycle manager | `overwatch/app_manager.go` `AppManager` | add/remove services with hash-based change detection, shutdown old before starting new | cfdrs-his `watcher.rs` | audited, partial | minimal | blocked | service lifecycle tests | high | `AppManager` trait defined; no lifecycle runtime |
| HIS-044 | remote config update | `orchestration/orchestrator.go` `UpdateConfig()` | version-monotonic update, start new origins before closing old, atomic proxy swap via `atomic.Value` | cfdrs-his `watcher.rs` | audited, partial | local tests | open gap | version ordering tests, atomic swap tests | critical | `InMemoryConfigOrchestrator` now provides an owned update/read seam for config JSON; parity tests verify initial config return and latest-update preservation; version monotonicity, proxy swap ordering, and CDC-backed remote flow remain open |
| HIS-045 | reload error recovery | watcher and orchestrator error paths | parse errors leave old service running, watch errors logged and continue, version downgrades rejected | cfdrs-his `watcher.rs` | audited, partial | local tests | open gap | failure mode tests | high | `reload_recovery_strategy()` plus `ReloadActionLoop` now preserve the previous config on nonfatal errors and stop on invariant failures; parity tests verify IO error recovery and remove-action continuation; runtime watcher integration remains deferred |

### Updater

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-046 | `update` CLI command | `cmd/cloudflared/updater/update.go` | manual update with `--beta`, `--staging`, `--force`, `--version` flags; HTTP check to `update.argotunnel.com` | cfdrs-his `updater.rs` | audited, partial | local tests | open gap | command tests, HTTP mock tests | high | `Updater` trait + constants; `StubUpdater` returns deferred; 6 parity tests (`stub_updater_returns_deferred`, `update_exit_success_is_11`, `update_exit_failure_is_10`, `marker_path_matches_go_postinst`, `should_skip_update_delegates_to_package_managed`, `update_server_matches_go`) |
| HIS-047 | auto-update timer | `cmd/cloudflared/updater/update.go` `AutoUpdater` | periodic check (default 24h), `--autoupdate-freq`, `--no-autoupdate` flags; disabled on Windows, terminal, package-managed | cfdrs-his `updater.rs` | audited, partial | local tests | open gap | timer tests, restriction tests | high | `AutoUpdater` trait + constants; `StubAutoUpdater` returns deferred; 1 parity test (`default_autoupdate_freq_is_24h`) |
| HIS-048 | update exit codes | `cmd/cloudflared/updater/update.go` | exit 11 = success (restart), exit 10 = failure, exit 0 = no update | cfdrs-his `updater.rs` | audited, parity-backed | local tests | open gap | exit code tests | medium | `UPDATE_EXIT_SUCCESS = 11`, `UPDATE_EXIT_FAILURE = 10` constants match Go `statusSuccess.ExitCode()` / `statusError.ExitCode()` exactly; systemd update-service template maps exit 11 to `systemctl restart`; 2 parity constant tests |
| HIS-049 | package manager detection | `cmd/cloudflared/updater/update.go` | `.installedFromPackageManager` marker file or `BuiltForPackageManager` build tag disables auto-update | cfdrs-his `updater.rs`, `environment.rs` | audited, parity-backed | local tests | open gap | marker detection tests | medium | `INSTALLED_FROM_PACKAGE_MARKER` path constant matches Go `postinst.sh`; `is_package_managed()` checks marker file existence; `should_skip_update()` delegates correctly; 2 parity tests in updater.rs, 2 in environment.rs |

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
| HIS-056 | postinst.sh behavior | `postinst.sh` | create `/usr/local/bin/cloudflared` symlink, create `/usr/local/etc/cloudflared/`, touch `.installedFromPackageManager` | not applicable | not audited | not applicable | not applicable | packaging tests | low | packaging concern, not Rust binary behavior |
| HIS-057 | postrm.sh behavior | `postrm.sh` | remove `/usr/local/bin/cloudflared` symlink, remove `.installedFromPackageManager` marker | not applicable | not audited | not applicable | not applicable | packaging tests | low | packaging concern, not Rust binary behavior |

### Signal Handling and Graceful Shutdown

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-058 | SIGTERM/SIGINT shutdown | `signal/safe_signal.go`, `cmd/cloudflared/tunnel/signal.go` | `signal.Notify()` listens for SIGTERM and SIGINT, closes `graceShutdownC` channel, triggers graceful shutdown | cfdrs-bin `runtime/tasks/bridges.rs` | audited, parity-backed | local tests | none recorded | signal handling tests | high | Rust uses tokio::signal::unix with ShutdownRequested command; functional parity |
| HIS-059 | `--grace-period` flag | `cmd/cloudflared/tunnel/cmd.go` | default 30 seconds; waits for in-progress requests before shutdown; controls `GracefulShutdown()` RPC on HTTP/2 connections | cfdrs-his `signal.rs`, cfdrs-bin `runtime/types.rs` | audited, partial | local tests | open gap | grace period flag tests, shutdown timing tests | critical | `DEFAULT_GRACE_PERIOD = 30s` and CLI/runtime wiring now use parsed grace-period values; parity tests verify empty/whitespace defaults, zero, max boundary (3m), invalid unit rejection, hours parsing, and bare-number rejection; connection-level graceful-shutdown behavior remains open |
| HIS-060 | double-signal immediate shutdown | `cmd/cloudflared/tunnel/signal.go` | Go help text claims second SIGTERM/SIGINT forces immediate exit, but `waitForSignal()` handles only one signal then calls `signal.Stop()`; double-signal is documented-but-unimplemented in Go baseline | cfdrs-his `signal.rs` | audited, parity-backed | local tests | intentional-gap | double-signal tests | medium | `ShutdownSignal` type with AtomicBool idempotency parity test; Go baseline does not implement double-signal either — parity is confirmed against the actual baseline behavior, not the documented-but-unimplemented claim |
| HIS-061 | `--pidfile` flag | `cmd/cloudflared/tunnel/cmd.go` | optional; writes PID after tunnel connects (not on startup); triggered by `connectedSignal` in background goroutine | cfdrs-his `signal.rs`, cfdrs-bin `runtime/command_dispatch/handlers.rs` | audited, partial | local tests | open gap | pidfile creation tests, timing tests | medium | pidfile helpers are wired on runtime service-ready and cleanup; exact `connectedSignal` timing still needs parity proof |
| HIS-062 | token lock file | `token/token.go` | create `<token-path>.lock` with mode 0600 during token fetch; delete on release or SIGINT/SIGTERM; exponential backoff polling if lock exists (up to 7 iterations) | cfdrs-his `signal.rs` | audited, partial | local tests | open gap | lock file tests, signal cleanup tests, concurrency tests | high | `acquire_token_lock()` and `release_token_lock()` with O_EXCL; mode 0600 now enforced via `OpenOptionsExt::mode()`; parity test verifies permissions; still missing: exponential backoff retry (Go retries 7x), stale lock deletion after backoff exhaustion, and SIGINT/SIGTERM cleanup handler for lock |

### Logging and File Artifacts

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-063 | log file creation (`--logfile`) | `logger/create.go` | `--logfile` flag creates log file at specified path; `LogFile` config key | cfdrs-his `logging.rs`, cfdrs-bin `runtime/logging.rs` | audited, partial | local tests | open gap | log file creation tests | high | runtime now opens an append sink for `--logfile` and continues duplicating output to stderr; parity tests verify file creation with 0644 mode and parent directory creation with 0744 mode; rotation and journald/systemd remain separate gaps |
| HIS-064 | log directory (`--log-directory`) | `logger/create.go`, `config/configuration.go` | `--log-directory` flag; auto-created by config discovery; default `/var/log/cloudflared` | cfdrs-his `logging.rs`, cfdrs-his `discovery.rs`, cfdrs-bin `startup/runtime_overrides.rs`, cfdrs-bin `runtime/logging.rs` | audited, partial | local tests | open gap | log directory tests | high | runtime now respects `--log-directory` and config `logDirectory` by writing `cloudflared.log` under the selected directory; parity tests verify rolling path join and default directory matches Go baseline; rotation parity is still open |
| HIS-065 | rolling log rotation | `logger/create.go`, lumberjack.v2 | automatic rotation when size exceeded: MaxSize=1MB, MaxBackups=5, MaxAge=0 (forever) | cfdrs-his `logging.rs`, cfdrs-bin `runtime/logging.rs` | audited, partial | local tests | open gap | rotation tests, size limit tests | high | runtime now rotates local log files using the admitted max-size/max-backups/max-age surface; parity tests verify rotation threshold boundaries and default dirname matches Go; exact lumberjack naming, journald/systemd parity, and host-collection integration remain open |
| HIS-066 | log file permissions | `logger/create.go` | files created with mode 0644, directories with mode 0744 | cfdrs-his `logging.rs`, cfdrs-bin `runtime/logging.rs` | audited, parity-backed | local tests | none recorded | permission assertion tests | medium | runtime now applies 0644 file mode and 0744 directory mode when it creates local log sinks; parity tests verify both permission constants match Go baseline exactly |
| HIS-067 | `--log-format-output` flag | `logger/configuration.go` | JSON or text log format output selection | cfdrs-his `logging.rs`, cfdrs-bin `runtime/logging.rs` | audited, partial | local tests | open gap | format output tests | medium | runtime now switches between text and JSON subscriber output using the parsed flag/config surface; parity tests verify default is text, JSON round-trips, and case-insensitive parsing matches Go; parity for every baseline field remains open |
| HIS-068 | `--loglevel` and `--transport-loglevel` | `logger/configuration.go` | default `info`; separate `--transport-loglevel` for transport layer | cfdrs-his `logging.rs`, cfdrs-bin `runtime/logging.rs` | audited, partial | local tests | open gap | log level filter tests | high | runtime now applies global log filtering from `--loglevel` and uses `--transport-loglevel` to widen verbosity when transport logging requests more detail; parity tests verify default info, all Go level variants, display round-trips, case-insensitive parsing, and transport-level widening; strict per-sink transport separation remains open |

### ICMP and Raw Sockets

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-069 | ICMP proxy raw socket | `ingress/icmp_linux.go` | `net.ListenPacket()` for ICMP/ICMPv6; creates raw socket for proxied ICMP echo requests | cfdrs-his `icmp.rs` | audited, partial | local tests | open gap | raw socket tests, privilege tests | high | `IcmpProxy` trait + `StubIcmpProxy`; no raw socket creation; 2 contract tests (`can_create_icmp_socket_does_not_panic`, `stub_icmp_returns_deferred`) + 4 ICMP constant parity tests (flag + env names) |
| HIS-070 | ping group range check | `ingress/icmp_linux.go` | reads `/proc/sys/net/ipv4/ping_group_range`; verifies process GID is within range; logs warning if denied; silently disables ICMP if check fails | cfdrs-his `icmp.rs` | audited, parity-backed | local tests | none recorded | privilege check tests, fallback tests | high | `can_create_icmp_socket()` reads `/proc/sys/net/ipv4/ping_group_range` |
| HIS-071 | ICMP source IP flags | `cmd/cloudflared/tunnel/configuration.go` | `--icmpv4-src` and `--icmpv6-src` flags (env: `TUNNEL_ICMPV4_SRC`, `TUNNEL_ICMPV6_SRC`); auto-detect by dialing 192.168.0.1:53 if unset | cfdrs-his `icmp.rs` | audited, partial | local tests | open gap | flag tests, auto-detection tests | medium | flag and env var constants match Go baseline exactly; 4 parity assertion tests; auto-detect logic not implemented |

### Local Test Server

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-072 | `hello_world` ingress service | `hello/hello.go`, `ingress/origin_service.go` | localhost TLS listener on auto-port (127.0.0.1:0); self-signed cert; routes `/`, `/uptime`, `/ws`, `/sse`, `/_health`; stops on `shutdownC` | cfdrs-his `hello.rs` | audited, partial | local tests | open gap | listener tests, route tests, TLS cert tests | medium | `HelloServer` trait + `StubHelloServer` + route constants; no listener/handler; 1 parity test (`hello_routes_match_go`) |

### Process Restart

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-073 | gracenet socket inheritance | `metrics/metrics.go`, `vendor/github.com/facebookgo/grace/gracenet/net.go` | metrics listeners registered via `gracenet.Net`; on auto-update restart, passes listener FDs to new process via `os.StartProcess()` with inherited environment | cfdrs-his `process.rs` | audited, partial | local tests | open gap | socket inheritance tests | medium | `GracefulRestart` trait + `StubGracefulRestart`; deferred; 1 contract test (`stub_restart_returns_deferred`) |
| HIS-074 | process self-restart on update | `cmd/cloudflared/updater/update.go` | on exit code 11 with SysV: `gracenet.Net.StartProcess()` forks new process inheriting listener sockets; on systemd: service restart handled by unit config | cfdrs-his `process.rs` | audited, partial | local tests | open gap | restart tests | medium | depends on updater; `StubGracefulRestart` returns deferred error; 1 contract test (`stub_restart_returns_deferred`) |

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
auto-create behavior (HIS-002), default path constants (HIS-004), HOME
expansion (HIS-005), tunnel credentials JSON parsing (HIS-006), origin cert PEM
parsing (HIS-007), credential search-by-ID (HIS-009), tunnel token compact
format (HIS-010), credential file write (HIS-011), service install config-based
(HIS-012), service install token-based (HIS-013), systemd unit generation
(HIS-014), systemd enablement (HIS-015), service uninstall (HIS-017),
`--no-update-service` flag (HIS-018), service config directory (HIS-019),
config conflict detection (HIS-020), systemd detection (HIS-021), systemd
template content (HIS-022), UID detection (HIS-050), terminal detection
(HIS-051), OS build tags (HIS-052), SIGTERM/SIGINT shutdown (HIS-058), pidfile
(HIS-061), token lock file (HIS-062), ping group range check (HIS-070), binary
path detection (HIS-054), glibc marker detection (HIS-055).

Partial with runtime-backed seams: credential search-by-ID (HIS-008, run-path
integration landed but wider evidence remains open), SysV init script (HIS-016,
HIS-023, template only), local HTTP metrics server (HIS-024 through HIS-031,
runtime listener plus partial endpoints including `/config`), all diagnostics
(HIS-032 through HIS-040, types + stub), watcher/reload (HIS-041 through
HIS-045, concrete reload/orchestrator seams plus deferred watcher wiring), updater (HIS-046 through HIS-049,
traits + constants), grace period (HIS-059, 30s default plus CLI/runtime
wiring, but connection-level graceful shutdown still open), double-signal
(HIS-060, type only), logging (HIS-063 through HIS-068,
runtime sink wiring plus config builder/types and bounded rotation), ICMP (HIS-069, HIS-071, traits + constants),
`hello_world` (HIS-072, trait only), process restart (HIS-073, HIS-074,
trait only), deployment evidence (HIS-053, intentional divergence).

No HIS rows remain fully absent. All 74 rows now have a Rust owner in
cfdrs-his or cfdrs-shared. Runtime behavior for blocked items (diagnostic HTTP
breadth, inotify, rolling rotation, raw sockets) is deferred behind owned seams.

### Divergence records

Two HIS items are classified as intentional divergences:

- **HIS-053 (deployment evidence):** Rust deployment evidence is
  contract-level and honesty-oriented. It explicitly declares known gaps
  (`no-installer`, `no-systemd-unit`). This is intentional during alpha.

- **HIS-059 (`--grace-period`):** Rust now uses the 30s default in
  cfdrs-his and threads parsed CLI values into cfdrs-bin runtime shutdown.
  This remains an `open gap` because connection-level `GracefulShutdown()`
  behavior and the double-signal escape are not yet implemented.

Note: HIS-053 is the only true `intentional divergence` status. HIS-059 is
`open gap` despite having the correct constant defined.

Blocked items use owned seams to define the API surface while keeping the
remaining runtime gaps explicit (diagnostic HTTP breadth, inotify/notify,
raw sockets). These include HIS-028, HIS-039, HIS-040 (remaining diagnostics
routes) and HIS-041 through HIS-044 (watcher/reload runtime wiring).

### Gap ranking by priority

Critical gaps (runtime exists, parity breadth still open):

- HIS-024: local HTTP metrics server (baseline-backed constant tests landed; container bind mode and startup ordering still open)
- HIS-025: `/ready` JSON endpoint (baseline-backed JSON shape tests landed; full connection-tracker semantics still open)
- HIS-027: `/metrics` Prometheus endpoint (config response shape test landed; full baseline registry still open)
- HIS-041: file watcher (needs notify crate)
- HIS-042: config reload action loop (runtime wiring and service-hash parity still open)
- HIS-044: remote config update handling (orchestrator parity tests landed; needs CDC-backed version ordering and proxy swap)
- HIS-059: `--grace-period` (baseline-backed edge-case tests landed; connection-level graceful shutdown still open)

High gaps (runtime-backed but incomplete):

- HIS-008: credential search-by-ID (needs wider integration evidence)
- HIS-016, HIS-023: SysV init (deferred, template exists)
- HIS-026: `/healthcheck` (parity test confirms exact Go body; broader server parity still open)
- HIS-031: metrics bind address (parity tests for `:PORT`, `localhost:PORT`, explicit IP; runtime-class/container routing still open)
- HIS-032 through HIS-034: diagnostic command and collectors (stub)
- HIS-039, HIS-040: diagnostic HTTP endpoints (stub)
- HIS-043: service lifecycle manager (trait only)
- HIS-045: reload error recovery (strategy, action loop, and parity tests implemented; watcher integration deferred)
- HIS-046, HIS-047: update command and auto-update (stub)
- HIS-062: token lock file (implemented)
- HIS-063: log file creation (runtime file sink landed; file perm parity tests verify 0644/0744; journald and rotation still open)
- HIS-064: log directory (runtime file sink landed; rolling path tests verified; host-collection parity still open)
- HIS-065: rolling log rotation (runtime rotation landed; threshold parity tests verify boundaries; exact lumberjack parity still open)
- HIS-068: `--loglevel` and `--transport-loglevel` (global filtering landed; parity tests verify all Go variants and transport widening; exact transport separation still open)
- HIS-069, HIS-070: ICMP raw socket and ping group check (stub + check)

Medium gaps (trait-defined or constants only):

- HIS-003: config strict-mode warnings
- HIS-028, HIS-029: quicktunnel and config endpoints (needs HTTP server)
- HIS-035 through HIS-037: diagnostic sub-collectors (stub)
- HIS-038: diagnostic instance discovery (local tests, real HTTP pending)
- HIS-048, HIS-049: update exit codes and package detection (constants)
- HIS-060: double-signal immediate shutdown (type only)
- HIS-066, HIS-067: log file permissions and format output (baseline-backed parity tests now verify constants and round-trips; runtime sink wiring landed)
- HIS-071: ICMP source IP flags (constants only)
- HIS-072: `hello_world` ingress listener (trait only)
- HIS-073, HIS-074: gracenet socket inheritance and process restart (trait only)

Low gaps:

- HIS-030: pprof endpoints
- HIS-054, HIS-055: deployment evidence details
- HIS-056, HIS-057: package manager scripts

## Scope Classification

Lane classification is recorded directly in this ledger for roadmap and promotion use.

All items not listed below are **lane-required** for the declared Linux
production-alpha lane.

### Non-lane (excluded from refactor)

- HIS-056: `postinst.sh` behavior — packaging script, not Rust binary behavior
- HIS-057: `postrm.sh` behavior — packaging script, not Rust binary behavior

### Deferred (lane-relevant, post-alpha)

SysV init:

- HIS-016: SysV init script generation — ADR-0005 states systemd governs alpha
- HIS-023: SysV init script exact content — same rationale

Diagnostics subsystem:

- HIS-032: `tunnel diag` CLI command
- HIS-033: system information collector
- HIS-034: tunnel state collector
- HIS-035: CLI configuration collector
- HIS-036: host log collector
- HIS-037: network traceroute collector
- HIS-038: diagnostic instance discovery
- HIS-039: `/diag/system` HTTP endpoint
- HIS-040: `/diag/tunnel` HTTP endpoint

Updater subsystem:

- HIS-046: `update` CLI command — requires external infrastructure
- HIS-047: auto-update timer — depends on updater
- HIS-048: update exit codes — depends on updater
- HIS-049: package manager detection — depends on updater

Local HTTP convenience endpoints:

- HIS-028: `/quicktunnel` endpoint — convenience feature
- HIS-029: `/config` endpoint — debugging aid
- HIS-030: `/debug/pprof/*` endpoints — runtime profiling

Environment and privilege:

- HIS-050: UID detection — gates deferred diagnostic log path
- HIS-051: terminal detection — gates deferred updater behavior

ICMP proxy:

- HIS-069: ICMP proxy raw socket — specialized feature, CAP_NET_RAW
- HIS-070: ping group range check — Linux privilege gate
- HIS-071: ICMP source IP flags — ICMP configuration

Miscellaneous:

- HIS-061: `--pidfile` flag — optional systemd integration
- HIS-072: `hello_world` ingress listener — test/demo server
- HIS-073: gracenet socket inheritance — zero-downtime restart optimization
- HIS-074: process self-restart on update — depends on updater

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
