# Compatibility Scope

This document defines what "compatible" means for the Rust rewrite.

It exists to keep behavioral compatibility, current implemented scope, and
production-alpha gate expectations from being blurred together.

## Primary Compatibility Baseline

The rewrite target is the behavior of the frozen Go snapshot in:

- [`baseline-2026.2.0/old-impl/`](../baseline-2026.2.0/old-impl/)

The target release baseline is:

- `2026.2.0`

The derived parity routing layer is:

- [`docs/parity/`](parity/)
- [`docs/parity/source-map.csv`](parity/source-map.csv)

## Compatibility Routing

Use the right source for the right question:

- behavior truth: [`baseline-2026.2.0/old-impl/`](../baseline-2026.2.0/old-impl/)
- derived parity routing: [`docs/parity/README.md`](parity/README.md), [`docs/parity/source-map.csv`](parity/source-map.csv), and the relevant parity doc
- current repository state: [`STATUS.md`](../STATUS.md)
- scope and lane boundary: [`REWRITE_CHARTER.md`](../REWRITE_CHARTER.md)
- implementation order: [`docs/phase-5/roadmap.md`](phase-5/roadmap.md)
- promotion boundary: [`docs/promotion-gates.md`](promotion-gates.md)

Do not claim compatibility from Rust code shape alone.

## Frozen Inputs

[`baseline-2026.2.0/old-impl/`](../baseline-2026.2.0/old-impl/) is a frozen input.

Do not modify it during normal rewrite work.

## Active Lane

The active lane is:

- Linux only
- target triple: `x86_64-unknown-linux-gnu`
- shipped GNU artifacts: `x86-64-v2`, `x86-64-v4`

This document does not imply broader platform parity.

## Production-Alpha Boundaries

Production-alpha requires all lane-required CLI, CDC, and HIS rows to be backed
by current evidence, plus a clean exit from the final `Performance Architecture
Overhaul` milestone.

Logging compatibility is part of that gate, not a follow-up item.
