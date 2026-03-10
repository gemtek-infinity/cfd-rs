# cloudflared-config Test Harness

This directory owns the parity harness and fixtures for the accepted first
slice only:

- config discovery/loading/normalization
- credentials surface
- ingress normalization, ordering, and defaulting

It must not grow into a general runtime or transport test area before those
subsystems start.

Current state:

- Phase 1A fixture taxonomy and harness scaffolding exist
- Phase 1B.2 can emit Rust actual artifacts for config discovery/loading fixtures
- Phase 1B.3 can emit Rust actual artifacts for credentials/origin-cert fixtures
- Phase 1B.4 can emit Rust actual artifacts for ingress normalization and ordering/defaulting fixtures
- Phase 1B.5 has checked-in Go truth artifacts for the accepted first-slice fixture surface
- Phase 1B.5 compare mode now runs a real Rust-versus-Go comparison for that surface
- config, credentials, and ingress behavior are only partially implemented
- Rust-versus-Go parity is not complete yet

Phase 1A outputs in this directory:

- explicit fixture taxonomy under `tests/fixtures/first-slice/`
- a checked-in golden artifact contract for future Go truth and Rust actuals
- Rust-side helper scaffolding and ignored parity test entrypoints
- an external runner entrypoint at `tools/first_slice_parity.py`
- a Rust actual emission path for the currently implemented first-slice fixture categories

Current execution model:

- `python3 tools/first_slice_parity.py inventory` shows the accepted fixture set
- `python3 tools/first_slice_parity.py capture-go-truth` refreshes the checked-in
 Go truth JSON artifacts under `tests/fixtures/first-slice/golden/go-truth/`
- `python3 tools/first_slice_parity.py check-go-truth` now verifies that every
 accepted fixture has a checked-in Go truth artifact
- `python3 tools/first_slice_parity.py emit-rust-actual` writes readable JSON
 artifacts for the currently implemented config, credentials, and ingress fixtures
- `python3 tools/first_slice_parity.py compare --require-go-truth --require-rust-actual`
 now runs a real compare and exits nonzero when artifacts differ
- `cargo test -p cloudflared-config` validates the fixture inventory and keeps
 the harness gate and a matching compare subset executable

Source-of-truth rule:

- use `baseline-2026.2.0/old-impl/` code and tests first
- use `baseline-2026.2.0/design-audit/` second

Do not modify frozen inputs from this test area.
