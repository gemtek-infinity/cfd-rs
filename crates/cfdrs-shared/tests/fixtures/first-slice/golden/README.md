# Golden Output Rules

This directory holds the checked-in artifact contract for the accepted
first-slice parity harness.

Layout:

- `schema/` documents the JSON envelope and report kinds
- `go-truth/` will hold captured Go outputs, one file per fixture
- `rust-actual/` will hold future Rust-emitted reports during comparison runs

Rules:

- prefer explicit checked-in goldens over approval-style snapshots
- keep one JSON artifact per fixture ID per producer side
- use the same canonical envelope for both Go truth and Rust actual reports
- compare exact canonical JSON when a harness report schema exists
- use structural or error-category comparison only where this is documented in
  `crates/cfdrs-shared/tests/fixtures/first-slice/fixture-index.toml`
  and the applicable owning harness code
- Phase 1A intentionally leaves `go-truth/` and `rust-actual/` empty; the
  harness must fail clearly rather than pretend those outputs exist
