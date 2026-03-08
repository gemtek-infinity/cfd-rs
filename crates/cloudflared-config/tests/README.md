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
- Go truth capture is not checked in yet
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
- `python3 tools/first_slice_parity.py check-go-truth` fails until Go truth JSON
 is captured under `tests/fixtures/first-slice/golden/go-truth/`
- `python3 tools/first_slice_parity.py emit-rust-actual` writes readable JSON
 artifacts for the currently implemented config, credentials, and ingress fixtures
- `cargo test -p cloudflared-config` validates the fixture inventory and keeps
 ignored parity tests visible without claiming passing parity

Source-of-truth rule:

- use `baseline-2026.2.0/old-impl/` code and tests first
- use `baseline-2026.2.0/design-audit/` second

Do not modify frozen inputs from this test area.
