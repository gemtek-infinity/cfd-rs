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
- first-slice evidence exists
- partial local tests only

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
in `baseline-2026.2.0/old-impl/` and comparison against the current Rust HIS
surface in `crates/cloudflared-cli/` and `crates/cloudflared-config/`.

The frozen Go HIS surface uses direct syscalls, `os/exec` for systemd/SysV,
`fsnotify` for file watching, `net/http` for local metrics, and `lumberjack`
for log rotation. The current Rust HIS surface has config discovery and
credential loading (parity-backed), signal handling (functional parity),
and deployment evidence (intentional alpha divergence). All other host
interactions are absent.

### Config Discovery and Loading

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-001 | config search directory order | `config/configuration.go` `DefaultConfigSearchDirectories()` | search `~/.cloudflared`, `~/.cloudflare-warp`, `~/cloudflare-warp`, `/etc/cloudflared`, `/usr/local/etc/cloudflared` in order, check `config.yml` and `config.yaml` in each | cloudflared-config `discovery.rs` | audited, parity-backed | first-slice evidence exists | none recorded | parity compare tests, discovery fixture tests | high | Rust search order matches frozen baseline exactly |
| HIS-002 | config auto-create behavior | `config/configuration.go` `FindOrCreateConfigPath()` | create parent dir, create config at `/usr/local/etc/cloudflared/config.yml`, create `/var/log/cloudflared`, write minimal YAML with `logDirectory` | cloudflared-config `discovery.rs` | audited, parity-backed | first-slice evidence exists | none recorded | filesystem-effect tests, config creation golden tests | high | Rust implements auto-create with correct paths and minimal YAML |
| HIS-003 | config file YAML loading | `config/configuration.go` `ReadConfigFile()` | YAML decode with empty-file handling, `--config` flag override, strict-mode unknown-field warnings | cloudflared-config `raw_config.rs`, `normalized.rs` | audited, partial | first-slice evidence exists | open gap | config golden tests, unknown-field warning tests | medium | Rust loads YAML and tracks warnings but strict-mode double-parse not confirmed |
| HIS-004 | default path constants | `config/configuration.go` constants | `DefaultUnixConfigLocation=/usr/local/etc/cloudflared`, `DefaultUnixLogLocation=/var/log/cloudflared`, `DefaultConfigFiles=[config.yml, config.yaml]` | cloudflared-config `discovery.rs` | audited, parity-backed | first-slice evidence exists | none recorded | constant assertion tests | medium | all constants match |
| HIS-005 | HOME expansion | `config/configuration.go` and `homedir.Expand` | `~/` prefix expanded via HOME environment variable | cloudflared-config `discovery.rs` | audited, parity-backed | first-slice evidence exists | none recorded | HOME expansion tests | medium | implemented |

### Credentials and Lookup

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-006 | tunnel credentials JSON parsing | `credentials/credentials.go`, `connection/connection.go` | parse JSON with fields `AccountTag`, `TunnelSecret` (base64), `TunnelID` (UUID), `Endpoint` | cloudflared-config `credentials/mod.rs` | audited, parity-backed | first-slice evidence exists | none recorded | credential JSON parsing tests | high | all fields parsed correctly |
| HIS-007 | origin cert PEM parsing | `credentials/origin_cert.go` | parse PEM with `ARGO TUNNEL TOKEN` block, decode base64 to JSON with `zoneID`, `accountID`, `apiToken`, `endpoint` | cloudflared-config `credentials/mod.rs` | audited, parity-backed | first-slice evidence exists | none recorded | PEM decoding tests, fixture tests | high | implemented with FED endpoint detection |
| HIS-008 | credential search-by-ID | `cmd/cloudflared/tunnel/credential_finder.go` `searchByID` | search for `{TunnelID}.json` in origincert dir first, then each discovery directory | none | audited, absent | not present | open gap | credential search tests, directory traversal tests | high | blocks `tunnel run` without explicit `--credentials-file` |
| HIS-009 | origin cert search across dirs | `credentials/origin_cert.go` `FindDefaultOriginCertPath()` | search discovery directories for `cert.pem`, return first match | none | audited, absent | not present | open gap | cert search tests | high | needed for cert-based flows |
| HIS-010 | tunnel token compact format | `connection/connection.go` `TunnelToken` | JSON struct with short keys `a`, `s`, `t`, `e`, base64-encoded for `--token` flag | none | audited, absent | not present | open gap | token parse and roundtrip tests | high | needed for token-based service install |
| HIS-011 | credential file write with mode 0400 | `cmd/cloudflared/tunnel/subcommands.go` | write JSON with `os.O_CREATE` and `os.O_EXCL`, mode 0400, fail if file exists | none | audited, absent | not present | open gap | file creation tests, permission tests | medium | needed for `tunnel create` |

