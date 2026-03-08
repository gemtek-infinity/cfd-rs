# Dependency Policy

This document defines how dependencies enter the Rust rewrite workspace.

The repository already has a Rust scaffold. Dependency policy therefore exists
to keep that scaffold minimal, honest, and aligned with the accepted rewrite
slice, not to predeclare the full future dependency graph.

## Non-Negotiable Constraints

- `baseline-2026.2.0/old-impl/` is frozen input
- `baseline-2026.2.0/design-audit/` is frozen input
- the Rust workspace version remains `2026.2.0-alpha.202603` until changed by
  explicit baseline/versioning policy
- manifests should describe code that exists today or the currently accepted
  next slice, not speculative later slices

## Admission Principles

Dependencies are admitted only when all of the following are true:

1. the owning subsystem slice is accepted
2. the dependency is justified by source-backed behavior or test needs
3. the crate boundary that owns the dependency is clear
4. the dependency does not quietly redesign externally visible behavior
5. a standard-library alternative is not sufficient

## Current Workspace Rule

The current workspace is a scaffold, not a partial runtime.

That means:

- no future transport, RPC, or async-runtime dependencies should be declared in
  manifests before code using them exists
- placeholder crates may remain dependency-free when they contain only module
  docs
- policy documents may describe later-approved libraries without preloading them
  into `Cargo.toml`

## Current Admitted Dependencies

The current scaffold admits only one external runtime dependency in manifests:

- `mimalloc` in the runnable binary crate

Reason:

- allocator policy is a process-wide runtime baseline
- the binary exists today and can own allocator choice honestly
- libraries must not set the global allocator

## Deferred Dependency Buckets

These libraries are approved only when their owning slice begins.

### Config, Credentials, And Ingress Normalization Slice

Admit only when implementation starts in `cloudflared-config`:

- `serde`
- `serde_json`
- `serde_yaml`
- `url`
- `uuid`
- `thiserror`

Notes:

- `serde_yaml` is tolerated for parity work even though the upstream crate line
  carries a deprecation marker; it must remain a deliberate compatibility
  choice, not a default convenience dependency
- if a more precise YAML strategy is later required, that change needs explicit
  compatibility review rather than silent substitution

### Async Control-Plane And Data-Plane Slices

Admit only when async implementation starts in the owning crates:

- `tokio`
- `tokio-util`

Rules:

- their admission must follow `docs/go-rust-semantic-mapping.md`
- do not add alternative channel/runtime frameworks by default

### Protocol And Wire Slices

Admit only when protocol implementation starts:

- `bytes`
- `capnp`

Rules:

- admission must be tied to exact wire and schema preservation work
- do not add protocol libraries speculatively because the crate name exists

### Logging And Observability Slices

Admit only when runtime logging or observability code starts:

- `tracing`
- `tracing-subscriber`

### Harness And First-Slice Test Support

These are not scaffold defaults. Admit them only when first-slice harness code
or first-slice implementation tests actually need them.

- `tempfile`: acceptable for deterministic filesystem-layout tests and config
  discovery harness cases
- `assert_cmd`: defer until a real CLI surface exists for the owning slice
- snapshot-style crates such as `insta`: avoid by default; prefer explicit
  checked-in golden files in fixture directories
- diff helpers such as `pretty_assertions`: avoid by default; use standard
  assertion output unless a concrete review burden justifies them

Rules:

- harness dev-dependencies for the accepted first slice belong in
  `crates/cloudflared-config/Cargo.toml`, not the workspace root
- do not add snapshot tooling merely to make approval easier; first prefer
  stable JSON or text goldens checked into the repo
- first-slice checked-in goldens belong under
  `crates/cloudflared-config/tests/fixtures/first-slice/golden/`
- CLI-process test helpers are premature until the Rust CLI actually emits the
  relevant first-slice surface

## Disallowed By Default

These require an explicit decision record before admission:

- repo-wide actor frameworks
- alternative async runtimes
- unbounded-channel libraries as a primary coordination primitive
- allocator libraries other than the accepted process allocator baseline
- Cloudflare-owned crates that are not already proven to be the best fit for an
  active slice
- speculative HTTP, QUIC, or RPC client/server stacks before the owning slice
  starts

## Crate Ownership Rules

Current crate intent is:

- `cloudflared-cli` owns process-level concerns such as allocator setup and,
  later, runtime initialization
- `cloudflared-config` owns config, credentials, and ingress normalization once
  that slice starts
- `cloudflared-core` should stay lean and hold shared types only when more than
  one crate needs them
- `cloudflared-proto` should remain empty until protocol work starts

Do not accumulate dependencies in `cloudflared-core` just because it looks like
shared infrastructure.

## Dependency Change Checklist

Before adding a dependency, document all of the following in the change:

1. owning slice
2. owning crate
3. source-backed reason
4. why the standard library is insufficient
5. whether the dependency affects external behavior, wire bytes, config
   semantics, or shutdown behavior

If any of those answers are unclear, the dependency should not be added yet.
