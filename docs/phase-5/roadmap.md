# Phase 5 Roadmap

This document is the normative implementation roadmap for Big Phase 5.
It is a roadmap, not a status file. [`STATUS.md`](../../STATUS.md) is the single tracked status source.

## Objective

Reach production-alpha on the declared Linux lane by closing the lane-required
behavior and contract gaps against frozen Go `2026.2.0`, then performing one
final performance-optimization architectural overhaul without regressing parity.

Logging compatibility across CLI flags/envs, local sinks, journald/systemd, and
upstream Cloudflare management services is part of the production-alpha gate.
It is not a deferred follow-up track.

## Milestones

The `milestone` column in [`roadmap-index.csv`](roadmap-index.csv) indicates the
milestone during which a row's exit evidence must be provided.  Rows whose
`blocked_by` dependency is already satisfied may be started at any time,
regardless of which milestone they close in.  The `blocked_by` column is the
real sequencing constraint; `milestone` is the evidence-closure target.

Rows that have already been proven and closed carry `status_bucket:
already_proven` with `blocked_by: none`, regardless of which milestone
originally owned them.

### 1. Program Reset

Goal:

- remove stale routing and historical audit surfaces
- keep [`STATUS.md`](../../STATUS.md) as the only tracked status file
- replace historical audit routing with [`docs/parity/source-map.csv`](../parity/source-map.csv)
- make [`Justfile`](../../Justfile) the normal human, AI, and CI command surface
- keep MCP routing tools and debtmap analysis tools separated in code, but require a debtmap-enabled operational server surface for agents
- ban contract-critical ad hoc literals from the initial hotspot files by moving them into owned contract modules

Exact rows:

- all already-proven or intentional-divergence rows mapped to `Program Reset` in [`docs/phase-5/roadmap-index.csv`](roadmap-index.csv)

Owner crates:

- `cfdrs-shared`, `cfdrs-bin`, `cfdrs-cli`, `cfdrs-cdc`, [`tools/mcp-cfd-rs`](../../tools/mcp-cfd-rs/)

Prerequisites:

- none

Required tests and checks:

- roadmap row coverage validator
- parity source-map coverage validator
- status contract validator
- charter/routing cleanup validator
- legacy cleanup validator
- architecture dependency validator
- Justfile contract validator
- contract-literal validator
- MCP routing/debtmap contract tests

Exit evidence:

- [`STATUS.md`](../../STATUS.md) is the only tracked status file
- the historical audit tree is gone
- [`docs/parity/source-map.csv`](../parity/source-map.csv) routes every parity row back to [`baseline-2026.2.0/`](../../baseline-2026.2.0/)
- [`Justfile`](../../Justfile) is the normal command surface for humans, agents, and CI
- debtmap-enabled MCP starts as the required operational surface
- the `--no-default-features` surface remains buildable only as a maintenance check

### 2. CDC Contract Foundation

Goal:

- close the lane-blocking Cloudflare contract and wire gaps first
- move CDC-owned protocol concerns toward `cfdrs-cdc`
- keep the upstream logging blockers explicit while CDC closes management-token and `/logs` contracts

Exact rows:

- all rows mapped to `CDC Contract Foundation` in [`docs/phase-5/roadmap-index.csv`](roadmap-index.csv)

Primary rows:

- `CDC-001` through `CDC-022`
- `CDC-040` through `CDC-043`

Owner crates:

- `cfdrs-cdc`
- `cfdrs-bin` only where runtime composition is still required during extraction
- `cfdrs-shared` for shared credential/config types already admitted

Prerequisites:

- `Program Reset`

Required tests:

- schema and wire fixture tests
- registration round-trip tests
- stream framing tests
- ingress-to-origin CDC path tests
- protocol negotiation and edge discovery tests
- management token and `/logs` contract tests for `CDC-023`, `CDC-024`, `CDC-026`, `CDC-038` once the slice reaches them