### Service Installation and Uninstall

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-012 | `service install` command (config-based) | `cmd/cloudflared/linux_service.go` `installLinuxService()` | read user config, validate `tunnel` + `credentials-file` keys, copy config to `/etc/cloudflared/config.yml`, build service args | none | audited, absent | not present | open gap | command tests, config validation tests, file copy tests | critical | lane-required |
| HIS-013 | `service install` command (token-based) | `cmd/cloudflared/linux_service.go`, `common_service.go` | parse token, validate, build args `["tunnel", "run", "--token", token]` | none | audited, absent | not present | open gap | command tests, token validation tests | critical | common install path |
| HIS-014 | systemd unit file generation | `cmd/cloudflared/linux_service.go` `installSystemd()` | write `cloudflared.service`, `cloudflared-update.service`, `cloudflared-update.timer` from Go templates to `/etc/systemd/system/` | none | audited, absent | not present | open gap | template generation tests, file write tests | critical | main service path |
| HIS-015 | systemd service enablement | `cmd/cloudflared/linux_service.go` | `systemctl enable`, `daemon-reload`, `start cloudflared.service`, optionally start update timer | none | audited, absent | not present | open gap | systemctl command tests | critical | service activation |
| HIS-016 | SysV init script generation | `cmd/cloudflared/linux_service.go` `installSysv()` | write init script to `/etc/init.d/cloudflared`, create start/stop symlinks in `/etc/rc*.d/` | none | audited, absent | not present | open gap | template tests, symlink tests | high | fallback for non-systemd hosts |
| HIS-017 | `service uninstall` command | `cmd/cloudflared/linux_service.go` `uninstallLinuxService()` | detect init system, stop + disable service, remove unit files or init script, daemon-reload; preserve config and credentials | none | audited, absent | not present | open gap | uninstall tests, file removal tests, preservation tests | critical | lane-required |
| HIS-018 | `--no-update-service` flag | `cmd/cloudflared/linux_service.go` | skip generation of `cloudflared-update.service` and timer during install | none | audited, absent | not present | open gap | flag tests | medium | install option |
| HIS-019 | service config directory | `cmd/cloudflared/linux_service.go` `ensureConfigDirExists()` | create `/etc/cloudflared/` if not present during install | none | audited, absent | not present | open gap | directory creation tests | high | install prerequisite |
| HIS-020 | config conflict detection | `cmd/cloudflared/linux_service.go` `buildArgsForConfig()` | if user config path != `/etc/cloudflared/config.yml` and service config exists, return error with remediation | none | audited, absent | not present | open gap | conflict detection tests | high | prevents silent config overwrite |

### Systemd and Init System

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-021 | systemd detection | `cmd/cloudflared/linux_service.go` `isSystemd()` | check `/run/systemd/system` existence | cloudflared-cli `runtime/deployment.rs` | audited, partial | weak | open gap | host-detection tests | high | Rust uses env vars not /run/systemd/system stat; detects for evidence only, not service management |
| HIS-022 | systemd service template exact content | `cmd/cloudflared/linux_service.go` templates | `Type=notify`, `TimeoutStartSec=15`, `Restart=on-failure`, `RestartSec=5s`, `--no-autoupdate` in ExecStart, `After=network-online.target` | none | audited, absent | not present | open gap | template content assertion tests | critical | exact template content is part of the operator contract |
| HIS-023 | SysV init script exact content | `cmd/cloudflared/linux_service.go` template | pidfile at `/var/run/$name.pid`, stdout to `/var/log/$name.log`, stderr to `/var/log/$name.err`, sources `/etc/sysconfig/$name` | none | audited, absent | not present | open gap | script content tests | high | fallback service path |

