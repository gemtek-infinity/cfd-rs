# First Slice And Parity Status

This file captures the accepted first-slice implementation history and the
current parity-backed state for that slice.

## Phase 1A Groundwork

Phase 1A groundwork now exists for the accepted first slice.

What exists now:

- explicit first-slice fixture taxonomy under `crates/cloudflared-config/tests/fixtures/first-slice/`
- executable harness entrypoint at `tools/first_slice_parity.py`
- checked-in JSON golden artifact contract for Go truth and Rust actual reports
- Rust-side helper scaffolding and ignored parity tests in
  `crates/cloudflared-config/tests/`

What does not exist yet:

- complete later-slice behavior outside the accepted first slice

Implication:

- the repo can now inventory and mechanically gate the first-slice parity
  contract
- the repo still must not claim broader rewrite parity is complete

## Phase 1B.1 Domain Skeleton

Phase 1B.1 groundwork now exists in `crates/cloudflared-config/`.

What exists now:

- admitted first-slice crate dependencies only in `crates/cloudflared-config/Cargo.toml`
- explicit module layout for discovery, raw config, normalized config,
  credentials, ingress, and error taxonomy
- honest public APIs for raw YAML loading, credential JSON loading, and raw to
  normalized config conversion boundaries
- narrow unit tests covering shape-level parsing and invariants

What does not exist yet:

- parity-complete config discovery behavior
- origin-cert PEM decoding behavior
- ingress validation and deterministic matching behavior
- Rust actual artifact emission for the Phase 1A harness

Implication:

- `cloudflared-config` is now a real owning crate for the accepted first slice
- the crate still must not be described as a completed first-slice port

## Phase 1B.2 Config Loading Path

Phase 1B.2 behavior now exists for the targeted config-loading fixtures.

What exists now:

- deterministic config discovery with default search order and auto-create side effects
- YAML file loading into the crate's raw config representation
- raw-to-normalized conversion for the currently targeted config surface
- narrow ingress validation for the current invalid and unicode config fixtures
- Rust actual artifact emission for targeted config discovery and config loading fixtures

What does not exist yet:

- Go truth outputs for comparison
- full credentials and origin-cert behavior parity
- full ingress validation and deterministic matching behavior
- CLI-origin ingress normalization artifacts

Implication:

- the accepted first slice now has a real config-loading path in Rust
- the repository still must not claim first-slice parity is complete

## Phase 1B.3 Credentials And Origin-Cert Path

Phase 1B.3 behavior now exists for the targeted credentials/origin-cert
fixtures.

What exists now:

- source-backed PEM scanning for origin certificates
- tolerance for legacy `PRIVATE KEY` and `CERTIFICATE` blocks during scanning
- rejection for unknown PEM block types and multiple token blocks
- origin-cert JSON token extraction with endpoint lowercasing
- account-id refresh validation through the `OriginCertUser` read path
- Rust actual artifact emission for the targeted `credentials-origin-cert`
  fixtures

What does not exist yet:

- Go truth outputs for comparison
- default origin-cert search-path resolution behavior
- tunnel credential file artifact fixtures
- full ingress normalization artifact coverage
- full first-slice parity comparisons

Implication:

- the accepted first slice now has a real credentials/origin-cert path in Rust
- the repository still must not claim first-slice parity is complete

## Phase 1B.4 Ingress Normalization And Matching Path

Phase 1B.4 behavior now exists for the targeted ingress normalization,
ordering/defaulting, and CLI single-origin fixtures.

What exists now:

- semantic ingress service normalization for the current fixture surface
- deterministic user-rule matching with host:port stripping and catch-all fallback
- punycode-aware hostname matching for the current IDN ingress fixture surface
- preserved no-ingress default 503 contract through normalized config output
- narrow CLI single-origin synthesis for `--hello-world`, `--bastion`, `--url`,
  and `--unix-socket`
- Rust actual artifact emission for targeted ingress-related fixture categories

What does not exist yet:

- Go truth outputs for comparison
- internal ingress-rule matching and negative rule-index behavior
- full regex-path semantics for general ingress matching
- full tunnel credential JSON artifact coverage
- full first-slice parity comparisons

Implication:

- the accepted first slice now has a real ingress normalization and matching
  path in Rust for the current fixture surface
- the repository still must not claim first-slice parity is complete

## Phase 1B.5 Go Truth Capture And Real Compare Path

Phase 1B.5 harness behavior now exists for the accepted first-slice fixture
surface.

What exists now:

- checked-in Go truth artifacts under
  `crates/cloudflared-config/tests/fixtures/first-slice/golden/go-truth/` for
  all 21 accepted first-slice fixtures
- a source-backed `capture-go-truth` workflow that stages a small Go helper in
  a temporary module and imports the frozen Go baseline via `replace`
- a real `compare` workflow that emits fresh Rust actual artifacts and compares
  them against the checked-in Go truth artifacts
- explicit mismatch reporting for exact, error-category, structural, semantic,
  and warning-or-report comparison modes
- a passing Go-truth presence gate and a passing real-compare smoke subset in
  the Rust test suite

What does not exist yet:

- closed first-slice Rust-versus-Go parity mismatches

Implication:

- the repository now has a real first-slice parity loop rather than a Rust-only
  artifact scaffold
- the repository still must not claim broader rewrite parity while most later-slice
  subsystems remain unported

## Phase 1B.6 First-Slice Parity Closure

Phase 1B.6 behavior now closes the known accepted first-slice Rust-versus-Go
parity mismatches.

What exists now:

- config-backed normalized ingress payloads now materialize Go-effective
  `originRequest` defaults and carry inherited IP rules into each rule payload
- CLI single-origin ingress normalization now matches Go truth for default-field
  representation, including `false`, `0`, and `1m30s`
- normalized-config artifact emission now matches Go truth for `warnings: null`
  when no warnings are present
- the full accepted first-slice compare is green: 21 compared, 21 matched,
  0 mismatched

What does not exist yet:

- internal ingress-rule matching and negative rule-index behavior
- full regex-path semantics for general ingress matching
- tunnel credential JSON fixture coverage
- any later-slice behavior outside the accepted first slice

Implication:

- the accepted first slice is now parity-backed against the checked-in Go truth
  fixture surface
- the repository still must not claim full-rewrite completion
