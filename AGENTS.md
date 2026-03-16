# AGENTS.md

Full AI routing contract:
[`docs/ai-context-routing.md`](docs/ai-context-routing.md).

This file keeps only local agent guardrails.

## Start

- Read `GCFGR.md` first if it exists.
- Then read [`docs/ai-context-routing.md`](docs/ai-context-routing.md).
- [`STATUS.md`](STATUS.md) is the only tracked status file.
- Use MCP-first startup tools:
  `status_summary`, `phase5_priority`, `parity_row_details`,
  `domain_gaps_ranked`, `baseline_source_mapping`,
  `crate_surface_summary`, `crate_dependency_graph`.
- Use compact routing tools:
  `get_context_snapshot`, `get_context_bundle`, `get_context_brief`.
- The operational debtmap-enabled MCP target is the normal startup surface.

## Local Guardrails

- Use [`Justfile`](Justfile) as the normal command surface.
- Formatting-only work still means `cargo +nightly fmt`, normally through
  `just fmt`.
- Do not edit [`baseline-2026.2.0/`](baseline-2026.2.0/) during normal
  rewrite work.
- Do not claim parity from Rust code shape alone.
- Reconcile [`STATUS.md`](STATUS.md) and
  [`docs/parity/source-map.csv`](docs/parity/source-map.csv) after parity or
  status-affecting work.
- If you touch `tools/mcp-cfd-rs*` or MCP-facing routing docs, rebuild and
  smoke the operational MCP surface before trusting MCP again.
