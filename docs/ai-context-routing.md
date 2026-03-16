# AI Context Routing

Canonical AI and Copilot routing contract.
Use the smallest context bundle that answers the question.

## Retrieval Order

1. Read `GCFGR.md` first if it exists.
   [`STATUS.md`](../STATUS.md) wins on any conflict.
2. If MCP is available, call `status_summary` first.
3. If MCP is unavailable, inaccessible, or the client does not expose
   `status_summary`, say what is missing, then read the `Active Snapshot`
   section in [`STATUS.md`](../STATUS.md).
4. Identify the owning domain or policy boundary.
5. Use the smallest matching MCP tool or leaf doc next.
6. Use frozen Go baseline code and tests first for behavior truth.
7. Use [`Justfile`](../Justfile) as the normal command surface.

## Command-entry defaults

Prefer the repo-owned command surface over hand-crafted local ops.

- full validation: `just validate-pr`
- formatting only: `just fmt`
- focused validation: `just validate-governance`, `just validate-app`,
  `just validate-tools`, `just validate-debtmap`
- MCP smoke: `just mcp-smoke`, `just mcp-smoke-maintenance`
- parity artifact workflows: `just shared-behavior-capture`,
  `just shared-behavior-compare`

If a matching Just recipe exists, do not replace it with an ad hoc `cargo`,
`python3 tools/...`, or `cargo run --manifest-path ...` command chain unless
you are explicitly debugging the recipe or isolating a failure inside it.

Prefer checked-in generators and validators for derived artifacts.
Do not hand-edit generated files such as
[`docs/parity/source-map.csv`](parity/source-map.csv).

## Minimum Context

| Question | Open first | Open next only if needed |
| --- | --- | --- |
| current status or priority | [`STATUS.md`](../STATUS.md) | [`docs/phase-5/roadmap.md`](phase-5/roadmap.md), [`docs/phase-5/roadmap-index.csv`](phase-5/roadmap-index.csv) |
| scope, lane, non-negotiables | [`REWRITE_CHARTER.md`](../REWRITE_CHARTER.md), [`docs/promotion-gates.md`](promotion-gates.md) | matching policy doc |
| parity work | [`docs/parity/README.md`](parity/README.md), [`docs/parity/source-map.csv`](parity/source-map.csv), owning ledger | matching feature doc, [`docs/parity/logging-compatibility.md`](parity/logging-compatibility.md), [`docs/phase-5/roadmap-index.csv`](phase-5/roadmap-index.csv) |
| design or dispatch pattern | [`docs/adr/0008-generic-dispatch-over-dyn-trait.md`](adr/0008-generic-dispatch-over-dyn-trait.md) | [`docs/engineering-standards.md`](engineering-standards.md), [`docs/adr/0001-hybrid-concurrency-model.md`](adr/0001-hybrid-concurrency-model.md) |
| runtime or dependency policy | [`docs/dependency-policy.md`](dependency-policy.md), [`docs/allocator-runtime-baseline.md`](allocator-runtime-baseline.md) | leaf policy or ADR |
| behavior truth | [`baseline-2026.2.0/`](../baseline-2026.2.0/), [`docs/parity/source-map.csv`](parity/source-map.csv) | relevant parity doc or owning tests |

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

- [`.vscode/mcp.json`](../.vscode/mcp.json) starts the required
  debtmap-enabled MCP surface through `just mcp-run`
- [`tools/mcp-cfd-rs/Cargo.toml`](../tools/mcp-cfd-rs/Cargo.toml) keeps
  debtmap in the default feature set for normal MCP startup
- the `--no-default-features` surface is maintenance-only and must not be
  treated as the normal agent startup target

## MCP Routing

### Startup and status (no parameters)

- use `status_summary` as the default startup entry for repo truth and
  per-domain parity progress
- use `phase5_priority` for the current lane-blocking queue
- use `crate_dependency_graph` for the workspace dependency graph and
  architecture-policy verdict

### Parity and milestone work

- use `parity_row_details` when you already know the ledger row ID
- use `domain_gaps_ranked` when you need bounded ranked work inside one domain
- use `baseline_source_mapping` to jump from a row ID to frozen baseline
  sources and feature docs
- use `crate_surface_summary` for one crate's ownership and allowed surface

### Context routing (curated, no file reads)

- use `get_context_bundle` for curated narrow context bundles
- use `get_context_brief` for compact first-read briefs
- use `get_context_snapshot` for compact source-backed routing snapshots

### File access (repo-boundary enforced)

- use `read_file` to read a repo file with truncation and boundary enforcement
- use `read_file_lines` for a specific line range
- use `file_metadata` for kind, size, and line count

### Search (scoped, bounded)

- use `find_governance` for governance and policy files
- use `find_behavior_truth` for frozen behavior and parity sources
- use `search_paths` for bounded repo-relative search
- use `grep_paths` for regex search across bounded repo-relative paths
- use `list_paths` to list repo paths under a directory

### Debtmap (cognitive-load analysis)

Debtmap is always available in the required debtmap-enabled MCP surface.
Use `debtmap_*` once the task is localized to hotspot, review, or refactor
work.

- use `debtmap_top_hotspots` for hotspot triage
- use `debtmap_file_summary` for one-file complexity detail
- use `debtmap_touched_files_review` for bounded review of touched files
- use `debtmap_code_smells` for one-file smell detection
- use `debtmap_function_complexity` for per-function complexity breakdown
- use `debtmap_unified_analysis` for deep structural review
- use `debtmap_ci_gate` for merge-blocking debtmap rules

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

- do not rely on MCP answers until the debtmap-enabled operational surface
  rebuilds and smoke-starts
- keep the `--no-default-features` surface green as a maintenance check,
  but it does not unblock normal MCP use on its own
- use `just mcp-smoke` and `just mcp-smoke-maintenance` as the normal smoke
  entrypoints

## Local Handoff

`GCFGR.md` (root of workspace, gitignored) is the mandatory local file for
preserving session state across context-window compactions and conversation
resumptions.

It is NOT canonical repository truth — [`STATUS.md`](../STATUS.md) wins on any
conflict.

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
   validate-pr status
2. **Active Work** — current row or task, next step, last completed step
3. **Blockers and Constraints** — hard constraints and must-not-forget rules
4. **Ledger Snapshot** — domain totals for closed, partial, divergence
5. **Decisions Log** — architectural and dependency decisions with rationale
6. **Session Mutations** — touched files and one-line summaries
7. **Architecture Invariants** — crate dependency directions
8. **Validation Entry** — exact Just commands

### Anti-drift rules

- GCFGR.md must stay gitignored (enforced by `validate-governance`)
- when GCFGR.md and [`STATUS.md`](../STATUS.md) disagree,
  [`STATUS.md`](../STATUS.md) wins
- do not duplicate stable governance into GCFGR.md — reference the governing
  file instead of copying its content
- keep the file under 200 lines; if it grows beyond that, compress older
  session mutations and decisions
