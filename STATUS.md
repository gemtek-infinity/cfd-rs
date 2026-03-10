# Cloudflared Rust Rewrite Status

## Classification

This repository is currently a rewrite workspace with a frozen Go reference
implementation and an intentionally narrow Rust first slice.

It is not yet a parity-complete Rust implementation workspace. The repository
now contains a Cargo workspace skeleton plus accepted first-slice behavior in
`crates/cloudflared-config/` for config discovery/loading, credentials
origin-cert decoding, ingress normalization and matching, and a real
first-slice Go-truth compare harness whose accepted fixture surface currently
compares green, but most broader production-alpha subsystem behavior is still
unported.

The scaffold is intentionally real but minimal:

- the workspace builds as a Rust scaffold with partial first-slice behavior
- the runnable binary exists
- policy and governance documents define the rewrite boundary
- manifests should reflect only code that exists today, not speculative future
  subsystem work

## Decided Now

### Compatibility Baseline

- The primary rewrite target is the frozen Go snapshot in `baseline-2026.2.0/old-impl/`.
- The target release is `2026.2.0`, as recorded at the top of
  `baseline-2026.2.0/old-impl/RELEASE_NOTES`.
- The documentation set in `baseline-2026.2.0/design-audit/` is the derived reference
  and navigation layer for that snapshot.
- The Rust workspace version must track that Go release baseline and use the
  format `<go-release>-alpha.YYYYmm`. The current workspace version is
  `2026.2.0-alpha.202603`.

### Source Precedence

If sources disagree, use this precedence order:

1. `baseline-2026.2.0/old-impl/` code and tests
2. `baseline-2026.2.0/design-audit/`
3. `AGENTS.md`
4. `SKILLS.md`

### Governance Documents

The following top-level rewrite decisions are part of the active scaffold:

- `docs/compatibility-scope.md`
- `docs/build-artifact-policy.md`
- `docs/go-rust-semantic-mapping.md`
- `docs/dependency-policy.md`
- `docs/allocator-runtime-baseline.md`
- `docs/adr/0001-hybrid-concurrency-model.md`
- `docs/adr/0002-transport-tls-crypto-lane.md`
- `docs/adr/0003-pingora-critical-path.md`

### Active Phase Model

- Big Phase 1 is done:
  - truth freeze is in place
  - the accepted first-slice compare is green
  - broader subsystem work remains mostly unported
- Big Phase 2 is current:
  - purpose: freeze the Linux production-alpha lane
  - active task: 2.3 Pingora critical-path ADR
- Big Phase 3 is later:
  - build the minimum runnable alpha on the frozen lane
- Big Phase 4 is later:
  - harden, validate, measure, and prove the alpha in real use
- Big Phase 5 is later:
  - widen intentionally only after the alpha is credible

### Active Lane

- Linux only
- target triple: `x86_64-unknown-linux-gnu`
- shipped GNU artifacts only:
  - `x86-64-v2`
  - `x86-64-v4`
- 0-RTT is required
- quiche first
- quiche + BoringSSL
- Pingora is in the production-alpha critical path
- FIPS belongs in the production-alpha lane
- Cloudflare-owned crates are preferred where they genuinely fit, but are not
  mandatory by default

### Current Workspace Shape

- `baseline-2026.2.0/old-impl/` contains the frozen Go source of truth
- `baseline-2026.2.0/design-audit/` contains the extracted spec set
- `docs/` contains rewrite-program decisions and semantic guidance
- `crates/` contains only minimal Rust crate skeletons

Current crate intent:

- `crates/cloudflared-cli`: runnable binary scaffold only
- `crates/cloudflared-config`: owning crate for the accepted first-slice
  domain skeleton and future config, credentials, and ingress normalization
  behavior
- `crates/cloudflared-config/tests/`: first-slice parity harness and fixtures
- `crates/cloudflared-core`: future shared types and cross-cutting primitives
- `crates/cloudflared-proto`: future wire-format and RPC boundary

