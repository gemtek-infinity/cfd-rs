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

Status: **complete**

Sub-stage status:

- Stage 2.1 (master repository truth): **complete**
- Stage 2.2 (scope, compatibility, governance): **complete**
- Stage 2.3 (historical phase and parity docs): **complete**
- Stage 2.4 (operator and contributor guidance): **complete**
- Stage 2.5 (AI instructions, skills, agent config): **complete**

Stage 2.5 outputs:

- `.github/copilot-instructions.md` updated — added `FINAL_PLAN.md` and
  `FINAL_PHASE.md` to governing files list; added parity work routing section
  with domain identification, ledger links, and cross-domain gap ranking
  pointer
- `AGENTS.md` updated — added parity ledgers, `FINAL_PLAN.md`,
  `FINAL_PHASE.md`, and `docs/deployment-notes.md` to "Use the right file"
  routing; replaced stale first-slice async constraint with general
  synchronous-first preference; added parity ledger update rule to working
  rules; added `CONTRIBUTING.md`, `docs/code-style.md`, and
  `docs/engineering-standards.md` to routing table
- `SKILLS.md` updated — replaced stale "first-slice bias" section with
  general "default code preferences"; added parity ledger identification
  and update steps to porting workflow; added ledger links
- `.github/instructions/rust.instructions.md` updated — replaced "for
  first-slice work" scoping with general synchronous-first preference
  (Tokio runtime is now active)
- `.github/instructions/markdown.instructions.md` reviewed — no changes
  needed (already accurate and consistent)
- `docs/ai-context-routing.md` updated — added "Parity audit, implementation,
  and gap review" task routing section with domain ledger, feature-group doc,
  and execution doc pointers

Stage 2 wrap-up outputs:

- `CONTRIBUTING.md` created — human contributor guide with build instructions,
  code style and engineering standards pointers, parity evidence requirements,
  implementation workflow, frozen-input rules, document hierarchy, and
  AI-assisted contribution guidance
- `docs/code-style.md` updated — added quick-reference summary table linking
  all 30 rules; changed header from "human-facing reference document" to
  "reference document for both human contributors and AI agents"
- `docs/engineering-standards.md` updated — added quick-reference summary
  table linking all 13 standards; changed header from "human-facing reference
  document" to "reference document for both human contributors and AI agents"
- `README.md` updated — Contributing section now links to `CONTRIBUTING.md`
  with quick-start pointers to code-style, engineering-standards, parity
  index, and AI routing docs
- `docs/README.md` updated — Rust Coding References section now includes
  `CONTRIBUTING.md` and updated descriptions with rule counts
- `AGENTS.md` updated — added `CONTRIBUTING.md`, `docs/code-style.md`, and
  `docs/engineering-standards.md` to "Use the right file" routing

Stage 2.4 outputs:

- `docs/deployment-notes.md` updated — added parity ledger cross-references,
  linked known deployment gaps to HIS checklist rows (HIS-012 through HIS-017,
  HIS-046, HIS-047, HIS-063 through HIS-065), linked operational caveats to
  CDC checklist rows (CDC-001, CDC-002, CDC-011, CDC-012, CDC-018, HIS-041,
  HIS-042)
- `docs/parity/README.md` created — parity navigation index with domain
  summaries, document links, cross-domain gap count table, and source-of-truth
  note
- `docs/README.md` updated — added parity index link and new Operator Guidance
  section with deployment-notes link
- crate READMEs reviewed — no crate-root READMEs exist in current workspace;
  target crate READMEs are Stage 3 work
- `crates/cfdrs-shared/tests/README.md` reviewed — already updated in
  Stage 2.3, no further changes needed

Stage 2.3 outputs:

- `docs/first-slice-freeze.md` marked as historical record — first slice
  complete and parity-backed, broader parity governed by domain ledgers
- `docs/status/first-slice-parity.md` marked as historical record — first
  slice only, broader parity tracked by domain ledgers
