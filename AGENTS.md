# AGENTS.md

Start cold reads with `docs/ai-context-routing.md`.
Use this file as a short operating guide, not as a document index dump.

## Read first

- `STATUS.md` — the only tracked status file
- `docs/phase-5/roadmap.md` — normative Phase 5 roadmap
- `docs/parity/README.md` — parity index
- `REWRITE_CHARTER.md` — non-negotiables and scope

## Working rules

- do not treat this repository as a blank-slate Rust project
- do not edit frozen inputs under `baseline-2026.2.0/`
- do not claim parity from Rust code shape alone
- keep patches narrow and source-grounded
- update the relevant parity ledger for parity work
- `GCFGR.md` is optional local handoff state only; `STATUS.md` wins
- when formatting Rust, use `cargo +nightly fmt`, never plain `cargo fmt`
- if you touch `tools/mcp-cfd-rs*` or MCP-facing routing docs, rebuild and smoke the debtmap-enabled MCP target before relying on MCP again

## MCP-first routing

When MCP is available, prefer the startup/routing MCP tools before opening larger docs.
The required operational MCP surface includes debtmap:

- repo status or startup truth: `status_summary`
- current lane-blocking queue: `phase5_priority`
- one exact parity row: `parity_row_details`
- ranked work inside one domain: `domain_gaps_ranked`
- jump from a row to frozen Go sources: `baseline_source_mapping`
- crate ownership or dependency direction: `crate_surface_summary`, `crate_dependency_graph`
- compact file routing: `get_context_snapshot`, `get_context_bundle`, `get_context_brief`
- hotspot/refactor/review work: `debtmap_*`

Fall back to docs only when MCP is unavailable or the first MCP answer is insufficient.

## Direct doc routing

- implementation order or milestone question: `docs/phase-5/roadmap.md`
- exact row ownership or milestone mapping: `docs/phase-5/roadmap-index.csv`
- behavior truth: frozen Go baseline first
- parity work: identify CLI, CDC, or HIS and open that ledger first