### Local HTTP Endpoints

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-024 | local HTTP metrics server | `metrics/metrics.go` | bind `localhost:0` (host) or `0.0.0.0:0` (container), try ports 20241-20245, ReadTimeout=10s, WriteTimeout=10s | none | audited, absent | not present | open gap | bind tests, port fallback tests | critical | no observability surface in Rust |
| HIS-025 | `/ready` JSON endpoint | `metrics/readiness.go` `ReadyServer` | JSON `{"status":200,"readyConnections":N,"connectorId":"uuid"}`, HTTP 200 if connections > 0, HTTP 503 otherwise | runtime readiness state machine only | audited, absent | not present | open gap | readiness HTTP tests, response shape tests | critical | Rust has internal readiness tracking but no HTTP endpoint |
| HIS-026 | `/healthcheck` endpoint | `metrics/metrics.go` | return `OK\n` as text/plain | none | audited, absent | not present | open gap | liveness probe tests | high | simple liveness check |
| HIS-027 | `/metrics` Prometheus endpoint | `metrics/metrics.go` `promhttp.Handler()` | Prometheus text format, `build_info` gauge with goversion/type/revision/version labels | none | audited, absent | not present | open gap | metrics format tests, build_info label tests | critical | monitoring integration |
| HIS-028 | `/quicktunnel` endpoint | `metrics/metrics.go` | JSON `{"hostname":"..."}` with quick tunnel URL | none | audited, absent | not present | open gap | quicktunnel response tests | medium | quick tunnel flow |
| HIS-029 | `/config` endpoint | orchestrator serving versioned config | JSON `{"version":N,"config":{ingress, warp-routing, originRequest}}` | none | audited, absent | not present | open gap | config endpoint tests | medium | remote config visibility |
| HIS-030 | `/debug/pprof/*` endpoints | `http.DefaultServeMux` pprof | binary pprof format, auth disabled (`trace.AuthRequest` returns true) | none | audited, absent | not present | open gap | pprof endpoint tests | low | debugging aid |
| HIS-031 | metrics bind address config | `metrics/metrics.go`, `--metrics` flag | `--metrics ADDRESS` flag overrides default | none | audited, absent | not present | open gap | flag tests | high | operator configuration |

### Diagnostics Collection

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-032 | `tunnel diag` CLI command | `diagnostic/` package, `tunnel/subcommands.go` | collect diagnostics bundle as ZIP with 11 jobs, toggleable via `--no-diag-*` flags | none | audited, absent | not present | open gap | command tests, ZIP output tests | high | operator diagnostics |
| HIS-033 | system information collector | `diagnostic/system_collector_linux.go` | collect memory, file descriptors, OS info, disk volumes; return `SystemInformationResponse` JSON | none | audited, absent | not present | open gap | system info tests, JSON shape tests | high | diagnostic bundle dependency |
| HIS-034 | tunnel state collector | `diagnostic/` and `/diag/tunnel` | collect tunnel ID, connector ID, active connections, ICMP sources; return `TunnelState` JSON | none | audited, absent | not present | open gap | tunnel state tests | high | diagnostic bundle dependency |
| HIS-035 | CLI configuration collector | `diagnostic/handlers.go` `/diag/configuration` | return `map[string]string` with `uid`, `log_file`, `log_directory`; exclude secrets | none | audited, absent | not present | open gap | handler tests, secret exclusion tests | medium | diagnostic info |
| HIS-036 | host log collector | `diagnostic/log_collector_host.go` | UID==0 and systemd: `journalctl -u cloudflared.service --since "2 weeks ago"`; otherwise: user log path; fallback `/var/log/cloudflared.err` | none | audited, absent | not present | open gap | log collection tests, privilege-based behavior tests | medium | host log diagnostics |
| HIS-037 | network traceroute collector | `diagnostic/` network collector | traceroute to `region{1,2}.v2.argotunnel.com` (IPv4/IPv6), default 5 hops, 5s timeout | none | audited, absent | not present | open gap | traceroute tests | medium | network diagnostics |
| HIS-038 | diagnostic instance discovery | `diagnostic/` metric port scanning | scan known ports 20241-20245 to find running instance | none | audited, absent | not present | open gap | port scan tests | medium | diagnostic client prerequisite |
| HIS-039 | `/diag/system` HTTP endpoint | `diagnostic/handlers.go` | system info JSON served on metrics server | none | audited, absent | not present | open gap | endpoint tests | high | served on local metrics server |
| HIS-040 | `/diag/tunnel` HTTP endpoint | `diagnostic/handlers.go` | tunnel state JSON served on metrics server | none | audited, absent | not present | open gap | endpoint tests | high | served on local metrics server |