### Frozen Inputs

The following directories are immutable inputs to the rewrite program and must
not be edited during normal rewrite work:

- `baseline-2026.2.0/old-impl/`
- `baseline-2026.2.0/design-audit/`

If those inputs appear inconsistent, update top-level governance documents or
the Rust workspace instead of modifying the frozen reference material.

### Dependency And Runtime Baseline

- The workspace already has a scaffold and should remain minimal and honest.
- The runnable binary must use `mimalloc` as the process allocator.
- The required `mimalloc` feature set is:
  - `override`
  - `no_thp`
  - `local_dynamic_tls`
  - `extended`
- Allocator choice belongs only at the runnable binary boundary.
- Async runtime choice is governed by
  `docs/allocator-runtime-baseline.md` and
  `docs/adr/0001-hybrid-concurrency-model.md`.

## Active Phase 2.3 Focus

Phase 2.3 now owns Pingora critical-path scope freeze for the frozen Linux
production-alpha lane.

What it covers now:

- Pingora's relationship to the quiche transport lane is explicitly frozen
- Pingora's initial responsibilities in the production-alpha path are explicit
- Pingora's explicit non-responsibilities are stated
- the first admitted Pingora crates are explicit

What it still must not imply:

- that 2.4 through 2.5 are already done
- that broader runtime, transport, Pingora, FIPS operational, or deployment
  implementation already exists

## Deferred Within Big Phase 2

The following lane-freeze work is intentionally deferred beyond 2.3:

- 2.4 FIPS-in-alpha definition:
  - runtime crypto boundary
  - build/link boundary
  - validation posture
- 2.5 deployment contract:
  - glibc assumptions
  - systemd/service expectations
  - container vs bare-metal assumptions
  - filesystem/layout expectations

## Deferred Beyond Big Phase 2

The following remain intentionally out of the current lane-freeze task:

- broader platform parity beyond Linux
- broader artifact scope beyond GNU `x86-64-v2` and `x86-64-v4`
- broad runtime implementation outside the accepted first slice
- transport, Pingora, FIPS operational, and deployment implementation work

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

## First Implementation Gate

No large-scale subsystem implementation should begin until all of the following
are true:

1. `docs/compatibility-scope.md` is accepted
2. `docs/go-rust-semantic-mapping.md` is accepted
3. `docs/dependency-policy.md` is accepted
4. `docs/allocator-runtime-baseline.md` is accepted
5. `docs/adr/0001-hybrid-concurrency-model.md` is accepted
6. the workspace skeleton is accepted
7. the first subsystem slice and its parity checks are frozen

## Recommended First Slice

Port config, credentials, and ingress normalization first.

Reason:

- the behavior is heavily documented
- the inputs and outputs are comparatively deterministic
- the slice is lower risk than transports, supervisors, or streaming bridges
- it freezes CLI/config/env precedence and credential parsing early
- it freezes ingress validation and default no-ingress behavior early
- it unlocks later runtime assembly work without starting in the highest-risk
  concurrency areas

Scope boundary for the first slice:

- include config discovery, parsing, validation, and credential handling
- include ingress parsing, validation, normalization, and deterministic rule
  matching
- include CLI-origin synthesis only to the extent needed to normalize
  single-origin ingress inputs
- exclude proxying, transports, supervisor logic, metrics servers, management,
  and orchestration

Current scaffold implication:

- narrow first-slice implementation code is now present
- the crate layout still reserves the correct boundaries for the remainder of
  this first slice
- manifests should stay sparse while the remaining slice behavior lands

## Done Means

A subsystem should not be called "ported" unless:

- behavior matches `baseline-2026.2.0/old-impl/`
- relevant config or CLI surface matches `baseline-2026.2.0/old-impl/`
- relevant wire bytes match `baseline-2026.2.0/old-impl/` where applicable
- documented quirks are either preserved or explicitly waived
- parity tests are documented and passing
