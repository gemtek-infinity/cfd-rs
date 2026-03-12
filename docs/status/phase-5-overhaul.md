# Phase 5 Overhaul Status

## Purpose

This document tracks the current execution status of the repository's Big Phase 5
overhaul work.

It does not own phase truth, lane truth, or promotion truth.

For those, the governing files remain:

- `docs/promotion-gates.md`
- `REWRITE_CHARTER.md`
- `STATUS.md`
- `docs/compatibility-scope.md`

For detailed execution planning, `FINAL_PHASE.md` wins.

This file exists to record:

- what Big Phase 5 execution work is active now
- what has already been established
- what remains unfinished
- which stage is currently in progress
- what the next bounded actions are

## Current Position

What exists now:

- accepted first-slice parity-backed config, credentials, and ingress behavior
- a narrow Rust executable surface centered on `validate`, `run`, `help`, and `version`
- a real runtime shell
- a partial transport, protocol, and proxy path
- deployment-proof and runtime-evidence work for the admitted lane
- partial Phase 5.1 wire and stream-serving work
- three live parity ledgers:
  - `docs/parity/cli/implementation-checklist.md`
  - `docs/parity/cdc/implementation-checklist.md`
  - `docs/parity/his/implementation-checklist.md`
- a repository execution plan for the overhaul in `FINAL_PHASE.md`

What does not exist yet:

- full CLI parity to the frozen baseline
- broad Cloudflare contract parity
- broad host and service interaction parity
- complete evidence-backed audit coverage across the three ledgers
- final-phase documentation truth across the repository
- the target crate map as the actual workspace structure
- production-alpha parity proof for the declared lane

## Final-Phase Structure

The overhaul is executed in three mandatory ordered stages:

1. audit
2. reconcile docs
3. refactor

That order is intentional.

We audit first so ownership boundaries are derived from upstream truth rather
than local preference.

We reconcile docs second so contributors are working from honest repository
truth.

We refactor third so the workspace structure follows audited parity surfaces
rather than guesses.

The final phase is organized around three primary parity domains:

- CLI
- CDC
- HIS

### CLI

This domain covers the blackbox command surface, including:

- command tree
- help and usage text
- flag names and aliases
- environment-variable bindings
- hidden and compatibility-only commands
- exit codes
- stdout and stderr placement
- formatting and spacing details

### CDC

This domain covers interactions between cloudflared and Cloudflare-managed
services and contracts, including:

- registration RPC and related registration content
- stream request and response contracts
- management and log-streaming contracts
- readiness and metrics contracts where externally relevant
- Cloudflare API interactions used by tunnel-related commands

### HIS

This domain covers interactions between cloudflared and the local host,
including:

- filesystem effects
- config discovery and file creation
- service and supervision behavior
- diagnostics collection
- watcher and reload behavior
- local endpoint exposure
- environment and privilege assumptions

## Tracking Documents

Primary execution and tracking documents:

- `FINAL_PLAN.md` — staged execution plan with sub-stage gates
- `FINAL_PHASE.md` — detailed execution reference (audit domains, evidence
  rules, refactor rules, risk register, contributor workflow)
- `docs/parity/cli/implementation-checklist.md`
- `docs/parity/cdc/implementation-checklist.md`
- `docs/parity/his/implementation-checklist.md`

Additional feature-group parity documents may be added under `docs/parity/`
when the master ledgers would otherwise become too dense to review effectively.

## Stage Status

### Stage 1: Audit

Status: **complete**

Outputs established now:

- CLI implementation checklist exists and is fully populated (32 rows)
- CLI feature-group audit documents exist:
  - `docs/parity/cli/root-and-global-flags.md`
  - `docs/parity/cli/tunnel-subtree.md`
  - `docs/parity/cli/access-subtree.md`
  - `docs/parity/cli/tail-and-management.md`
- CLI baseline evidence captures exist in `docs/parity/cli/captures/`:
  - `root-surface.txt` — root help, empty invocation, version
  - `tunnel-subtree.txt` — tunnel and all tunnel subcommand help
  - `access-subtree.txt` — access subtree and forward alias
  - `tail-management-service-update.txt` — tail, management, service, update
  - `error-and-compat.txt` — unknown commands, bad flags, proxy-dns, db-connect
  - `rust-current-surface.txt` — current Rust binary outputs for comparison
- CDC implementation checklist exists and is fully populated (44 rows)
- CDC feature-group audit documents exist:
  - `docs/parity/cdc/registration-rpc.md`
  - `docs/parity/cdc/stream-contracts.md`
  - `docs/parity/cdc/management-and-diagnostics.md`
  - `docs/parity/cdc/metrics-readiness-and-api.md`
- HIS implementation checklist exists and is fully populated (74 rows)
- HIS feature-group audit documents exist:
  - `docs/parity/his/service-installation.md`
  - `docs/parity/his/filesystem-and-layout.md`
  - `docs/parity/his/diagnostics-and-collection.md`
  - `docs/parity/his/reload-and-watcher.md`

Sub-stage status:

