# Compatibility Scope

This document defines what "compatible" means for the Rust rewrite.

It exists to keep behavioral compatibility, current implemented scope, and later deployment concerns from being blurred together.

## Primary compatibility baseline

The rewrite target is the behavior of the frozen Go snapshot in:

- `baseline-2026.2.0/old-impl/`

The target release baseline is:

- `2026.2.0`

The derived navigation/spec layer is:

- `baseline-2026.2.0/design-audit/`

## Compatibility routing

Use the right source for the right question:

- behavior and parity:
  1. `baseline-2026.2.0/old-impl/` code and tests
  2. `baseline-2026.2.0/design-audit/`

- current repository state:
  - `STATUS.md`

- non-negotiables and scope:
  - `REWRITE_CHARTER.md`

- dependency and runtime policy:
  - `docs/*.md`

- workflow notes:
  - `AGENTS.md`
  - `SKILLS.md`

Do not claim compatibility from Rust code shape alone.

## Frozen inputs

`baseline-2026.2.0/old-impl/` and `baseline-2026.2.0/design-audit/` are frozen inputs.

Do not modify either directory during normal rewrite work.

## Active lane

The active lane is:

- Linux only
- target triple: `x86_64-unknown-linux-gnu`
- shipped GNU artifacts:
  - `x86-64-v2`
  - `x86-64-v4`

This document does not imply broader platform parity.

## First accepted implementation slice

The first accepted implementation slice remains:

- config discovery/loading/normalization
- credentials surface
- ingress normalization/ordering/defaulting

Anything beyond that must be justified explicitly by accepted scope and checked behavior.
