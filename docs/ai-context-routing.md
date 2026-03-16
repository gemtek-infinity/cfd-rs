# AI Context Routing

This file defines the minimum-file startup order, the MCP contract, and the
command-entry rule for the rewrite.

## Retrieval Order

1. if MCP is available, call `status_summary` first
2. if MCP is unavailable, read the `Active Snapshot` section in [`STATUS.md`](../STATUS.md)
3. identify the owning domain or policy boundary
4. use `phase5_priority`, `parity_row_details`, `domain_gaps_ranked`, `baseline_source_mapping`, `crate_surface_summary`, or `crate_dependency_graph` before widening to larger docs
5. load only the matching roadmap, ledger, feature doc, or policy file when the first MCP answer is insufficient
6. use frozen Go baseline code/tests first for behavior truth
7. use [`Justfile`](../Justfile) as the normal command entry surface for local execution, CI, and agent-directed commands

## Command-entry defaults

Prefer the repo-owned command surface over hand-crafted local ops.

- full validation: `just validate-pr`
- formatting only: `just fmt`
- focused validation: `just validate-governance`, `just validate-app`, `just validate-tools`, `just validate-debtmap`
- MCP smoke: `just mcp-smoke`, `just mcp-smoke-maintenance`
- parity artifact workflows: `just shared-behavior-capture`, `just shared-behavior-compare`

If a matching Just recipe exists, do not replace it with an ad hoc `cargo`, `python3 tools/...`, or `cargo run --manifest-path ...` command chain unless you are explicitly debugging the recipe or isolating a failure inside it.

Prefer checked-in generators and validators for derived artifacts. Do not hand-edit generated files such as [`docs/parity/source-map.csv`](parity/source-map.csv).

## Minimum Context

### Status Or Current Priority

Load first:

- [`STATUS.md`](../STATUS.md)

Load next only if needed:

- [`docs/phase-5/roadmap.md`](phase-5/roadmap.md)
- [`docs/phase-5/roadmap-index.csv`](phase-5/roadmap-index.csv)

### Scope, Lane, And Non-Negotiables

Load first:

- [`REWRITE_CHARTER.md`](../REWRITE_CHARTER.md)
- [`docs/promotion-gates.md`](promotion-gates.md)

### Parity Work

Load first:

- [`docs/parity/README.md`](parity/README.md)
- [`docs/parity/source-map.csv`](parity/source-map.csv)
- the relevant domain ledger:
  - [`docs/parity/cli/implementation-checklist.md`](parity/cli/implementation-checklist.md)
  - [`docs/parity/cdc/implementation-checklist.md`](parity/cdc/implementation-checklist.md)
  - [`docs/parity/his/implementation-checklist.md`](parity/his/implementation-checklist.md)

Load next only if needed:

- the matching feature-group document under [`docs/parity/`](parity/)
- [`docs/parity/logging-compatibility.md`](parity/logging-compatibility.md) for logging and management-log questions
- [`docs/phase-5/roadmap-index.csv`](phase-5/roadmap-index.csv)

### Design Pattern Or Dispatch Mechanism Question

Load first:

- [`docs/adr/0008-generic-dispatch-over-dyn-trait.md`](adr/0008-generic-dispatch-over-dyn-trait.md)

Load next only if needed:

- [`docs/engineering-standards.md`](../docs/engineering-standards.md) § 14
- [`docs/adr/0001-hybrid-concurrency-model.md`](adr/0001-hybrid-concurrency-model.md)
  for task ownership and concurrency shape questions

### Runtime Or Dependency Policy

Load first:

- [`docs/dependency-policy.md`](dependency-policy.md)
- [`docs/allocator-runtime-baseline.md`](allocator-runtime-baseline.md)

### Behavior Truth

Load first:

- [`baseline-2026.2.0/`](../baseline-2026.2.0/)
- [`docs/parity/source-map.csv`](parity/source-map.csv)

Never claim parity from Rust code shape alone.

## MCP Contract

### Core bundles