### Watcher and Config Reload

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-041 | file watcher (inotify) | `watcher/file.go` | fsnotify watcher, triggers on Write events only, shutdown channel | none | audited, absent | not present | open gap | watch event tests, shutdown tests | critical | reload foundation |
| HIS-042 | config reload action loop | `cmd/cloudflared/app_service.go` `actionLoop()` | receive config updates on channel, create/update/remove services by hash comparison | none | audited, absent | not present | open gap | reload integration tests, hash comparison tests | critical | operator-expected behavior |
| HIS-043 | service lifecycle manager | `overwatch/app_manager.go` `AppManager` | add/remove services with hash-based change detection, shutdown old before starting new | none | audited, absent | not present | open gap | service lifecycle tests | high | reload depends on this |
| HIS-044 | remote config update | `orchestration/orchestrator.go` `UpdateConfig()` | version-monotonic update, start new origins before closing old, atomic proxy swap via `atomic.Value` | none | audited, absent | not present | open gap | version ordering tests, atomic swap tests | critical | edge-pushed config |
| HIS-045 | reload error recovery | watcher and orchestrator error paths | parse errors leave old service running, watch errors logged and continue, version downgrades rejected | none | audited, absent | not present | open gap | failure mode tests | high | operator safety |

### Updater

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-046 | `update` CLI command | `cmd/cloudflared/updater/update.go` | manual update with `--beta`, `--staging`, `--force`, `--version` flags; HTTP check to `update.argotunnel.com` | none | audited, absent | not present | open gap | command tests, HTTP mock tests | high | operator self-update |
| HIS-047 | auto-update timer | `cmd/cloudflared/updater/update.go` `AutoUpdater` | periodic check (default 24h), `--autoupdate-freq`, `--no-autoupdate` flags; disabled on Windows, terminal, package-managed | none | audited, absent | not present | open gap | timer tests, restriction tests | high | service auto-update |
| HIS-048 | update exit codes | `cmd/cloudflared/updater/update.go` | exit 11 = success (restart), exit 10 = failure, exit 0 = no update | none | audited, absent | not present | open gap | exit code tests | medium | systemd service integration |
| HIS-049 | package manager detection | `cmd/cloudflared/updater/update.go` | `.installedFromPackageManager` marker file or `BuiltForPackageManager` build tag disables auto-update | none | audited, absent | not present | open gap | marker detection tests | medium | update safety |

### Environment and Privilege

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-050 | UID detection | `diagnostic/handlers.go` `os.Getuid()` | UID stored in diagnostic config, UID==0 enables journalctl log path | none | audited, absent | not present | open gap | privilege behavior tests | medium | diagnostic privilege gate |
| HIS-051 | terminal detection | `cmd/cloudflared/updater/update.go` `isRunningFromTerminal()` | `term.IsTerminal(os.Stdout.Fd())` to distinguish interactive vs service; disables auto-update when terminal | none | audited, absent | not present | open gap | terminal detection tests | medium | update behavior gate |
| HIS-052 | OS-specific build tags | multiple platform files | `linux_service.go`, `system_collector_linux.go`, `collector_unix.go` with build tags | current Rust uses `cfg!(target_os)` for deployment evidence | audited, partial | weak | open gap | platform-specific build tests | medium | Rust has platform detection for evidence but not for platform-specific service behavior |

