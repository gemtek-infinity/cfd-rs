# Cloudflared Rust Rewrite Status

## Active Snapshot

- lane: Linux only, `x86_64-unknown-linux-gnu`, quiche + BoringSSL,
  0-RTT required
- workspace version: `2026.2.0-alpha.202603`
- active milestone: `Parity Revalidation`
- next milestone: `Performance Architecture Overhaul`
- highest-risk blockers: none (all 150 rows closed)
- production-alpha logging blocker set: cleared
- behavior truth: [`baseline-2026.2.0/`](baseline-2026.2.0/)
- parity routing: [`docs/parity/source-map.csv`](docs/parity/source-map.csv)
- command surface: [`Justfile`](Justfile)
- status rule: this file is the only tracked status source for both humans
  and AI

## Current Reality

This repository is a real but partial Rust rewrite of `cloudflared`.

What exists now:

- `Program Reset`, `CDC Contract Foundation`, `Host and Runtime Foundation`,
  `CLI Foundation`, `Command Family Closure`, and `Proof Closure` are complete
- all 150 parity rows are closed with current evidence in domain ledgers
- roadmap index fully reconciled: 142 `already_proven`, 6 `intentional_divergence`,
  2 `non_lane`
- live parity ledgers and source routing under [`docs/parity/`](docs/parity/)
- debtmap-enabled MCP server surface for bounded repo truth and routing
- repo-wide task entry through [`Justfile`](Justfile)

What does not exist yet:

- `HIS-073` and `HIS-074` (gracenet restart) are closed as intentional
  divergences with trait seams preserved for post-alpha
- `Parity Revalidation` and final `Performance Architecture Overhaul`

## Active Milestone

### Parity Revalidation

- objective:
  systematically re-audit all 150 parity rows against the frozen baseline;
  fix stale evidence, tighten partial claims, verify intentional divergences
  are properly documented; produce a clean auditable parity surface before
  the performance milestone
- current front edge:
  milestone entry; no rows have been re-audited yet
- exit still requires:
  full re-audit pass across all 150 rows with current source-grounded evidence;
  every intentional divergence has an explicit deferred boundary and rationale

Previous milestone (`Proof Closure`) is complete.

Next milestone after Parity Revalidation:

- `Performance Architecture Overhaul`

## Priority Rows

No lane-blocking rows remain. All 150 parity rows are closed.

## Parity Snapshot

Counts from the `Rust status now` column in each domain ledger.

| Domain | Total | Closed | Partial | Not audited | % Closed |
| --- | --- | --- | --- | --- | --- |
| CLI | 32 | 32 | 0 | 0 | 100% |
| CDC | 44 | 44 | 0 | 0 | 100% |
| HIS | 74 | 74 | 0 | 0 | 100% |
| **Total** | **150** | **150** | **0** | **0** | **100%** |

Closed breakdown:

- CLI: 30 `audited, parity-backed` + 2 `audited, intentional divergence`
  (`CLI-030`, `CLI-031`)
- CDC: 44 `audited, parity-backed`
- HIS: 68 `audited, parity-backed` +
  4 `audited, intentional divergence` (`HIS-030`, `HIS-053`,
  `HIS-073`, `HIS-074`) +
  2 `non_lane` (`HIS-056`, `HIS-057`)

## Test Snapshot

1253 tests passing across 5 app crates:

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