- Stage 1.1 (CLI audit): **complete**
- Stage 1.2 (CDC audit): **complete**
- Stage 1.3 (HIS audit): **complete**

Aggregate exit conditions:

- all three domains have complete implementation checklists: **yes** (150 rows total)
- major feature groups enumerated in dedicated documents: **yes** (12 feature-group docs)
- high-risk parity gaps identified and ranked across all three domains: **yes** — see
  "Cross-Domain Gap Ranking" below (32 critical, 62 high, 50 medium, 6 low)
- refactor target crate map justified from audited evidence: **yes** — see
  "Target Crate Map Justification" below
- document reconciliation list complete enough to execute: **yes** — see
  `FINAL_PLAN.md` § Complete Document Reconciliation Inventory
- intentional divergences recorded explicitly: **yes** (3 CLI + 2 CDC + 7 HIS)

All three audit sub-stages and the aggregate exit condition are satisfied.
Stage 1 is complete.

### Stage 2: Reconcile Docs

Status: **in progress** (Stage 2.1 complete)

Sub-stage status:

- Stage 2.1 (master repository truth): **complete**
- Stage 2.2 (scope, compatibility, governance): not started
- Stage 2.3 (historical phase and parity docs): not started
- Stage 2.4 (operator and contributor guidance): not started
- Stage 2.5 (AI instructions, skills, agent config): not started

Stage 2.1 outputs:

- root `README.md` created — honest about current state, gaps, and parity
  progress
- `STATUS.md` reduced to a short index with ledger-grounded truth
- `docs/README.md` updated with clear section groupings and parity links
- `docs/status/rewrite-foundation.md` reduced — removed duplication of lane
  and phase model owned by other governing docs
- `docs/status/active-surface.md` rewritten — replaced 200+ lines of
  phase-by-phase accretion with crate-grounded content that points to parity
  ledgers
- `docs/promotion-gates.md` reviewed — no changes needed (governing truth is
  accurate)

Required outputs remaining:

- rewritten top-level repository truth
- updated README and docs map
- updated phase and status wording
- updated crate-ownership wording
- target crate README content
- links from top-level docs to parity ledgers

This stage is mandatory implementation work, not cleanup.

### Stage 3: Refactor

Status: not started

Target crate map:

- `cfdrs-bin`
- `cfdrs-cli`
- `cfdrs-cdc`
- `cfdrs-his`
- `cfdrs-shared`

Refactor purpose:

- align ownership with audited parity domains
- reduce mixed responsibility in the current workspace
- make the repository legible to human contributors
- make future parity work land in the right crate by default

Refactor constraint:

- do not create target crates or begin ownership moves before the Stage 1 audit gate
  and the minimum Stage 2 documentation gate described in `FINAL_PHASE.md` are satisfied

## Cross-Domain Gap Ranking

This section consolidates the per-domain gap rankings from the three parity
ledgers into a single ranked inventory for implementation and refactor ordering.
It satisfies the Stage 1 aggregate exit condition requiring cross-domain
identification and ranking.

Priority counts across all three domains (150 total rows):

| Priority | CLI | CDC | HIS | Total |
| --- | --- | --- | --- | --- |
| Critical | 9 | 10 | 13 | 32 |
| High | 13 | 18 | 31 | 62 |
| Medium | 10 | 15 | 25 | 50 |
| Low | 0 | 1 | 5 | 6 |

### Tier 1 — Lane-blocking critical gaps (implementation-order priority)

These gaps block production-alpha on the declared Linux lane. Recommended
implementation order follows dependency chains, not alphabetical order.

1. **Registration wire encoding** — CDC-001, CDC-002: Cap'n Proto schema and
   binary encoding vs current JSON. All edge communication depends on this.
   Must be resolved before any CDC parity can be claimed.

2. **Stream framing and codec** — CDC-011, CDC-012, CDC-018: ConnectRequest
   and ConnectResponse wire framing, incoming stream round-trip. Depends on
   registration encoding resolution.

3. **Management and log-streaming** — CDC-023, CDC-024, CDC-026: management
   service routes, auth middleware, log streaming WebSocket. Entirely absent
   in Rust. Required for operator observability.

4. **Cloudflare REST API client** — CDC-033, CDC-034: tunnel CRUD and API
   response envelope. Entirely absent. Required for `tunnel create`, `tunnel
   list`, `tunnel delete`, and related commands.

5. **CLI command surface** — CLI-001, CLI-002, CLI-003: root invocation, help
   text, global flags. Current Rust exposes 4 commands vs 9 families and 1
   flag vs 50+. Blocks all user-facing parity.

6. **Tunnel command tree** — CLI-008, CLI-010, CLI-012: tunnel root behavior,
   create, run. Core tunnel lifecycle commands.

7. **Service install and uninstall** — HIS-012 through HIS-017, HIS-022:
   Linux service management and systemd template. Entirely absent. Required
   for the declared Linux lane.

8. **Local HTTP endpoints** — HIS-024, HIS-025, HIS-027: metrics server,
   ready endpoint, Prometheus metrics. Absent. Required for operator
   monitoring.