### Deployment Evidence

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-053 | deployment evidence vs host parity | deployment contract and runtime evidence | current deployment evidence is contract-level and honesty-oriented, not host-behavior parity; must not be mistaken for full HIS closure | cloudflared-cli `deployment_evidence.rs` | audited, intentional divergence | partial local tests only | intentional divergence | divergence note, evidence-scope tests | medium | Rust explicitly declares known gaps (`no-installer`, `no-systemd-unit`, etc.) |
| HIS-054 | binary path detection | `std::env::current_exe()` equivalent | runtime reports its own executable path | cloudflared-cli `deployment_evidence.rs` | audited, parity-backed | partial local tests only | none recorded | binary path tests | low | implemented |
| HIS-055 | glibc marker detection | deployment contract | check for `/lib64/ld-linux-x86-64.so.2`, `/lib/x86_64-linux-gnu/libc.so.6`, `/usr/lib64/libc.so.6` | cloudflared-cli `deployment.rs` | audited, parity-backed | partial local tests only | none recorded | glibc detection tests | low | implemented, specific to declared lane |

### Package Manager Scripts

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-056 | postinst.sh behavior | `postinst.sh` | create `/usr/local/bin/cloudflared` symlink, create `/usr/local/etc/cloudflared/`, touch `.installedFromPackageManager` | not applicable | not audited | not applicable | not applicable | packaging tests | low | packaging concern, not Rust binary behavior |
| HIS-057 | postrm.sh behavior | `postrm.sh` | remove `/usr/local/bin/cloudflared` symlink, remove `.installedFromPackageManager` marker | not applicable | not audited | not applicable | not applicable | packaging tests | low | packaging concern, not Rust binary behavior |

### Signal Handling and Graceful Shutdown

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-058 | SIGTERM/SIGINT shutdown | `signal/safe_signal.go`, `cmd/cloudflared/tunnel/signal.go` | `signal.Notify()` listens for SIGTERM and SIGINT, closes `graceShutdownC` channel, triggers graceful shutdown | cloudflared-cli `runtime/tasks/bridges.rs` | audited, parity-backed | partial local tests only | none recorded | signal handling tests | high | Rust uses tokio::signal::unix with ShutdownRequested command; functional parity |
| HIS-059 | `--grace-period` flag | `cmd/cloudflared/tunnel/cmd.go` | default 30 seconds; waits for in-progress requests before shutdown; controls `GracefulShutdown()` RPC on HTTP/2 connections | cloudflared-cli `runtime/types.rs` | audited, partial | weak | open gap | grace period flag tests, shutdown timing tests | critical | Rust internal default is 100ms, no CLI flag exposed; Go default is 30s; significant behavioral divergence |
| HIS-060 | double-signal immediate shutdown | `cmd/cloudflared/tunnel/signal.go` | second SIGTERM/SIGINT interrupts grace period wait, forces immediate exit | none | audited, absent | not present | open gap | double-signal tests | medium | operator escape hatch |
| HIS-061 | `--pidfile` flag | `cmd/cloudflared/tunnel/cmd.go` | optional; writes PID after tunnel connects (not on startup); triggered by `connectedSignal` in background goroutine | none | audited, absent | not present | open gap | pidfile creation tests, timing tests | medium | optional systemd integration |
| HIS-062 | token lock file | `token/token.go` | create `<token-path>.lock` with mode 0600 during token fetch; delete on release or SIGINT/SIGTERM; exponential backoff polling if lock exists (up to 7 iterations) | none | audited, absent | not present | open gap | lock file tests, signal cleanup tests, concurrency tests | high | prevents concurrent token fetch (AUTH-1736) |

