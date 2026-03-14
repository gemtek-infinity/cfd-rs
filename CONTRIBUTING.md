# Contributing

Read these first:

1. [`README.md`](README.md)
2. [`STATUS.md`](STATUS.md)
3. [`docs/phase-5/roadmap.md`](docs/phase-5/roadmap.md)
4. [`REWRITE_CHARTER.md`](REWRITE_CHARTER.md)
5. [`docs/promotion-gates.md`](docs/promotion-gates.md)
6. [`docs/ai-context-routing.md`](docs/ai-context-routing.md)

## How to find work

Parity work is tracked in three live ledgers:

- [`docs/parity/cli/implementation-checklist.md`](docs/parity/cli/implementation-checklist.md)
- [`docs/parity/cdc/implementation-checklist.md`](docs/parity/cdc/implementation-checklist.md)
- [`docs/parity/his/implementation-checklist.md`](docs/parity/his/implementation-checklist.md)

Use [`STATUS.md`](STATUS.md) for the current priority queue.
Use [`docs/phase-5/roadmap-index.csv`](docs/phase-5/roadmap-index.csv) when you need the exact milestone for a row.
Use [`docs/parity/source-map.csv`](docs/parity/source-map.csv) when you need the exact frozen-baseline jump path.

## Build and test

Normal entry:

```bash
just validate-pr
```

Useful focused entrypoints:

```bash
just fmt
just fmt-check
just validate-governance
just validate-app
just validate-tools
just mcp-smoke
```

`fmt` always means `cargo +nightly fmt --all`.

## Parity evidence

Do not claim parity from Rust code shape alone.
Evidence must come from the frozen Go baseline in [`baseline-2026.2.0/old-impl/`](baseline-2026.2.0/old-impl/).
Typical evidence includes:

- blackbox output comparison
- wire-format round-trip tests
- contract-level tests
- host-behavior tests

Update the touched ledger row when the evidence changes.
Update [`docs/parity/source-map.csv`](docs/parity/source-map.csv) when the baseline routing for a row changes.

## Document order

When documents conflict, resolve in this order:

1. frozen Go baseline code and tests
2. [`REWRITE_CHARTER.md`](REWRITE_CHARTER.md) and [`docs/compatibility-scope.md`](docs/compatibility-scope.md)
3. [`docs/promotion-gates.md`](docs/promotion-gates.md)
4. [`STATUS.md`](STATUS.md)
5. [`docs/phase-5/roadmap.md`](docs/phase-5/roadmap.md) and [`docs/phase-5/roadmap-index.csv`](docs/phase-5/roadmap-index.csv)
6. [`docs/parity/README.md`](docs/parity/README.md), [`docs/parity/source-map.csv`](docs/parity/source-map.csv), and the relevant parity doc
7. [`AGENTS.md`](AGENTS.md) and [`SKILLS.md`](SKILLS.md)

## AI-assisted work

AI contributors should start with [`docs/ai-context-routing.md`](docs/ai-context-routing.md).
When MCP is available, prefer `status_summary`, `phase5_priority`, `parity_row_details`, `domain_gaps_ranked`, `baseline_source_mapping`, `crate_surface_summary`, and `crate_dependency_graph` before loading larger docs.
The operational MCP target is debtmap-enabled; if MCP files change, rebuild and smoke that target before trusting MCP again.
Use [`Justfile`](Justfile) as the normal command surface rather than open-coded cargo chains.
