# Cloudflared Rust Rewrite Status

## Active Snapshot

- lane: Linux only, `x86_64-unknown-linux-gnu`, quiche + BoringSSL, 0-RTT required
- compatibility baseline: frozen Go `2026.2.0` in [`baseline-2026.2.0/`](baseline-2026.2.0/)
- parity routing baseline: [`docs/parity/source-map.csv`](docs/parity/source-map.csv)
- workspace version: `2026.2.0-alpha.202603`
- roadmap state: `Program Reset` complete; active implementation milestone: `CDC Contract Foundation`
- highest-risk blockers: `CDC-001`, `CDC-002`, `CDC-011`, `CDC-012`, `CDC-018`, `CLI-001`, `CLI-002`, `CLI-003`, `HIS-012` through `HIS-017`, `HIS-024`, `HIS-025`, `HIS-041`, `HIS-042`
- production-alpha logging blocker set: `CLI-003`, `CLI-023`, `CLI-024`, `CDC-023`, `CDC-024`, `CDC-026`, `CDC-038`, `HIS-036`, `HIS-050`, `HIS-063`, `HIS-064`, `HIS-065`, `HIS-067`, `HIS-068`
- status rule: this file is the only tracked status source for both humans and AI

## Current Reality

This repository is a real but partial Rust rewrite of `cloudflared`.

What exists now:

- `cfdrs-bin`: binary entrypoint, runtime composition, QUIC tunnel shell, Pingora seam, deployment/performance/failure evidence
- `cfdrs-cli`: CLI parsing for all 40+ baseline command paths, 40+ global flags, help, dispatch (stubs for most commands), and CLI-facing error/output types
- `cfdrs-cdc`: registration and stream contract types
- `cfdrs-his`: filesystem config discovery IO, credential lookup, service install/uninstall trait contracts, systemd/SysV template generation, metrics/readiness contracts backing a runtime-owned local listener, diagnostics collection stubs, file watcher and config reload trait contracts, signal handling, logging configuration types, updater stubs, ICMP proxy stubs, hello server stub, environment/privilege detection
- `cfdrs-shared`: config, credentials, ingress, discovery constants, error taxonomy, artifact conversion
- live parity ledgers, feature docs, and source routing under [`docs/parity/`](docs/parity/)
- frozen Go baseline in [`baseline-2026.2.0/`](baseline-2026.2.0/)
- debtmap-enabled MCP server surface for bounded repo truth and routing
- repo-wide task entry through [`Justfile`](Justfile)

What does not exist yet:

- Cap'n Proto registration RPC and full stream round-trip through origin
- management service, log streaming, Cloudflare REST API client, and management-token workflows
- broad CLI behavioral parity: root service-mode runtime, tunnel/access/tail/service/update behavioral implementations behind parsed stubs
- service install/uninstall: `CommandRunner` trait integration and command dispatch are wired and parity-tested; real host `systemctl` execution not yet verified end-to-end
- local HTTP endpoints: runtime now binds local `/ready`, `/healthcheck`, and `/metrics`; diagnostics, quicktunnel, config, and pprof endpoints remain pending
- config reload and file watcher: trait contracts exist; `notify`/`inotify` integration pending
- logging sinks: runtime now honors `--logfile`, `--log-directory`, `--log-format-output`, and global log level wiring; rolling rotation, journald/systemd output, and upstream management log streaming remain pending
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

- registration schema and wire encoding closure for `CDC-001` through `CDC-006`
- stream framing and round-trip closure for `CDC-011` through `CDC-018`
- baseline-backed CDC ownership in `cfdrs-cdc` rather than runtime-local shortcuts
- matching roadmap, source-map, and ledger evidence for every closed CDC row

Next milestone after CDC closure:

- `Host and Runtime Foundation`

## Priority Rows

Tier 1 lane-blocking rows, in implementation order:

1. `CDC-001`, `CDC-002` — registration schema and wire encoding
2. `CDC-011`, `CDC-012`, `CDC-018` — stream schema, framing, and round-trip
3. `CLI-001`, `CLI-002`, `CLI-003` — root invocation, help text, global flags
4. `CLI-008`, `CLI-010`, `CLI-012` — tunnel root behavior, create, run
5. `HIS-012` through `HIS-015`, `HIS-017`, `HIS-022` — service install/uninstall and systemd templates (command path wired; real host `CommandRunner` execution still needs end-to-end verification)
6. `HIS-024`, `HIS-025`, `HIS-027` — local metrics, readiness, and Prometheus exposure (runtime binding landed; remaining gaps are endpoint breadth and exact parity semantics)
7. `HIS-041`, `HIS-042`, `HIS-044` — file watcher, reload loop, remote config update (trait contracts exist; integration pending)
8. logging blocker set — `CLI-003`, `CLI-023`, `CLI-024`, `CDC-023`, `CDC-024`, `CDC-026`, `CDC-038`, `HIS-036`, `HIS-063`, `HIS-064`, `HIS-065`, `HIS-067`, `HIS-068`
9. `CDC-033`, `CDC-034` — Cloudflare REST API client and response envelope
10. `cloudflare-rs` remains gate-only for `CDC-033`, `CDC-034`, `CDC-038` and dependent CLI flows; no dependency admission during prep
11. final milestone: `Performance Architecture Overhaul` after proof closure reruns cleanly

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
- `cfdrs-cli` owns logging flags, help text, aliases, and env bindings
- `cfdrs-his` owns local sinks, file rotation, journald/systemd behavior, and host collection
- `cfdrs-cdc` owns management token scope, `/logs` protocol, and upstream logging contracts
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
