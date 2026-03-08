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

The current scaffold does not initialize an async runtime yet.

That is intentional.

Rules:

- do not start Tokio merely because the workspace contains a CLI crate
- do not add detached background runtime behavior to make the scaffold look more
  complete than it is
- keep the current binary message explicit that no runtime behavior has been
  implemented yet

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

The scaffold must remain visibly a scaffold.

Until subsystem code lands:

- no runtime initialization should pretend the daemon exists
- no allocator or runtime code should imply compatibility already exists
- manifests should remain sparse enough that `cargo check` reflects reality