- `docs/status/porting-rules.md` marked as partially superseded — first
  implementation gate satisfied, first slice complete, broader porting governed
  by final-phase program
- `crates/cfdrs-shared/tests/README.md` updated — broader parity
  tracking pointer added
- `tools/first_slice_parity.py` reviewed — already clearly scoped to
  first-slice surface, no changes needed

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

Stage 2.2 outputs:

- `REWRITE_CHARTER.md` reviewed — no changes needed (governing truth is
  accurate)
- `docs/compatibility-scope.md` reviewed — no changes needed
- `docs/build-artifact-policy.md` reviewed — no changes needed
- `docs/dependency-policy.md` updated — deferred dependency buckets reconciled
  against current workspace manifests; three admitted slices (config,
  async, observability) marked as admitted with ongoing rules preserved;
  Protocol/Wire remains genuinely deferred
- `docs/allocator-runtime-baseline.md` updated — removed embedded phase
  number from scaffold honesty rule; language is now phase-agnostic
- `docs/go-rust-semantic-mapping.md` updated — "Current Scaffold Application"
  renamed and updated to reflect operational state; "First Slice Constraint"
  generalized to "Synchronous And Deterministic Work"; garbled end-of-file
  paragraph ordering fixed
- `docs/adr/0001-hybrid-concurrency-model.md` updated — "Current Scaffold
  Implication" replaced with "Current Operational State" reflecting that
  Tokio and hybrid concurrency primitives are now active
- `docs/adr/0002-transport-tls-crypto-lane.md` reviewed — no changes needed
- `docs/adr/0003-pingora-critical-path.md` updated — acknowledged that
  `pingora-http` is admitted in workspace dependencies; removed stale
  deferred follow-ups for ADRs that already exist
- `docs/adr/0004-fips-in-alpha-definition.md` reviewed — no changes needed
- `docs/adr/0005-deployment-contract.md` reviewed — no changes needed
- `docs/adr/ADR-0006-standard-format-and-workspace-dependency-admission.md`
  reviewed — no changes needed

Required outputs remaining:

- target crate README content (Stage 3, after crate skeletons exist)

All five Stage 2 sub-stages are complete. Stage 2 documentation reconciliation
exit conditions are satisfied:

- repository-wide wording is aligned with the final-phase program
- stale ownership language has been removed or replaced
- parity documents are linked from top-level docs
- the repository can be read by a human contributor without relying on tribal knowledge
- the document set is accurate enough to support the refactor without ambiguity

### Stage 3: Refactor

Status: **in progress**

Sub-stage status:

- Stage 3.1 (scope pruning and divergence triage): **complete**
- Stage 3.2 (atomic refactor into five crates): **complete**
- Stage 3.3 (tooling and automation hardening): not started

Stage 3.2 progress:

- Wave 0 (preparation): **complete** — target crate map confirmed in 4
  documents, migration slice boundaries confirmed from audit evidence,
  code-level migration boundary mapping verified
- Wave 1 (workspace skeleton creation): **complete** — 5 target crates
  created with Cargo.toml, README.md, src/lib.rs; old crates intact;
  workspace Cargo.toml updated; tests, clippy, and debtmap_ci_gate pass
- Wave 2 (CLI ownership move): **complete** — surface/ module (types.rs,
  output.rs, error.rs, parse.rs, help.rs) moved from cloudflared-cli to
  cfdrs-cli; visibility changed from pub(crate) to pub for cross-crate access;
  execute logic inlined into cfdrs-bin main.rs
- Wave 3 (binary and runtime composition move): **complete** — combined with
  Wave 4 scope; all remaining cloudflared-cli code (main.rs, runtime/,
  startup/, protocol.rs, transport/, proxy/, tests/) moved to cfdrs-bin;
  cloudflared-cli retired from workspace
