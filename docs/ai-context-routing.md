# AI Context Routing

This file defines the minimum-file startup order and the MCP contract for the rewrite.

## Retrieval Order

1. if MCP is available, call `status_summary` first
2. if MCP is unavailable, read the `Active Snapshot` section in `STATUS.md`
3. identify the owning domain or policy boundary
4. use `phase5_priority`, `parity_row_details`, `domain_gaps_ranked`, `baseline_source_mapping`, `crate_surface_summary`, or `crate_dependency_graph` before widening to larger docs
5. load only the matching roadmap, ledger, or policy file when the first MCP answer is insufficient
6. use frozen Go baseline code/tests first for behavior truth

## Minimum Context

### Status or current priority

Load first:

- `STATUS.md`

Load next only if needed:

- `docs/phase-5/roadmap.md`
- `docs/phase-5/roadmap-index.csv`

### Scope, lane, and non-negotiables

Load first:

- `REWRITE_CHARTER.md`
- `docs/promotion-gates.md`

### Parity work

Load first:

- `docs/parity/README.md`
- the relevant domain ledger:
  - `docs/parity/cli/implementation-checklist.md`
  - `docs/parity/cdc/implementation-checklist.md`
  - `docs/parity/his/implementation-checklist.md`

Load next only if needed:

- the matching feature-group document under `docs/parity/`
- `docs/phase-5/roadmap-index.csv`

### Runtime or dependency policy

Load first:

- `docs/dependency-policy.md`
- `docs/allocator-runtime-baseline.md`

### Behavior truth

Load first:

- `baseline-2026.2.0/old-impl/`
- `baseline-2026.2.0/design-audit/`

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

- `.vscode/mcp.json` starts the required debtmap-enabled MCP surface with `--features debtmap`
- `tools/mcp-cfd-rs/Cargo.toml` keeps debtmap in the default feature set for normal MCP startup
- the `--no-default-features` surface is maintenance-only and must not be treated as the normal agent startup target

## MCP Routing

- use `status_summary` as the default startup entry for repo truth
- use `phase5_priority` for the current lane-blocking queue
- use `parity_row_details` when you already know the ledger row ID
- use `domain_gaps_ranked` when you need bounded ranked work inside one domain
- use `baseline_source_mapping` to jump from a row ID to the frozen baseline source area
- use `crate_surface_summary` or `crate_dependency_graph` before broad code scans
- debtmap is always available in the required MCP surface; use `debtmap_*` once the task is localized to hotspot, review, or refactor work

## MCP Maintenance Mode

Enter MCP maintenance mode whenever a change touches:

- `tools/mcp-cfd-rs*`
- MCP tool contracts
- MCP bundle or snapshot names
- MCP-facing routing docs

During maintenance mode:

- do not rely on MCP answers until the debtmap-enabled operational surface rebuilds and smoke-starts
- keep the `--no-default-features` surface green as a maintenance check, but it does not unblock normal MCP use on its own

## Local Handoff

`GCFGR.md` is optional local overflow state for long or fragile sessions.
It is not canonical repository truth.
Use it only when handoff fidelity matters or context compaction is near.
