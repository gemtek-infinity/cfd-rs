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

### Runtime Or Dependency Policy

Load first:

- [`docs/dependency-policy.md`](dependency-policy.md)
- [`docs/allocator-runtime-baseline.md`](allocator-runtime-baseline.md)

### Behavior Truth

Load first:

- [`baseline-2026.2.0/old-impl/`](../baseline-2026.2.0/old-impl/)
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

- use `status_summary` as the default startup entry for repo truth
- use `phase5_priority` for the current lane-blocking queue
- use `parity_row_details` when you already know the ledger row ID
- use `domain_gaps_ranked` when you need bounded ranked work inside one domain
- use `baseline_source_mapping` to jump from a row ID to the frozen baseline source area and exact parity feature doc
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
- use `just mcp-smoke` and `just mcp-smoke-maintenance` as the normal smoke entrypoints

## Local Handoff

`GCFGR.md` is optional local overflow state for long or fragile sessions.
It is not canonical repository truth.
Use it only when handoff fidelity matters or context compaction is near.