- Wave 4 (CDC ownership move): **complete** — registration.rs and stream.rs
  moved from cloudflared-proto to cfdrs-cdc; all cloudflared_proto:: imports
  updated to cfdrs_cdc::; cloudflared-proto retired from workspace
- Wave 5 (HIS ownership move): **complete** — ownership declared in cfdrs-his
  README; discovery.rs moved to cfdrs-his; shared types (config, credentials,
  ingress, error taxonomy) moved to cfdrs-shared; cloudflared-config dissolved
- Wave 6 (shared extraction): **complete** — ownership declared in cfdrs-shared
  README; shared types moved to cfdrs-shared; cloudflared-config dissolved
- Wave 7 (retirement): **complete** — cloudflared-core removed from workspace
  (empty, no dependents); cloudflared-proto removed from workspace (code moved
  to cfdrs-cdc); CI workflow updated to reference new crate names

Stage 3.1 outputs:

- `docs/status/stage-3.1-scope-triage.md` created — scope boundary document
  with lane definition, classification scheme, and confirmed refactor scope
- all 150 parity ledger rows classified: 108 lane-required, 37 deferred,
  3 compatibility-only, 2 non-lane
- non-lane items named: HIS-056 and HIS-057 (packaging scripts, not binary
  behavior)
- compatibility-only items named: CLI-025 (proxy-dns), CLI-026 (db-connect),
  CLI-027 (classic tunnels) — require error stubs, not working implementations
- deferred items named with rationale: 6 CLI, 4 CDC, 27 HIS — including
  SysV init (ADR-0005 — systemd governs alpha), diagnostics subsystem,
  updater subsystem, ICMP proxy, and convenience endpoints
- scope classification sections added to all three parity ledgers
- confirmed refactor scope: 111 surfaces (108 lane-required + 3 error stubs)
  mapped to five target crates

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
| `cfdrs-cli` | Owns the 32-row CLI parity surface: command tree, help text, flags, env bindings, exit codes, formatting. All 9 critical CLI gaps and 13 high CLI gaps land here. Current Rust CLI surface lives in `crates/cfdrs-cli/src/`. |
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

Stage 3.2 (atomic refactor into five crates) is complete. All 8 waves
finished: workspace restructured from 4-crate layout to 5-crate target
layout (cfdrs-bin, cfdrs-cdc, cfdrs-cli, cfdrs-his, cfdrs-shared).
Retired crates: cloudflared-cli, cloudflared-proto, cloudflared-core,
cloudflared-config. CI workflow updated. Config modules reorganized
under `cfdrs-shared/src/config/` subdirectory. All documentation
reconciled to reference new crate names.

The next bounded action is:

- Stage 3.3: Tooling and automation hardening

Stage 3.3 work inventory:

1. review `.debtmap.toml` — verify analysis covers new crate paths (current
   pattern-based config appears sufficient)
2. review `tools/first_slice_parity.py` and `tools/first_slice_go_capture/` —
   mark as first-slice-scoped historical tooling if no longer applicable
3. verify MCP server (`tools/mcp-cfd-rs/`) routes and context bundles reflect
   the actual crate map (workspace-member snapshot, ownership routing)
4. verify parity capture harnesses are buildable and runnable in the new
   workspace structure
5. verify `rustfmt.toml` applies correctly across all crates
6. confirm build, test, clippy, and fmt commands work correctly across
   the full workspace

Preconditions already satisfied by Stage 3.2:

- CI workflows (on-pr-push.yml, on-pr-merge.yml) already reference correct
  crate names
- Copilot instructions and AI routing docs already reflect actual crate paths
- no active docs reference obsolete crate names

The remaining Stage 3 sub-stages are:

1. ~~Stage 3.1: Scope pruning and divergence triage~~ (complete)
2. ~~Stage 3.2: Atomic refactor into five crates~~ (complete)
3. Stage 3.3: Tooling and automation hardening

No Stage 3 sub-stage may be skipped or reordered.