- `status-core`
- `phase5-roadmap`
- `parity-cli`
- `parity-cdc`
- `parity-his`
- `runtime-deps`
- `behavior-baseline`
- `crate-ownership`

### Core snapshots

- `governing-files`
- `status-active`
- `phase5-milestone`
- `scope-lane`
- `runtime-deps`
- `behavior-baseline`
- `crate-ownership`

### Core tools

- `find_governance`
- `find_behavior_truth`
- `search_paths`
- `grep_paths`
- `list_paths`
- `get_context_bundle`
- `get_context_brief`
- `get_context_snapshot`
- `read_file`
- `read_file_lines`
- `file_metadata`
- `status_summary`
- `phase5_priority`
- `parity_row_details`
- `domain_gaps_ranked`
- `baseline_source_mapping`
- `crate_surface_summary`
- `crate_dependency_graph`

### Debtmap extension tools

- `debtmap_top_hotspots`
- `debtmap_file_summary`
- `debtmap_touched_files_review`
- `debtmap_code_smells`
- `debtmap_function_complexity`
- `debtmap_unified_analysis`
- `debtmap_ci_gate`

### Editor Default Surface

- [`.vscode/mcp.json`](../.vscode/mcp.json) starts the required debtmap-enabled MCP surface through `just mcp-run`
- [`tools/mcp-cfd-rs/Cargo.toml`](../tools/mcp-cfd-rs/Cargo.toml) keeps debtmap in the default feature set for normal MCP startup
- the `--no-default-features` surface is maintenance-only and must not be treated as the normal agent startup target

## MCP Routing

### Startup and status (no parameters)

- use `status_summary` as the default startup entry for repo truth and per-domain parity progress (closed, partial, absent counts for CLI, CDC, HIS)
- use `phase5_priority` for the current lane-blocking queue
- use `crate_dependency_graph` for the workspace dependency graph and architecture-policy verdict

### Parity and milestone work

- use `parity_row_details` when you already know the ledger row ID — returns combined ledger and roadmap detail
- use `domain_gaps_ranked` when you need bounded ranked work inside one domain — includes partial vs absent breakdown so you can prioritize rows that are already started
- use `baseline_source_mapping` to jump from a row ID to the frozen baseline source area and exact parity feature doc
- use `crate_surface_summary` to get one crate's ownership, surface, and allowed dependencies

### Context routing (curated, no file reads)

- use `get_context_bundle` for a curated narrow context bundle keyed by a common repository question type
- use `get_context_brief` for a compact first-read brief of a curated bundle
- use `get_context_snapshot` for a compact source-backed snapshot of a core rewrite routing question

### File access (repo-boundary enforced)

- use `read_file` to read a repo file with truncation and repo-boundary enforcement
- use `read_file_lines` to read a specific line range from a repo file
- use `file_metadata` to get metadata (kind, size, line count) for a repo path

### Search (scoped, bounded)

- use `find_governance` to search governance and policy files for grounded hits
- use `find_behavior_truth` to search frozen behavior and parity sources for grounded hits
- use `search_paths` to search specific repo-relative files or directories for grounded hits
- use `grep_paths` to regex search across repo-relative files or directories, returning matched lines with paths and line numbers
- use `list_paths` to list repo paths under a directory with optional recursion and extension filtering

### Debtmap (cognitive-load analysis)

Debtmap is always available in the required MCP surface.
Use `debtmap_*` once the task is localized to hotspot, review, or refactor work.

- use `debtmap_top_hotspots` for the top cognitive-load hotspot files (repo-wide or bounded by path prefix) — use for refactor triage, not as always-on context
- use `debtmap_file_summary` for a focused debtmap summary of one file — includes per-function complexity, code smells, TODO locations, and long-function line numbers
- use `debtmap_touched_files_review` to score a provided list of touched files for bounded cognitive-load review
- use `debtmap_code_smells` to detect code smells in a single file using AST analysis
- use `debtmap_function_complexity` to get per-function complexity breakdown for a single file
- use `debtmap_unified_analysis` for full unified debtmap analysis for deep structural review
- use `debtmap_ci_gate` to evaluate CI gate rules against the repo or a bounded file set — blocking violations must be fixed before merge; each violation includes path, line, score, and a fix suggestion