### Logging and File Artifacts

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-063 | log file creation (`--logfile`) | `logger/create.go` | `--logfile` flag creates log file at specified path; `LogFile` config key | none | audited, absent | not present | open gap | log file creation tests | high | Rust writes to stderr only, no file appender |
| HIS-064 | log directory (`--log-directory`) | `logger/create.go`, `config/configuration.go` | `--log-directory` flag; auto-created by config discovery; default `/var/log/cloudflared` | cloudflared-config `discovery.rs` | audited, partial | first-slice evidence exists | open gap | log directory tests | high | Rust creates directory but does not write log files to it |
| HIS-065 | rolling log rotation | `logger/create.go`, lumberjack.v2 | automatic rotation when size exceeded: MaxSize=1MB, MaxBackups=5, MaxAge=0 (forever) | none | audited, absent | not present | open gap | rotation tests, size limit tests | high | no equivalent in Rust |
| HIS-066 | log file permissions | `logger/create.go` | files created with mode 0644, directories with mode 0744 | none | audited, absent | not present | open gap | permission assertion tests | medium | Rust auto-creates dirs but does not create log files |
| HIS-067 | `--log-format-output` flag | `logger/configuration.go` | JSON or text log format output selection | none | audited, absent | not present | open gap | format output tests | medium | Rust uses tracing_subscriber default format |
| HIS-068 | `--loglevel` and `--transport-loglevel` | `logger/configuration.go` | default `info`; separate `--transport-loglevel` for transport layer | none | audited, absent | not present | open gap | log level filter tests | high | no CLI-configurable log level in Rust |

### ICMP and Raw Sockets

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-069 | ICMP proxy raw socket | `ingress/icmp_linux.go` | `net.ListenPacket()` for ICMP/ICMPv6; creates raw socket for proxied ICMP echo requests | none | audited, absent | not present | open gap | raw socket tests, privilege tests | high | requires CAP_NET_RAW or ping_group membership |
| HIS-070 | ping group range check | `ingress/icmp_linux.go` | reads `/proc/sys/net/ipv4/ping_group_range`; verifies process GID is within range; logs warning if denied; silently disables ICMP if check fails | none | audited, absent | not present | open gap | privilege check tests, fallback tests | high | Linux-specific privilege gate |
| HIS-071 | ICMP source IP flags | `cmd/cloudflared/tunnel/configuration.go` | `--icmpv4-src` and `--icmpv6-src` flags (env: `TUNNEL_ICMPV4_SRC`, `TUNNEL_ICMPV6_SRC`); auto-detect by dialing 192.168.0.1:53 if unset | none | audited, absent | not present | open gap | flag tests, auto-detection tests | medium | ICMP proxy configuration |

### Local Test Server

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-072 | `hello_world` ingress service | `hello/hello.go`, `ingress/origin_service.go` | localhost TLS listener on auto-port (127.0.0.1:0); self-signed cert; routes `/`, `/uptime`, `/ws`, `/sse`, `/_health`; stops on `shutdownC` | cloudflared-config `ingress/types.rs` | audited, partial | weak | open gap | listener tests, route tests, TLS cert tests | medium | Rust parses `hello_world` config but has no listener or handler |

### Process Restart

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIS-073 | gracenet socket inheritance | `metrics/metrics.go`, `vendor/github.com/facebookgo/grace/gracenet/net.go` | metrics listeners registered via `gracenet.Net`; on auto-update restart, passes listener FDs to new process via `os.StartProcess()` with inherited environment | none | audited, absent | not present | open gap | socket inheritance tests | medium | Rust uses async runtime; may intentionally diverge |
| HIS-074 | process self-restart on update | `cmd/cloudflared/updater/update.go` | on exit code 11 with SysV: `gracenet.Net.StartProcess()` forks new process inheriting listener sockets; on systemd: service restart handled by unit config | none | audited, absent | not present | open gap | restart tests | medium | depends on updater implementation |

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
parsing (HIS-007), SIGTERM/SIGINT shutdown (HIS-058), binary path detection
(HIS-054), glibc marker detection (HIS-055).

Partial: config YAML loading (HIS-003, strict mode unconfirmed), systemd
detection (HIS-021, different method), deployment evidence (HIS-053,
intentional divergence), log directory creation (HIS-064, no file writer),
`hello_world` config parsing (HIS-072, no listener), OS build tags (HIS-052,
detection only), grace period (HIS-059, 100ms default vs 30s).

Missing: full service install/uninstall, systemd/SysV templates, local HTTP
metrics server, all HTTP endpoints, diagnostics collection, watcher/reload,
updater, ICMP proxy, log file creation, log rotation, token lock file,
credential search, double-signal handling, pidfile.

### Divergence records

Two HIS items are classified as intentional divergences:

- **HIS-053 (deployment evidence):** Rust deployment evidence is
  contract-level and honesty-oriented. It explicitly declares known gaps
  (`no-installer`, `no-systemd-unit`). This is intentional during alpha.

