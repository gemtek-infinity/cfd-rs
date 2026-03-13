# ADR 0001: Hybrid Concurrency Model

- Status: Accepted
- Date: 2026-03-08

## Context

The Rust rewrite must preserve the externally visible behavior of the frozen Go
reference in [baseline-2026.2.0/old-impl/](../../baseline-2026.2.0/old-impl/) while avoiding a naive one-goroutine-to-one-task
translation.

The repository already has a scaffold, not a blank-slate rewrite.

Two failure modes need to be prevented early:

1. introducing a repo-wide actor framework that adds mailbox hops and hidden
   behavior changes to hot paths
2. translating Go goroutines into detached Tokio tasks without clear ownership,
   cancellation, or shutdown order

The runtime model must also respect the currently admitted deterministic surfaces:

- config
- credentials
- ingress normalization

Those surfaces are largely deterministic and should not force premature async architecture into the
workspace.

## Decision

The rewrite adopts a hybrid concurrency model:

- actor-inspired control plane
- Tokio structured-async data plane
- single-owner state machines for long-lived coordinators
- direct async I/O tasks for hot-path forwarding
- bounded queues only
- explicit hierarchical cancellation and shutdown

This decision is normative for runtime subsystem work.

## Consequences

### Accepted Control-Plane Shape

Long-lived coordinators such as supervisors, orchestrators, session managers,
and connection managers should use:

- one owner task
- one authoritative mutable state holder
- bounded `mpsc` for commands/events
- `oneshot` for request/reply acknowledgements
- `CancellationToken` for shutdown hierarchy
- `JoinSet` for owned child task tracking

### Accepted Data-Plane Shape

Hot-path stream and datagram forwarding should use:

- direct async tasks
- direct socket and stream operations
- bounded buffering only
- explicit deadline and cancellation propagation

Do not route hot-path bytes through general-purpose actor mailboxes.

### Rejected Defaults

The following are rejected as default architecture:

- repo-wide actor framework adoption
- unbounded queues
- detached tasks without owner tracking
- global mutable state behind async locks as a first resort

## Current Operational State

The concurrency model described by this ADR is now operationally active.

Tokio, `tokio-util`, and related primitives are admitted in workspace
manifests and used by the admitted runtime/lifecycle shell, QUIC transport
core, Pingora proxy seam, and observability surface.

The hybrid concurrency model — actor-inspired control plane and
Tokio structured-async data plane — governs all new runtime subsystem work.

## Relationship To Other Documents

- [docs/go-rust-semantic-mapping.md](../go-rust-semantic-mapping.md) is the detailed normative doctrine
- [docs/allocator-runtime-baseline.md](../allocator-runtime-baseline.md) defines process-level runtime admission
- [docs/dependency-policy.md](../dependency-policy.md) controls when runtime dependencies enter manifests
