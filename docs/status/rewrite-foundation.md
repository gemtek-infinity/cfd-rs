# Rewrite Foundation Status

This file captures the current repository-wide foundation state.

Use it when the question is about what the workspace is, which lane is active,
which governance documents are current, or which crates currently own real
behavior.

## Classification

This repository is currently a rewrite workspace with a frozen Go reference
implementation and an intentionally narrow Rust first slice.

It is not yet a parity-complete Rust implementation workspace. The repository
now contains a Cargo workspace skeleton plus accepted first-slice behavior in
`crates/cloudflared-config/` for config discovery/loading, credentials
origin-cert decoding, ingress normalization and matching, and a real
first-slice Go-truth compare harness whose accepted fixture surface currently
compares green. It also now contains a narrow Phase 3.3 QUIC tunnel core in
`crates/cloudflared-cli/` that owns startup, supervision, transport session
establishment, and runtime config handoff. Phase 3.4 adds a Pingora proxy seam
with runtime lifecycle participation and a first admitted origin/proxy path
(`http_status` routing) confined to `crates/cloudflared-cli/src/proxy.rs`. The
admitted origin path is intentionally narrow. Broader wire/protocol behavior
and general proxy completeness remain later slices. Most broader
production-alpha subsystem behavior is still unported.

The scaffold is intentionally real but minimal:

- the workspace builds as a Rust scaffold with partial first-slice behavior
- the runnable binary now exposes the admitted Phase 3.3 QUIC tunnel-core
  surface and the Phase 3.4 Pingora proxy seam with its first origin path
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
- `docs/adr/0004-fips-in-alpha-definition.md`
- `docs/adr/0005-deployment-contract.md`
- `docs/adr/ADR-0006-standard-format-and-workspace-dependency-admission.md`

### Active Phase Model

- Big Phase 1 is done:
  - truth freeze is in place
  - the accepted first-slice compare is green
  - broader subsystem work remains mostly unported
- Big Phase 2 is closed and frozen:
  - purpose was to freeze the Linux production-alpha lane
  - tasks 2.0 through 2.6 are complete at the governance level
- Big Phase 3 is current:
  - purpose: build the minimum runnable alpha on the frozen lane
  - Phase 3.3 QUIC tunnel core is admitted
  - Phase 3.4 Pingora proxy seam (3.4a–c) is admitted
  - active task: 3.4d docs/tests/status reconciliation
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

- `crates/cloudflared-cli`: narrow admitted alpha entry surface for help,
  version, config-backed startup validation, the current runtime/lifecycle
  owner, the current QUIC transport core, and the Pingora proxy seam
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
  - Tokio is now admitted at the binary boundary for the active runtime/
    lifecycle shell that underpins the current Phase 3.3 tunnel core.
- Async runtime choice is governed by
  `docs/allocator-runtime-baseline.md` and
  `docs/adr/0001-hybrid-concurrency-model.md`.