Exit evidence:

- registration and stream contracts match the frozen baseline on the admitted lane
- current CDC shortcuts are either removed or explicitly documented as temporary with a closure path
- all closed CDC rows show current evidence in the ledger and stable routing in [`docs/parity/source-map.csv`](../parity/source-map.csv)

### 3. Host and Runtime Foundation

Goal:

- close lane-required host interaction and local runtime surfaces
- make the long-lived process behavior honest and operable on Linux
- close the local half of the logging blocker set: file sinks, rotation, journald/systemd, and host collection

Exact rows:

- all rows mapped to `Host and Runtime Foundation` in [`docs/phase-5/roadmap-index.csv`](roadmap-index.csv)

Primary rows:

- `HIS-008` through `HIS-031`
- `HIS-041` through `HIS-045`
- `HIS-052` through `HIS-074` except rows explicitly deferred or non-lane

Owner crates:

- `cfdrs-his`
- `cfdrs-bin` for orchestration seams only
- `cfdrs-shared` for already-admitted shared config/credential types

Prerequisites:

- `Program Reset`
- `CDC Contract Foundation` where HIS surfaces depend on CDC contracts

Required tests:

- service install/uninstall tests
- template/content tests for systemd assets
- metrics/readiness/health endpoint tests
- watcher/reload and failure-recovery tests
- signal, grace-period, pidfile, logging, journald, and deployment contract tests
- host log-collection tests for `HIS-036`, `HIS-050`, `HIS-063` through `HIS-068`

Exit evidence:

- Linux host-facing lane contract is real for the admitted alpha path
- local sink and host-collection logging rows closed here are evidenced in the HIS ledger
- host/runtime rows closed here are evidenced in the HIS ledger

### 4. CLI Foundation

Goal:

- make the top-level CLI surface honest against the frozen baseline
- close root, help, global flag, and core tunnel lifecycle gaps
- close CLI-visible logging flags, aliases, defaults, and env bindings

Exact rows:

- all rows mapped to `CLI Foundation` in [`docs/phase-5/roadmap-index.csv`](roadmap-index.csv)

Primary rows:

- `CLI-001` through `CLI-021`
- `CLI-029` through `CLI-032`

Owner crates:

- `cfdrs-cli`
- `cfdrs-bin` only for execution handoff seams

Prerequisites:

- `Program Reset`
- `CDC Contract Foundation` and `Host and Runtime Foundation` where command behavior depends on those surfaces

Required tests:

- root invocation matrix
- exact help and usage snapshots
- flag/default/env binding tests
- tunnel command dispatch tests
- exit-code and stdout/stderr placement tests
- exact logging-flag and env-binding tests for `CLI-003`

Exit evidence:

- root and tunnel base surfaces no longer rely on alpha-only command shortcuts
- CLI logging entry semantics match the frozen baseline for the admitted lane
- CLI ledger reflects baseline-backed behavior for the closed rows

### 5. Command Family Closure

Goal:

- close remaining command families and the CDC/HIS surfaces they depend on
- finish the user-visible surface required for the declared Linux lane
- close the remaining cross-domain logging surface: tail, management token flow, and upstream `/logs` streaming

Exact rows:

- all rows mapped to `Command Family Closure` in [`docs/phase-5/roadmap-index.csv`](roadmap-index.csv)

Primary rows:

- `CLI-022` through `CLI-028`
- `CDC-023` through `CDC-039`
- command-linked HIS rows such as diagnostics, updater, and management-facing host collectors

Owner crates:

- `cfdrs-cli`
- `cfdrs-cdc`
- `cfdrs-his`

Prerequisites:

- `CLI Foundation`
- `CDC Contract Foundation`
- `Host and Runtime Foundation`

Required tests:

- access, tail, service, management, route, vnet, token, and compatibility-path tests
- REST API contract tests
- management WebSocket and auth tests
- diagnostic/update path tests where still lane-required
- end-to-end logging compatibility tests across `CLI-023`, `CLI-024`, `CDC-023`, `CDC-024`, `CDC-026`, and `CDC-038`

