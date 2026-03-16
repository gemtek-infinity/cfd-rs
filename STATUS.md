# Cloudflared Rust Rewrite Status

## Active Snapshot

- lane: Linux only, `x86_64-unknown-linux-gnu`, quiche + BoringSSL,
  0-RTT required
- workspace version: `2026.2.0-alpha.202603`
- active milestone: `CLI Foundation`
- next milestone: `Command Family Closure`
- highest-risk blockers: `CLI-001`, `HIS-016`
- production-alpha logging blocker set:
  `CLI-023`, `CLI-024`, `CDC-023`, `CDC-024`, `CDC-026`, `CDC-038`, `HIS-036`
- behavior truth: [`baseline-2026.2.0/`](baseline-2026.2.0/)
- parity routing: [`docs/parity/source-map.csv`](docs/parity/source-map.csv)
- command surface: [`Justfile`](Justfile)
- status rule: this file is the only tracked status source for both humans
  and AI

## Current Reality

This repository is a real but partial Rust rewrite of `cloudflared`.

What exists now:

- `Program Reset`, `CDC Contract Foundation`, and `Host and Runtime Foundation`
  are complete
- live parity ledgers and source routing under [`docs/parity/`](docs/parity/)
- debtmap-enabled MCP server surface for bounded repo truth and routing
- repo-wide task entry through [`Justfile`](Justfile)

What does not exist yet:

- CLI Foundation behavioral closure for 11 of 19 milestone rows
- `CLI-001` service-mode runtime composition
- behavioral dispatch for `CLI-009` through `CLI-015`, `CLI-019`, `CLI-020`,
  and `CLI-032`
- management service, log streaming, and Cloudflare REST API client
- final `Performance Architecture Overhaul`

## Active Milestone

### CLI Foundation

- objective:
  make the root, help, global-flag, and core tunnel lifecycle surface honest
  against the frozen baseline
- current front edge:
  `CLI-001`, then `CLI-009` through `CLI-015`, `CLI-019`, `CLI-020`, `CLI-032`
- exit still requires:
  behavioral implementations for the remaining partial CLI Foundation rows

Next milestone after CLI Foundation closure:

- `Command Family Closure`

## Priority Rows

Tier 1 lane-blocking rows, in implementation order:

1. `CDC-001`, `CDC-002` — registration schema and wire encoding (closed)
2. `CDC-011`, `CDC-012` — stream schema and framing (closed)
3. `CLI-001`, `CLI-002`, `CLI-003` — root invocation, help text,
   global flags.
   `CLI-002` and `CLI-003` are closed; `CLI-001` remains partial.
4. `CLI-007`, `CLI-008`, `CLI-010`, `CLI-012` — service, tunnel root,
   create, run.
   `CLI-007` and `CLI-008` are closed; `CLI-010` is blocked on `CDC-033`;
   `CLI-012` is alpha-limited.
5. `HIS-012` through `HIS-015`, `HIS-017`, `HIS-022` — service
   install/uninstall and systemd templates.
   Closed; `HIS-016` remains partial.
6. `HIS-024`, `HIS-025`, `HIS-026`, `HIS-027` — local metrics,
   readiness, healthcheck, and Prometheus exposure.
   Closed; exact parity details still matter to the closure story.
7. `HIS-041`, `HIS-042`, `HIS-043`, `HIS-044`, `HIS-045` — file watcher,
   reload loop, service manager, remote config update, reload recovery.
   Closed; re-apply through `ReloadActionLoop` remains pending.
8. logging blocker set — `CLI-023`, `CLI-024`, `CDC-023`, `CDC-024`,
   `CDC-026`, `CDC-038`, `HIS-036`.
   Keep this set explicit while closing CLI Foundation rows.
9. `CDC-033`, `CDC-034` — Cloudflare REST API client and response envelope
10. `cloudflare-rs` remains gate-only for `CDC-033`, `CDC-034`, `CDC-038`
    and dependent CLI flows; no dependency admission during prep
11. final milestone: `Performance Architecture Overhaul` after proof closure
    reruns cleanly

## Parity Snapshot

Counts from the `Rust status now` column in each domain ledger.

| Domain | Total | Closed | Partial | Not audited | % Closed |
| --- | --- | --- | --- | --- | --- |
| CLI | 32 | 13 | 19 | 0 | 41% |
| CDC | 44 | 29 | 15 | 0 | 66% |
| HIS | 74 | 49 | 23 | 2 | 66% |
| **Total** | **150** | **91** | **57** | **2** | **61%** |

Closed breakdown:

- CLI: 12 `audited, parity-backed` + 1 `audited, intentional divergence`
  (`CLI-031`)
- CDC: 29 `audited, parity-backed`
- HIS: 44 `audited, parity-backed` + 4 `closed` +
  1 `audited, intentional divergence` (`HIS-053`)

## Test Snapshot

991 tests passing across 5 app crates:

- `cfdrs-bin`
- `cfdrs-cdc`
- `cfdrs-cli`
- `cfdrs-his`
- `cfdrs-shared`

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
- shared types stay in `cfdrs-shared` only when more than one top-level
  domain needs them
- `cfdrs-shared` owns log configuration types:
  `LogLevel`, `LogFormat`, `LogConfig`, `RollingConfig`, `FileConfig`,
  `ConsoleConfig`, `build_log_config`, permission constants
- `cfdrs-cli` owns logging flags, help text, aliases, and env bindings
- `cfdrs-his` owns local sinks, file rotation, journald/systemd behavior,
  host collection, and `LogSink` trait
- `cfdrs-cdc` owns management token scope, `/logs` protocol, upstream logging
  contracts, and wire-protocol `LogLevel`
- performance work must preserve these boundaries; it may optimize seams but
  must not collapse the workspace into a convenience monolith

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
