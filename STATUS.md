# Cloudflared Rust Rewrite Status

## Active Snapshot

- lane: Linux only, `x86_64-unknown-linux-gnu`, quiche + BoringSSL, 0-RTT required
- compatibility baseline: frozen Go `2026.2.0` in [`baseline-2026.2.0/`](baseline-2026.2.0/)
- parity routing baseline: [`docs/parity/source-map.csv`](docs/parity/source-map.csv)
- workspace version: `2026.2.0-alpha.202603`
- roadmap state: `Program Reset` complete; active implementation milestone: `CDC Contract Foundation`
- highest-risk blockers: `CLI-001`, `HIS-016`
- production-alpha logging blocker set: `CLI-023`, `CLI-024`, `CDC-023`, `CDC-024`, `CDC-026`, `CDC-038`, `HIS-036`
- status rule: this file is the only tracked status source for both humans and AI

## Current Reality

This repository is a real but partial Rust rewrite of `cloudflared`.

What exists now:

- `cfdrs-bin`: binary entrypoint, runtime composition with concrete `TransportService` enum (no trait objects), QUIC tunnel shell with datagram dispatch and session management, Pingora seam, config file watcher bridged into async runtime, deployment/performance/failure evidence
- `cfdrs-cli`: CLI parsing for all 40+ baseline command paths, 40+ global flags, help, dispatch (stubs for most commands), and CLI-facing error/output types
- `cfdrs-cdc`: full registration schema types (TunnelAuth, ClientInfo, ConnectionOptions, ConnectionDetails, ConnectionError with retry semantics, ConnectionResponse union, RPC contract types for SessionManager and ConfigurationManager), feature flag categorization, filtering, and selector (`build_feature_list`), stream contract types and metadata constants, CDC-owned Cap'n Proto wire codec (registration, unregister, and stream request/response encode/decode, runtime-wired in lifecycle.rs and proxy), datagram session types and wire marshal/unmarshal (V2 and V3), edge address management types (AddrSet, Region, Regions with two-region failover), protocol constants (stream signatures, TLS server names, ALPN, edge discovery DNS), management token JWT parsing (`parse_management_token`, `ManagementTokenClaims`) matching Go `UnsafeClaimsWithoutVerification`, Cap'n Proto generated bindings from frozen baseline schemas (`tunnelrpc.capnp` and `quic_metadata_protocol.capnp`)
- `cfdrs-his`: filesystem config discovery IO, credential lookup, service install/uninstall trait contracts, systemd/SysV template generation, metrics/readiness contracts backing a runtime-owned local listener, diagnostics collection types and handlers, file watcher and config reload seams, `NotifyFileWatcher` using `notify::RecommendedWatcher` with write-only filtering, signal handling, `ConnectedSignal` one-shot type matching Go `signal.Signal` with `sync.Once` for pidfile timing parity, `TokenLock` with 7-iteration exponential backoff and stale lock deletion matching Go `token.lock`, logging configuration types, updater stubs, ICMP proxy stubs, hello server stub, environment/privilege detection, `ManagedService` trait and generic `ServiceManager<S>` with hash-based change detection matching Go overwatch `AppManager`, channel-driven `ReloadActionLoop::run()` matching Go `actionLoop()`, versioned `InMemoryConfigOrchestrator` with monotonic version enforcement matching Go `Orchestrator.UpdateConfig()`
- `cfdrs-shared`: config, credentials, ingress, discovery constants, error taxonomy, artifact conversion, log configuration types (`LogLevel`, `LogFormat`, `LogConfig`, `RollingConfig`, `FileConfig`, `ConsoleConfig`, `build_log_config`)
- live parity ledgers, feature docs, and source routing under [`docs/parity/`](docs/parity/)
- frozen Go baseline in [`baseline-2026.2.0/`](baseline-2026.2.0/)
- debtmap-enabled MCP server surface for bounded repo truth and routing
- repo-wide task entry through [`Justfile`](Justfile)

What does not exist yet:

