# Cloudflared Rust Rewrite Status

## Active Snapshot

- lane: Linux only, `x86_64-unknown-linux-gnu`, quiche + BoringSSL,
  0-RTT required
- workspace version: `2026.2.0-alpha.202603`
- active milestone: `Command Family Closure`
- next milestone: `Proof Closure`
- highest-risk blockers: `HIS-016`
- production-alpha logging blocker set:
  `CLI-023`, `CLI-024`, `CDC-026`, `HIS-036`
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

- CLI Foundation milestone is complete; `Command Family Closure` is next
- management service, log streaming, and Cloudflare REST API client
- final `Performance Architecture Overhaul`

## Active Milestone

### Command Family Closure

- objective:
  close remaining command families and the CDC/HIS surfaces they depend on;
  finish the user-visible surface required for the declared Linux lane;
  close the remaining cross-domain logging surface
- current front edge:
  `HIS-016`, `CLI-023`, `CDC-026`
- exit still requires:
  behavioral implementation for remaining partial CLI, CDC, and HIS rows
  mapped to `Command Family Closure` in the roadmap index

Previous milestone (`CLI Foundation`) is complete.

Next milestone after Command Family Closure:

- `Proof Closure`

## Priority Rows

Tier 1 lane-blocking rows, in implementation order:

1. `HIS-016` — SysV init script generation remains the last open Host and
   Runtime Foundation row and a proof-closure blocker
2. `CLI-023`, `CLI-024`, `CDC-026`, `HIS-036` —
   explicit cross-domain logging blocker set
3. `HIS-059`, `HIS-069`, `HIS-071`, `HIS-072`, `HIS-073`, `HIS-074` —
   remaining command-linked host/runtime rows

## Parity Snapshot

Counts from the `Rust status now` column in each domain ledger.

| Domain | Total | Closed | Partial | Not audited | % Closed |
| --- | --- | --- | --- | --- | --- |
| CLI | 32 | 25 | 7 | 0 | 78% |
| CDC | 44 | 39 | 5 | 0 | 89% |
| HIS | 74 | 49 | 23 | 2 | 66% |
| **Total** | **150** | **113** | **35** | **2** | **75%** |

Closed breakdown:

- CLI: 24 `audited, parity-backed` + 1 `audited, intentional divergence`
  (`CLI-031`)
- CDC: 39 `audited, parity-backed`
- HIS: 48 `audited, parity-backed` +
  1 `audited, intentional divergence` (`HIS-053`)

## Test Snapshot

1053 tests passing across 5 app crates:

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