Exit evidence:

- remaining lane-required command families are no longer placeholder-only
- compatibility-only paths emit the exact baseline-visible behavior required
- upstream logging contracts are evidenced and cross-linked to the logging contract doc

### 6. Proof Closure

Goal:

- rerun the admitted parity and contract evidence after functional closure
- record final deferments, non-lane items, and intentional divergences explicitly
- rerun the full logging contract after CLI, CDC, and HIS closure

Exact rows:

- all rows mapped to `Proof Closure` in [`docs/phase-5/roadmap-index.csv`](roadmap-index.csv)

Owner crates:

- all five workspace crates as needed

Prerequisites:

- `CDC Contract Foundation`
- `Host and Runtime Foundation`
- `CLI Foundation`
- `Command Family Closure`

Required tests:

- full parity reruns for closed surfaces
- command/output snapshot suite
- contract tests across CLI, CDC, and HIS
- logging compatibility reruns across local and upstream surfaces
- failure/recovery and deployment reruns
- architecture dependency validator

Exit evidence:

- lane-required behavior claims are backed by current evidence rather than historical notes
- deferred and non-lane rows remain explicit and bounded
- production-alpha is not claimed yet; this is the pre-optimization baseline

### 7. Parity Revalidation

Goal:

- systematically re-audit all 150 parity rows against the frozen baseline after
  functional and proof closure
- fix stale evidence, tighten partial claims, and verify intentional divergences
  are properly documented with deferred boundaries
- produce a clean, auditable parity surface before the performance milestone

Exact rows:

- all 150 rows across CLI, CDC, and HIS; no new rows are introduced
- HIS-069 (ICMP raw-socket proxy) is the primary implementation target;
  `nix` is admitted for ICMP socket creation in `cfdrs-his`
- HIS-073 and HIS-074 (gracenet restart) remain intentional divergences with
  trait seams preserved for post-alpha implementation

Owner crates:

- all five workspace crates as needed

Prerequisites:

- `Proof Closure`

Required tests:

- full parity reruns for all closed surfaces
- ICMP proxy runtime tests (HIS-069) with `nix`
- command/output snapshot suite
- contract tests across CLI, CDC, and HIS
- logging compatibility reruns across local and upstream surfaces
- architecture dependency validator

Exit evidence:

- every closed row has current, source-grounded evidence in its domain ledger
- every intentional divergence has an explicit deferred boundary and rationale
- HIS-069 ICMP proxy is either closed or explicitly diverged with evidence
- no stale or historical-only parity claims remain
- the parity surface is clean enough to withstand a cold re-audit

### 8. Performance Architecture Overhaul

Goal:

- optimize the production-alpha critical path without changing the accepted behavior contract
- simplify or decouple hot-path architecture where the current structure adds copies, contention, or ownership friction

Exact rows:

- no new parity row set is introduced here; this milestone reruns the full admitted set after architectural optimization

Hot-path scope:

- transport/control-stream critical path
- proxy/origin dispatch critical path
- config/watch/reload runtime path where it affects long-lived process cost
- allocation, tasking, buffering, and lock-contention hotspots
- crate seams that currently force unnecessary coupling or copies in hot paths

Owner crates:

- any crate on the hot path, with `cfdrs-bin` acting only as composition owner

Prerequisites:

- `Parity Revalidation`

Required tests:

- deterministic perf probes or benchmarks for the admitted lane
- no-regression parity and contract reruns
- logging compatibility reruns after optimization changes on any admitted logging path
- failure/recovery and deployment reruns
- architecture dependency validator after optimization refactors

Exit evidence:

- no parity regressions
- crate-boundary contract remains intact or improves
- performance-critical hot paths are simpler or measurably better
- production-alpha readiness is claimed only after this rerun is green