- Cap'n Proto RPC dispatch: CDC-007 (unregisterConnection), CDC-008 (updateLocalConfiguration), CDC-009 (registerUdpSession/unregisterUdpSession), CDC-010 (updateConfiguration) dispatch layers closed; all control-stream operations use raw `capnp::serialize` wire path
- management service, log streaming, Cloudflare REST API client, and management-token workflows
- broad CLI behavioral parity: root service-mode runtime, tunnel/access/tail/service/update behavioral implementations behind parsed stubs
- service install/uninstall: `CommandRunner` trait integration and command dispatch are wired and parity-tested; real host `systemctl` execution not yet verified end-to-end
- local HTTP endpoints: runtime now binds local `/ready`, `/healthcheck`, `/metrics`, `/config`, and `/diag/configuration` via axum with `prometheus-client` registry, baseline-backed container bind mode, Go `ConnTracker` connection counting, and full 19-metric Prometheus name inventory; quicktunnel, `/diag/system`, `/diag/tunnel`, and real pprof endpoints remain pending
- config reload and file watcher: reload action loop with channel-driven `run()` matching Go `actionLoop()`, `ManagedService` trait and generic `ServiceManager<S>` with hash-based dedup matching Go overwatch, versioned `InMemoryConfigOrchestrator` with monotonic version enforcement; `NotifyFileWatcher` using `notify::RecommendedWatcher` with write-only filtering and closure-based callbacks is wired and parity-tested; runtime watcher integration in cfdrs-bin is wired — `spawn_config_watcher()` bridges the blocking watcher into the async runtime via `spawn_blocking`, `ConfigFileChanged` command reports changes, `shutdown_flag()` enables async cancellation; re-apply path through `ReloadActionLoop` remains pending
- logging sinks: local sink surface parity-backed — `--logfile`, `--log-directory`, `--log-format-output`, global log level, bounded file rotation with backup-count enforcement, conditional `tracing_journald` layer, `sd_notify::notify` `READY=1`; local output format intentionally differs from Go zerolog (upstream format parity is CDC-026); upstream management `/logs` streaming remains pending
- ICMP proxy, hello server, graceful restart: trait stubs exist; real implementations pending
- performance-architectural overhaul of the final admitted hot paths

## Active Milestone

### CDC Contract Foundation

Current objective:

- replace JSON/custom wire shortcuts with baseline-backed CDC contracts
- close the lane-blocking registration and stream gaps first
- keep CLI and HIS work unblocked only where CDC dependencies are already explicit
- keep the logging blocker set explicit while CDC closes the management-token and `/logs` contracts

Current milestone exit requires:

- registration schema and wire encoding closure for `CDC-001` through `CDC-006` (closed)
- stream framing and round-trip closure for `CDC-011` through `CDC-018` (closed)
- remaining CDC Contract Foundation gaps: none — all CDC Contract Foundation rows closed
- baseline-backed CDC ownership in `cfdrs-cdc` rather than runtime-local shortcuts
- matching roadmap, source-map, and ledger evidence for every closed CDC row

Next milestone after CDC closure:

- `Host and Runtime Foundation`

## Priority Rows

Tier 1 lane-blocking rows, in implementation order:

1. `CDC-001`, `CDC-002` — registration schema and wire encoding (closed)
2. `CDC-011`, `CDC-012` — stream schema and framing (closed)
3. `CLI-001`, `CLI-002`, `CLI-003` — root invocation, help text, global flags (CLI-002 and CLI-003 closed; CLI-001 blocked on HIS-043)
4. `CLI-007`, `CLI-008`, `CLI-010`, `CLI-012` — service, tunnel root, create, run (CLI-007 and CLI-008 closed; CLI-010 and CLI-012 blocked on CDC)
5. `HIS-012` through `HIS-015`, `HIS-017`, `HIS-022` — service install/uninstall and systemd templates (closed; HIS-016 SysV fallback still partial; real host `CommandRunner` execution still needs end-to-end verification)
6. `HIS-024`, `HIS-025`, `HIS-026`, `HIS-027` — local metrics, readiness, healthcheck, and Prometheus exposure (closed; container bind mode, Go ConnTracker connection counting, exact healthcheck parity, and full 19-metric name inventory)
7. `HIS-041`, `HIS-042`, `HIS-043`, `HIS-044`, `HIS-045` — file watcher, reload loop, service manager, remote config update, reload recovery (HIS-041, HIS-042, HIS-043, HIS-044, HIS-045 closed; HIS-041 runtime watcher wired in cfdrs-bin, re-apply path through ReloadActionLoop pending)
8. logging blocker set — `CLI-023`, `CLI-024`, `CDC-023`, `CDC-024`, `CDC-026`, `CDC-038`, `HIS-036` (CLI-003, HIS-050, HIS-063, HIS-064, HIS-065, HIS-067, HIS-068 closed)
9. `CDC-033`, `CDC-034` — Cloudflare REST API client and response envelope
10. `cloudflare-rs` remains gate-only for `CDC-033`, `CDC-034`, `CDC-038` and dependent CLI flows; no dependency admission during prep
11. final milestone: `Performance Architecture Overhaul` after proof closure reruns cleanly