9. **Config reload and file watcher** — HIS-041, HIS-042, HIS-044: file
   watcher, reload action loop, remote config update. Absent. Required for
   long-running tunnel operation.

10. **Grace period shutdown** — HIS-059: `--grace-period` flag with 30s
    default. Not exposed in Rust CLI.

### Tier 2 — High gaps (next-priority implementation)

High gaps are individually documented in each domain's ledger. The
highest-impact high gaps across domains are:

- credential and token handling: HIS-008 through HIS-010, CDC-042, CDC-043
- edge discovery and protocol negotiation: CDC-021, CDC-022
- control stream lifecycle: CDC-019
- diagnostics command and collectors: HIS-032 through HIS-034, HIS-039,
  HIS-040
- access subtree: CLI-022 (6 subcommands and aliases)
- update command: CLI-006, HIS-046, HIS-047
- logging file artifacts: HIS-063 through HIS-065, HIS-068

### Per-domain gap details

For the complete per-domain ranked gap inventory, see:

- `docs/parity/cli/implementation-checklist.md` § Gap ranking by priority
- `docs/parity/cdc/implementation-checklist.md` § Gap ranking by priority
- `docs/parity/his/implementation-checklist.md` § Gap ranking by priority

## Target Crate Map Justification

The target crate map in `FINAL_PLAN.md` is justified by the audited parity
domains. Each target crate corresponds to a distinct ownership boundary
derived from the three audit domains, not from Go package structure or Rust
crate convenience.

| Target crate | Justification from audit evidence |
| --- | --- |
| `cfdrs-bin` | Process entrypoint, runtime composition, lifecycle orchestration. Owns the seam between CLI dispatch, CDC connections, and HIS host interactions. Not a parity domain itself — it composes the three domains. |
| `cfdrs-cli` | Owns the 32-row CLI parity surface: command tree, help text, flags, env bindings, exit codes, formatting. All 9 critical CLI gaps and 13 high CLI gaps land here. Current Rust CLI surface lives in `crates/cloudflared-cli/src/surface/`. |
| `cfdrs-cdc` | Owns the 44-row CDC parity surface: registration RPC, stream contracts, management service, log streaming, metrics and readiness contracts, Cloudflare API client. All 10 critical CDC gaps and 18 high CDC gaps land here. Wire encoding (Cap'n Proto binary vs JSON) is the single highest-risk gap in the entire rewrite. |
| `cfdrs-his` | Owns the 74-row HIS parity surface: service install and uninstall, filesystem layout, diagnostics collection, config reload and watcher, local endpoint exposure, privilege and environment assumptions. All 13 critical HIS gaps and 31 high HIS gaps land here. |
| `cfdrs-shared` | Narrowly admitted cross-domain types only. The audit evidence shows limited overlap between domains. Shared types are restricted to: error plumbing, config types used by both CDC and HIS, and credential types referenced by both CLI dispatch and CDC registration. Must not become a dump crate. |

The three parity domains (CLI, CDC, HIS) map cleanly to three ownership
crates because the frozen Go baseline organizes its behavior along these same
boundaries. The audit confirms that cross-domain coupling is limited to
credential and config types, which justifies a narrow shared crate rather
than a wide one.

## Known High-Risk Areas

This section is a quick-reference summary. For the ranked and cross-referenced
version, see "Cross-Domain Gap Ranking" above.

- registration RPC wire encoding (JSON vs Cap'n Proto)
- stream framing and codec parity (custom binary vs Cap'n Proto)
- management and log-streaming contracts (entirely absent in Rust)
- Cloudflare REST API client (entirely absent in Rust)
- exact CLI surface mismatch (hidden and compatibility command paths)
- Linux service install and uninstall (entirely absent in Rust)
- local HTTP metrics server and readiness endpoint (absent)
- config reload and file watcher (absent, explicitly declared)
- auto-update mechanism (absent)
- diagnostics collection and CLI command (absent)

## Anti-Drift Rules

- `docs/promotion-gates.md` owns phase and promotion truth
- `FINAL_PHASE.md` owns overhaul execution detail
- this file records current status only
- do not claim parity from Rust code shape alone
- do not let docs describe intended structure as current reality before it exists
- do not refactor before the owning parity surface is audited
- do not create target crates before the documented audit and documentation gates are satisfied
- do not use the shared crate as a dumping ground
- do not record vague progress such as “mostly done”
- do not leave divergences undocumented

## Progress Reporting Model

Progress should be reported in terms of:

- audited feature groups
- reconciled documents
- completed refactor waves
- parity-backed closures
- named remaining gaps

Avoid reporting progress only in terms of file count or code movement.

## Immediate Next Actions

Stage 2.1 (master repository truth) is complete. The next sub-stage is:

- Stage 2.2: Scope, compatibility, and governance review

The remaining Stage 2 sub-stages are:

1. ~~Stage 2.1: Master repository truth~~ (complete)
2. Stage 2.2: Scope, compatibility, and governance review
3. Stage 2.3: Historical phase and parity documents
4. Stage 2.4: Operator and contributor guidance
5. Stage 2.5: AI instructions, skills, and agent configuration

No Stage 2 sub-stage may be skipped or reordered.
