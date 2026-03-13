# Contributing

Read these first:

1. `README.md`
2. `STATUS.md`
3. `docs/phase-5/roadmap.md`
4. `REWRITE_CHARTER.md`
5. `docs/promotion-gates.md`

## How to find work

Parity work is tracked in three live ledgers:

- `docs/parity/cli/implementation-checklist.md`
- `docs/parity/cdc/implementation-checklist.md`
- `docs/parity/his/implementation-checklist.md`

Use `STATUS.md` for the current priority queue.
Use `docs/phase-5/roadmap-index.csv` when you need the exact milestone for a row.

## Build and test

```bash
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo +nightly fmt --all
```

All four commands must pass before submission.

## Parity evidence

Do not claim parity from Rust code shape alone.
Evidence must come from the frozen Go baseline in `baseline-2026.2.0/old-impl/`.
Typical evidence includes:

- blackbox output comparison
- wire-format round-trip tests
- contract-level tests
- host-behavior tests

Update the touched ledger row when the evidence changes.

## Document order

When documents conflict, resolve in this order:

1. frozen Go baseline code and tests
2. frozen design-audit documents
3. `REWRITE_CHARTER.md` and `docs/compatibility-scope.md`
4. `docs/promotion-gates.md`
5. `STATUS.md`
6. `docs/phase-5/roadmap.md` and `docs/phase-5/roadmap-index.csv`
7. `AGENTS.md` and `SKILLS.md`

## AI-assisted work

AI contributors should start with `docs/ai-context-routing.md`.
When MCP is available, prefer `status_summary`, `phase5_priority`, `parity_row_details`, `domain_gaps_ranked`, `baseline_source_mapping`, `crate_surface_summary`, and `crate_dependency_graph` before loading larger docs.
The operational MCP target is debtmap-enabled; if MCP files change, rebuild and smoke that target before trusting MCP again.
