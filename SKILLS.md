# Rewrite Workflow Note

This file is a repeatable workflow note for porting subsystems.
It is not a non-negotiables file and it is not a status file.

## Always start here

For any subsystem or slice:

1. identify the exact accepted scope
2. identify the owning crate
3. identify the parity domain (CLI, CDC, or HIS) and the relevant ledger row
4. read the relevant parity feature doc and `docs/parity/source-map.csv` entry
5. read the corresponding Go source and tests
6. check dependency and runtime policy before adding crates or async structure
7. implement the smallest source-grounded slice
8. write contract-level tests
9. update the relevant parity ledger row with evidence status
10. claim parity only after checked behavior

Parity ledgers:

- [docs/parity/cli/implementation-checklist.md](docs/parity/cli/implementation-checklist.md)
- [docs/parity/cdc/implementation-checklist.md](docs/parity/cdc/implementation-checklist.md)
- [docs/parity/his/implementation-checklist.md](docs/parity/his/implementation-checklist.md)

For the full domain and document index, see [docs/parity/README.md](docs/parity/README.md).

## Source order for subsystem work

1. [baseline-2026.2.0/old-impl/](baseline-2026.2.0/old-impl/) code and tests
2. [docs/parity/source-map.csv](docs/parity/source-map.csv) and the relevant parity doc
3. [REWRITE_CHARTER.md](REWRITE_CHARTER.md)
4. [STATUS.md](STATUS.md)
5. matching policy docs under [docs/](docs/)

## Default code preferences

- prefer synchronous and deterministic code unless the accepted slice requires async
- keep normalization logic explicit
- avoid premature daemon/runtime structure in new subsystems
- do not imply transport, proxy, supervisor, orchestration, management, metrics,
  or RPC behavior unless the slice being ported requires it

## Anti-drift reminder

If this file disagrees with:

- Go code/tests
- parity ledgers, feature docs, and `docs/parity/source-map.csv`
- [REWRITE_CHARTER.md](REWRITE_CHARTER.md)
- [STATUS.md](STATUS.md)
- policy docs under [docs/](docs/)

then this file loses.
