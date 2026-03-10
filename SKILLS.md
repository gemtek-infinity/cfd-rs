# Rewrite Workflow Note

This file is a repeatable workflow note for porting subsystems.
It is not a non-negotiables file and it is not a status file.

## Always start here

For any subsystem or slice:

1. identify the exact accepted scope
2. identify the owning crate
3. read the relevant design-audit sections
4. read the corresponding Go source and tests
5. check dependency and runtime policy before adding crates or async structure
6. implement the smallest source-grounded slice
7. write contract-level tests
8. claim parity only after checked behavior

## Source order for subsystem work

1. `baseline-2026.2.0/old-impl/` code and tests
2. `baseline-2026.2.0/design-audit/`
3. `REWRITE_CHARTER.md`
4. `STATUS.md`
5. matching policy docs under `docs/`

## First-slice bias

For the accepted first slice:

- prefer synchronous and deterministic code
- keep normalization logic explicit
- avoid premature daemon/runtime structure
- do not imply transport, proxy, supervisor, orchestration, management, metrics, or RPC behavior

## Anti-drift reminder

If this file disagrees with:

- Go code/tests
- design-audit docs
- `REWRITE_CHARTER.md`
- `STATUS.md`
- policy docs under `docs/`

then this file loses.
