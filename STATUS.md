# Cloudflared Rust Rewrite Status

## Classification

This repository is currently a rewrite planning and scaffolding repository with
a frozen Go reference implementation.

It is not yet a Rust implementation workspace in the substantive sense. The
repository now contains a Cargo workspace skeleton plus a first-slice domain
skeleton in `crates/cloudflared-config/`, but no subsystem behavior has been
ported yet.

The scaffold is intentionally real but minimal:

- the workspace builds as a Rust scaffold
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
- `docs/go-rust-semantic-mapping.md`
- `docs/dependency-policy.md`
- `docs/allocator-runtime-baseline.md`
- `docs/adr/0001-hybrid-concurrency-model.md`

### Phase 1 Rewrite Scope

- Primary platform: `x86_64-unknown-linux-gnu`
- In scope:
  - CLI behavior
  - config and credentials behavior
  - runtime behavior
  - wire and protocol behavior
  - metrics, readiness, and management behavior
  - Linux-relevant service/runtime semantics that affect externally visible behavior

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

## Deferred Later

The following are intentionally deferred until explicitly promoted:

- macOS parity
- Windows parity
- packaging and installer parity
- release automation parity
- updater workflow parity
- FIPS artifact and compliance parity

## Missing Before Large-Scale Porting

These items are still missing before MCP-assisted or large-scale subsystem work
should begin:

- accepted compatibility-scope decision for FIPS/compliance
- captured Go truth outputs and passing first-slice parity tests

## Phase 1A Groundwork

Phase 1A groundwork now exists for the accepted first slice.

What exists now:

- explicit first-slice fixture taxonomy under `crates/cloudflared-config/tests/fixtures/first-slice/`
- executable harness entrypoint at `tools/first_slice_parity.py`
- checked-in JSON golden artifact contract for Go truth and Rust actual reports
- Rust-side helper scaffolding and ignored parity tests in
  `crates/cloudflared-config/tests/`

What does not exist yet:

- captured Go truth outputs
- Rust-emitted parity reports
- implemented config, credential, or ingress behavior
- passing first-slice parity comparisons

Implication:

- the repo can now inventory and mechanically gate the first-slice parity
  contract
- the repo still must not claim first-slice parity is complete

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

- no subsystem implementation code is present yet
- the crate layout already reserves the correct boundaries for this first slice
- manifests should stay sparse until this slice starts landing

## Done Means

A subsystem should not be called "ported" unless:

- behavior matches `baseline-2026.2.0/old-impl/`
- relevant config or CLI surface matches `baseline-2026.2.0/old-impl/`
- relevant wire bytes match `baseline-2026.2.0/old-impl/` where applicable
- documented quirks are either preserved or explicitly waived
- parity tests are documented and passing
