# cfdrs-shared Evidence Tests

This directory holds evergreen parity evidence for the shared config,
credentials, and ingress surfaces.

Covered surfaces:

- config discovery and loading
- credentials and origin-cert decoding
- ingress normalization, ordering, and flag-derived defaults

The assets in `tests/fixtures/shared-behavior/` are baseline-backed evidence.
They are not a general runtime or transport test area.

## Execution model

- `python3 tools/shared_behavior_parity.py inventory` lists the fixture set
- `python3 tools/shared_behavior_parity.py capture-go-truth` refreshes checked-in Go truth artifacts
- `python3 tools/shared_behavior_parity.py check-go-truth` verifies that every selected fixture has Go truth
- `python3 tools/shared_behavior_parity.py emit-rust-actual` emits readable Rust-side artifacts
- `python3 tools/shared_behavior_parity.py compare --require-go-truth --require-rust-actual` performs the Rust-vs-Go compare loop
- `cargo test -p cfdrs-shared` validates the fixture inventory, Go-truth gate, targeted emission paths, and compare closure for these shared surfaces

## Source of truth

- `baseline-2026.2.0/old-impl/` code and tests first
- `docs/parity/source-map.csv` for bounded row-to-source routing

Do not modify frozen inputs from this test area.