### Debtmap score categories

| Score range | Category | Priority | Action |
| --- | --- | --- | --- |
| < 15.0 | negligible | low | ignore |
| 15.0–29.99 | reviewable | medium | review when in the file |
| 30.0–44.99 | hotspot | high | reduce when touched; blocks CI gate |
| 45.0–74.99 | high_hotspot | high | refactor now; blocks CI gate |
| >= 75.0 | critical_hotspot | critical | refactor now; blocks CI gate |

### Debtmap CI gate rules

Blocking (must fix before merge):

- function or file score >= 30.0
- god_object_score >= 45.0
- debt_density > 50.0 per 1K LOC
- function cyclomatic complexity >= 31
- function cognitive complexity >= 25

Warning (monitor, non-blocking):

- score 15.0–29.99 (medium priority)
- god_object_score < 45.0
- coupling classification `highly_coupled` or `Hub`
- function cyclomatic 21–30 or cognitive 15–24

The `debtmap_ci_gate` tool returns the thresholds in every response so agents
do not need to memorize them.
The Justfile `validate-debtmap` recipe enforces the same score >= 30.0 gate
using the debtmap CLI JSON output.

## MCP Maintenance Mode

Enter MCP maintenance mode whenever a change touches:

- `tools/mcp-cfd-rs*`
- MCP tool contracts
- MCP bundle or snapshot names
- MCP-facing routing docs

During maintenance mode:

- do not rely on MCP answers until the debtmap-enabled operational surface rebuilds and smoke-starts
- keep the `--no-default-features` surface green as a maintenance check, but it does not unblock normal MCP use on its own
- use `just mcp-smoke` and `just mcp-smoke-maintenance` as the normal smoke entrypoints

## Local Handoff

[`GCFGR.md`](../GCFGR.md) (root of workspace, gitignored) is the mandatory local file for preserving
session state across context-window compactions and conversation resumptions.

It is NOT canonical repository truth — [`STATUS.md`](../STATUS.md) wins on any conflict.

### When to read GCFGR.md

- at every cold start and conversation resumption
- after any context-window compaction event
- when something feels off: a claim seems stale, a file path is wrong, or a
  ledger count does not match expectations

### When to write GCFGR.md

- before ending a session that produced non-trivial progress
- when the context window is approaching capacity
- after any milestone, ledger, or blocker change

### Required sections

The file uses a fixed section order optimized for fast AI context recovery.
Sections must appear in this exact order:

1. **Instant Context** — branch, commit, test count, workspace version,
   validate-pr status. One glance tells the agent where it is.
2. **Active Work** — what is in progress RIGHT NOW: the current row or task,
   what step is next, what the last completed step was. This is the most
   important section for resumption fidelity.
3. **Blockers and Constraints** — hard constraints that caused past mistakes
   or wasted cycles. Things the agent MUST NOT forget: `!Send` constraints,
   dependency decisions, wire-format gotchas, debtmap thresholds.
4. **Ledger Snapshot** — domain totals (closed/partial/divergence). Quick
   numerical reality check.
5. **Decisions Log** — architectural and dependency decisions made during the
   session, with one-line rationale. Prevents re-litigating settled questions.
6. **Session Mutations** — files touched this session with one-line summaries.
   Lets the agent verify what actually changed versus what it thinks changed.
7. **Architecture Invariants** — crate dependency directions. Rarely changes
   but prevents cross-domain violations.
8. **Validation Entry** — the exact Just commands. Prevents the agent from
   inventing ad-hoc cargo chains.

### Anti-drift rules

- GCFGR.md must stay gitignored (enforced by `validate-governance`)
- when GCFGR.md and [`STATUS.md`](../STATUS.md) disagree, [`STATUS.md`](../STATUS.md) wins
- do not duplicate stable governance into GCFGR.md — reference the governing
  file instead of copying its content
- keep the file under 200 lines; if it grows beyond that, compress older
  session mutations and decisions
