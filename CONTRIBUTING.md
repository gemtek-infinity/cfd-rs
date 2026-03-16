# Contributing

Human contribution workflow for this repository.

## Start

- [`README.md`](README.md) — landing page
- [`docs/README.md`](docs/README.md) — human documentation map
- [`STATUS.md`](STATUS.md) — current blockers, milestone, parity snapshot

Open deeper leaf docs only when the task needs them.

## Find Work

- [`STATUS.md`](STATUS.md) — current blockers and priority
- [`docs/phase-5/roadmap.md`](docs/phase-5/roadmap.md) — implementation order
- [`docs/phase-5/roadmap-index.csv`](docs/phase-5/roadmap-index.csv) —
  exact row ownership
- [`docs/parity/README.md`](docs/parity/README.md) — parity navigation
- [`docs/parity/source-map.csv`](docs/parity/source-map.csv) — baseline routing

## Work Loop

1. Make the smallest source-grounded change.
2. Keep crate ownership boundaries intact.
3. Update the owning parity ledger when behavior or evidence changes.
4. Regenerate or validate [`docs/parity/source-map.csv`](docs/parity/source-map.csv)
   through repo tooling if routing changed.
5. Update [`STATUS.md`](STATUS.md) if current reality changed.
6. Run the full validation gate.

## Validate

Default entry:

```bash
just validate-pr
```

Focused entrypoints:

- `just fmt` — formatting only
- `just validate-governance` — docs and source-map validation
- `just validate-app` — app crates
- `just validate-tools` — MCP and tool crates
- `just mcp-smoke` — operational MCP smoke when touching MCP tools or routing
  docs

The operational MCP target is debtmap-enabled.

`just fmt` runs `cargo +nightly fmt --all`.

## Parity Evidence

Do not claim parity from Rust code shape alone.

Use evidence from the frozen Go baseline in
[`baseline-2026.2.0/`](baseline-2026.2.0/):

- blackbox output comparison
- wire-format round-trip tests
- contract-level tests
- host-behavior tests

## Before Review

- Keep the diff narrow and source-grounded.
- Include parity ledger, source-map, and [`STATUS.md`](STATUS.md) updates when
  the change affects them.
- Do not hand-edit generated artifacts such as
  [`docs/parity/source-map.csv`](docs/parity/source-map.csv); regenerate or
  validate them through repo tooling.

## Conflict Order

1. frozen Go baseline code and tests
2. [`REWRITE_CHARTER.md`](REWRITE_CHARTER.md) and
   [`docs/compatibility-scope.md`](docs/compatibility-scope.md)
3. [`docs/promotion-gates.md`](docs/promotion-gates.md)
4. [`STATUS.md`](STATUS.md)
5. [`docs/phase-5/roadmap.md`](docs/phase-5/roadmap.md) and
   [`docs/phase-5/roadmap-index.csv`](docs/phase-5/roadmap-index.csv)
6. [`docs/parity/README.md`](docs/parity/README.md),
   [`docs/parity/source-map.csv`](docs/parity/source-map.csv), and the
   relevant parity doc
7. workflow notes such as [`AGENTS.md`](AGENTS.md) and [`SKILLS.md`](SKILLS.md)