## Parity Progress Summary

Counts from the `Rust status now` column in each domain ledger.

| Domain | Total | Closed | Partial | Not audited | % Closed |
| --- | --- | --- | --- | --- | --- |
| CLI | 32 | 13 | 19 | 0 | 41% |
| CDC | 44 | 29 | 15 | 0 | 66% |
| HIS | 74 | 49 | 23 | 2 | 66% |
| **Total** | **150** | **91** | **57** | **2** | **61%** |

Closed breakdown:

- CLI: 12 `audited, parity-backed` + 1 `audited, intentional divergence` (CLI-031)
- CDC: 29 `audited, parity-backed`
- HIS: 44 `audited, parity-backed` + 4 `closed` + 1 `audited, intentional divergence` (HIS-053)

Test suite: 870 tests passing across 5 app crates (`cfdrs-bin`, `cfdrs-cdc`, `cfdrs-cli`, `cfdrs-his`, `cfdrs-shared`).

## Architecture Contract

Allowed crate dependency direction:

- `cfdrs-bin -> cfdrs-cli, cfdrs-cdc, cfdrs-his, cfdrs-shared`
- `cfdrs-cli -> cfdrs-shared`
- `cfdrs-cdc -> cfdrs-shared`
- `cfdrs-his -> cfdrs-shared`
- `cfdrs-shared` must not depend on domain crates
- CLI, CDC, and HIS must not depend on each other directly

Ownership rules:

- CLI parity work lands in `cfdrs-cli`
- Cloudflare contract work lands in `cfdrs-cdc`
- host/runtime interaction work lands in `cfdrs-his`
- shared types stay in `cfdrs-shared` only when more than one top-level domain needs them
- `cfdrs-shared` owns log configuration types (`LogLevel`, `LogFormat`, `LogConfig`, `RollingConfig`, `FileConfig`, `ConsoleConfig`, `build_log_config`, permission constants) — see ADR-0007
- `cfdrs-cli` owns logging flags, help text, aliases, and env bindings
- `cfdrs-his` owns local sinks, file rotation, journald/systemd behavior, host collection, and `LogSink` trait
- `cfdrs-cdc` owns management token scope, `/logs` protocol, upstream logging contracts, and wire-protocol `LogLevel`
- performance work must preserve these boundaries; it may optimize seams but must not collapse the workspace into a convenience monolith

## Canonical Links

- scope and non-negotiables: [`REWRITE_CHARTER.md`](REWRITE_CHARTER.md)
- roadmap: [`docs/phase-5/roadmap.md`](docs/phase-5/roadmap.md)
- roadmap row map: [`docs/phase-5/roadmap-index.csv`](docs/phase-5/roadmap-index.csv)
- parity index: [`docs/parity/README.md`](docs/parity/README.md)
- parity source routing: [`docs/parity/source-map.csv`](docs/parity/source-map.csv)
- logging contract: [`docs/parity/logging-compatibility.md`](docs/parity/logging-compatibility.md)
- CLI ledger: [`docs/parity/cli/implementation-checklist.md`](docs/parity/cli/implementation-checklist.md)
- CDC ledger: [`docs/parity/cdc/implementation-checklist.md`](docs/parity/cdc/implementation-checklist.md)
- HIS ledger: [`docs/parity/his/implementation-checklist.md`](docs/parity/his/implementation-checklist.md)
- phase model and promotion rules: [`docs/promotion-gates.md`](docs/promotion-gates.md)
- AI routing: [`docs/ai-context-routing.md`](docs/ai-context-routing.md)
- command surface: [`Justfile`](Justfile)
