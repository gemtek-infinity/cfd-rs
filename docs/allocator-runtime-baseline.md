# Allocator And Runtime Baseline

This document defines the process-level baseline for the Rust scaffold and the
rules for introducing the async runtime.

The workspace already has a runnable binary scaffold. This document keeps that
binary honest without implying that runtime subsystems are already ported.

## Frozen Inputs

Do not modify either frozen input as part of allocator or runtime work:

- `baseline-2026.2.0/old-impl/`
- `baseline-2026.2.0/design-audit/`

## Target Platform

The primary target remains:

- `x86_64-unknown-linux-gnu`

This baseline is written for that target first.

## Allocator Baseline

The runnable binary must use `mimalloc` as the global allocator.

Required feature set:

- `override`
- `no_thp`
- `local_dynamic_tls`
- `extended`

Rules:

- allocator configuration lives only in the runnable binary crate
- library crates must not declare a global allocator
- changing allocator policy requires explicit review because allocator behavior
  is process-wide and operationally visible

## Runtime Baseline

The current scaffold now initializes Tokio only at the binary boundary for the
admitted runtime/lifecycle shell that carries the Phase 3.3 QUIC tunnel core
and the admitted Phase 3.4–3.7 and 4.1 layers above it.

Rules:

- keep Tokio ownership at the runnable binary boundary
- do not add detached background runtime behavior to make later slices look
  more complete than they are
- keep runtime-owned lifecycle behavior explicit and bounded to the admitted
  shell rather than smuggling in later transport or proxy work

## Async Runtime Admission Rule

When the first async subsystem slice begins, the accepted runtime baseline is:

- Tokio runtime

Constraints:

- runtime ownership belongs at the binary boundary
- libraries should remain runtime-aware only to the extent required by their
  public async interfaces
- runtime structure must follow `docs/go-rust-semantic-mapping.md`
- concurrency architecture is formally recorded in
  `docs/adr/0001-hybrid-concurrency-model.md`

## Control-Plane And Data-Plane Consequences

When Tokio enters the workspace, it is not a license to treat every subsystem
as an actor or to put every path behind mailboxes.

The accepted model is:

- actor-inspired control plane
- direct Tokio async tasks for hot-path data plane
- bounded queues only
- explicit ownership, cancellation, and join discipline

## Scaffold Honesty Rule

The scaffold must remain visibly partial.

At the current 4.1 state:

- runtime initialization owns lifecycle, supervision, the admitted QUIC
  transport core, the Pingora proxy seam, wire/protocol boundary,
  security/compliance boundary, standard-format integration, and
  observability/operability reporting
- no allocator or runtime code should imply broader compatibility already exists
- manifests should remain sparse enough that `cargo check` reflects reality