- **HIS-059 (grace period default):** Rust internal default is 100ms; Go
  default is 30s. This is documented as an `open gap` requiring a CLI flag,
  not as an intentional divergence — the `--grace-period` flag is absent.

Note: HIS-053 is the only true `intentional divergence` status. HIS-059 is
`open gap` despite having a partial Rust implementation (different default,
no flag).

No HIS evidence harnesses with machine-comparable artifacts exist yet. All
evidence is recorded in feature-group audit documents. Host-behavior capture
artifacts (filesystem effects, systemd template content, diagnostics output
shapes) are deferred to implementation stages.

### Gap ranking by priority

Critical gaps (lane-blocking):

- HIS-012 through HIS-017: service install and uninstall commands and systemd/SysV integration
- HIS-022: systemd service template exact content
- HIS-024: local HTTP metrics server
- HIS-025: `/ready` JSON endpoint
- HIS-027: `/metrics` Prometheus endpoint
- HIS-041: file watcher
- HIS-042: config reload action loop
- HIS-044: remote config update handling
- HIS-059: `--grace-period` flag (30s default, not exposed in Rust)

High gaps:

- HIS-008, HIS-009, HIS-010: credential search and lookup
- HIS-019, HIS-020: service config directory and conflict detection
- HIS-021: systemd detection method divergence
- HIS-023: SysV init script content
- HIS-026: `/healthcheck` endpoint
- HIS-031: metrics bind address config
- HIS-032 through HIS-034: diagnostic command and collectors
- HIS-039, HIS-040: diagnostic HTTP endpoints
- HIS-043: service lifecycle manager
- HIS-045: reload error recovery
- HIS-046, HIS-047: update command and auto-update
- HIS-058: SIGTERM/SIGINT shutdown (implemented, verify parity)
- HIS-062: token lock file
- HIS-063: log file creation (`--logfile`)
- HIS-064: log directory (directory exists, file writer missing)
- HIS-065: rolling log rotation
- HIS-068: `--loglevel` and `--transport-loglevel`
- HIS-069, HIS-070: ICMP raw socket and ping group check

Medium gaps:

- HIS-003: config strict-mode warnings
- HIS-010: tunnel token compact format
- HIS-011: credential file write mode
- HIS-028, HIS-029: quicktunnel and config endpoints
- HIS-035 through HIS-038: diagnostic sub-collectors
- HIS-048, HIS-049: update exit codes and package detection
- HIS-050, HIS-051, HIS-052: privilege and terminal detection
- HIS-060: double-signal immediate shutdown
- HIS-061: `--pidfile` flag
- HIS-066, HIS-067: log file permissions and format output
- HIS-071: ICMP source IP flags
- HIS-072: `hello_world` ingress listener
- HIS-073, HIS-074: gracenet socket inheritance and process restart

Low gaps:

- HIS-030: pprof endpoints
- HIS-054, HIS-055: deployment evidence details
- HIS-056, HIS-057: package manager scripts

## Immediate Work Queue

1. ~~inventory Linux service install and uninstall behavior~~ — done, see `service-installation.md`
2. ~~inventory local metrics, readiness, diagnostics endpoints~~ — done, see `diagnostics-and-collection.md`
3. ~~inventory diagnostics collector surfaces and output shapes~~ — done, see `diagnostics-and-collection.md`
4. ~~inventory watcher and reload behavior~~ — done, see `reload-and-watcher.md`
5. ~~inventory filesystem paths and side effects~~ — done, see `filesystem-and-layout.md`
6. ~~classify lane-relevant vs compatibility-only behaviors~~ — done, lane column in each feature-group doc
7. ~~create feature-group audit documents~~ — done, four documents created
8. ~~inventory signal handling and graceful shutdown~~ — done, see `reload-and-watcher.md`
9. ~~inventory logging file artifacts and rotation~~ — done, see `filesystem-and-layout.md`
10. ~~inventory ICMP proxy privilege surface~~ — done, see `diagnostics-and-collection.md`
11. ~~inventory process restart and socket inheritance~~ — done, see `reload-and-watcher.md`
