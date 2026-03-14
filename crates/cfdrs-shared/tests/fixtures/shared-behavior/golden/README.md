# Golden Output Rules

This directory holds the checked-in artifact contract for shared-behavior parity evidence.

Layout:

- `schema/` — canonical JSON envelope and report kinds
- `go-truth/` — frozen Go outputs, one file per fixture
- `rust-actual/` — Rust-emitted outputs for comparison runs

Rules:

- keep one JSON artifact per fixture ID per producer
- use the same canonical envelope for both Go truth and Rust actual reports
- compare exact canonical JSON when a report schema exists
- use `fixture-index.toml` to document any structural-comparison exceptions
