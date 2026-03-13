# Go To Rust Semantic Mapping

This document is the normative concurrency and lifecycle doctrine for the Rust
rewrite.

This document is complemented by
`docs/adr/0001-hybrid-concurrency-model.md`, which records the architectural
decision at ADR level.

It is intentionally not a crate-substitution cheat sheet. The rewrite target is
behavioral compatibility with `baseline-2026.2.0/old-impl/`, not one-for-one library replacement.

## Current Application

This doctrine is operationally active.

The workspace now has an admitted runtime/lifecycle shell, QUIC transport
core, Pingora proxy seam, and observability surface. Tokio, `tokio-util`,
and the hybrid concurrency primitives described below are admitted in
workspace manifests and used by active runtime code.

The doctrine governs all new runtime and async subsystem work.

## Doctrine Summary

The rewrite uses a hybrid concurrency model:

- actor-inspired control plane
- Tokio structured-async data plane
- single-owner state machines for long-lived coordinators
- direct async tasks for hot-path I/O
- bounded queues only
- explicit lifecycle ownership
- explicit hierarchical cancellation and shutdown

This means:

- long-lived coordinators such as supervisors, orchestrators, session managers,
  and config owners should have one clear owner task and one clear mutable state
  authority
- hot-path stream and datagram forwarding should use direct async tasks and
  direct I/O primitives rather than mailbox hops
- no repo-wide actor framework is adopted by default
- no subsystem may rely on detached tasks with unclear ownership

## Synchronous And Deterministic Work

For slices that are primarily parsing, validation, or normalization work
(such as config, credentials, and ingress normalization):

- prefer direct synchronous parsing and validation code
- do not introduce task graphs, channels, or runtime ownership merely to mimic
  daemon structure
- keep CLI-origin normalization thin and deterministic
- reserve async/runtime machinery for slices that genuinely need it

## Control Plane vs Data Plane

### Control Plane

The control plane includes:

- supervisors
- orchestrators
- config owners
- session registries
- connection managers
- management/control RPC coordinators

Control-plane work should be modeled as single-owner state machines with
bounded mailboxes and explicit ownership of their children.

Default shape:

- one owner task
- one mutable state authority
- bounded `mpsc` for commands/events
- `oneshot` for request/reply acknowledgements
- `CancellationToken` for parent-child shutdown
- `JoinSet` for owned child tasks

### Data Plane

The data plane includes:

- stream forwarding
- datagram forwarding
- request/response proxying
- bidirectional copy loops
- flush-sensitive HTTP/SSE/gRPC paths

Data-plane work should use direct async tasks and direct I/O primitives.
Avoid routing hot-path bytes through general control-plane mailboxes.

Default shape:

- spawned task with explicit owner
- direct socket/stream operations
- bounded buffering only
- explicit timeout/cancellation propagation
- `copy_bidirectional` only when half-close semantics match the Go behavior

## Default Primitives

These are the default choices unless the Go behavior proves otherwise.

| Primitive | Default use | Why |
| --- | --- | --- |
| bounded `tokio::sync::mpsc` | control-plane command/event queues | explicit backpressure and queue ownership |
| `tokio::sync::oneshot` | request/reply completion, acknowledgements, one-result handoff | preserves rendezvous-like intent without introducing shared mutable reply state |
| `tokio_util::sync::CancellationToken` | hierarchical shutdown and cancellation trees | explicit parent-child cancellation ownership |
| `tokio::task::JoinSet` | owned child task management | structured task ownership and join-on-shutdown |
| `tokio::sync::Semaphore` | admission control and bounded concurrent work | explicit concurrency budget instead of implicit queue growth |
| `tokio::time::interval` | periodic maintenance loops | standard periodic scheduling with cancellation points |
| `std::sync::Mutex` / `std::sync::RwLock` | short, non-async critical sections | lowest-overhead ownership boundary when no await is needed |

## Conditional Primitives

These are valid, but only with a narrow reason tied to Go behavior.

| Primitive | Use when | Do not use when | Main risk |
| --- | --- | --- | --- |
| `tokio::sync::watch` | latest-value state publication is the actual contract and lagging consumers only need the newest state | every event matters or intermediate transitions matter | silently collapsing state transitions |
| `tokio::sync::broadcast` | one producer must fan out discrete notifications to multiple subscribers and lagging is acceptable/documented | consumers must observe every event or subscriber lag is unacceptable | dropped messages under pressure |
| `tokio::sync::Mutex` / `tokio::sync::RwLock` | state must stay locked across an async suspension and redesign would materially complicate correctness | a sync lock or ownership transfer can avoid holding across await | contention, deadlocks, scheduler stalls |
| `tokio::io::copy_bidirectional` | the Go path is a symmetric duplex stream bridge and default half-close behavior matches the contract | one side requires protocol-aware flushing, framing, or special close ordering | subtle half-close mismatch |
| `tokio::time::timeout` / `timeout_at` | the Go behavior expresses a hard deadline on one operation | the real contract is a session-wide budget or coordinated shutdown deadline | turning policy deadlines into local call wrappers |

## Discouraged Patterns

These patterns should be treated as exceptions that require explicit written
justification.

- unbounded queues
- detached `tokio::spawn` without owner tracking
- repo-wide actor framework adoption
- generic mailboxing for hot-path stream/data forwarding
- global mutable state behind async locks
- using `watch` as an event bus
- using `broadcast` as a guaranteed-delivery channel
- recursive sleep loops when `interval` expresses the behavior more clearly
- replacing shutdown with best-effort task drop

## Primitive Guidance

### bounded `mpsc`

Use for control-plane command and event queues.

Rules:

- every queue must have an explicit capacity
- capacity must be chosen from behavioral constraints, not convenience
- sender ownership and receiver ownership must be documented
- queue saturation must be treated as a design signal, not ignored

### `oneshot`

Use for request/reply, readiness acknowledgements, and single-result handoff.

Rules:

- prefer `oneshot` over shared mutable reply slots
- cancellation of either side must be handled explicitly

### `watch`

Use only for latest-state publication where overwriting intermediate values is
acceptable.

Rules:

- do not use `watch` where every event matters
- do not use `watch` to simulate a work queue

### `broadcast`

Use only for lossy fan-out notifications where lagging receivers may miss old
messages without violating behavior.

Rules:

- treat lag and dropped messages as part of the contract if used
- do not use for authoritative control-plane commands

### `Semaphore`

Use for explicit admission control, concurrent stream/session limits, and
resource budgeting.

Rules:

- prefer `Semaphore` over hidden concurrency limits embedded in queue size
- release ordering must align with the Go lifecycle contract

### `JoinSet`

Use for child tasks owned by a supervisor, orchestrator, or manager.

Rules:

- parent owns spawn and join responsibility
- child failures must be surfaced to the owner
- shutdown should await child completion in a defined order

### `CancellationToken`

Use as the default cancellation tree primitive.

Rules:

- one parent token per ownership boundary
- derive child tokens for owned child tasks
- do not replace deadline semantics with plain cancellation

### `Mutex` / `RwLock`

Use sync locks by default for short critical sections that do not cross await.

Rules:

- use async locks only when the lock must remain held across async suspension
- avoid lock-based sharing where message passing or single ownership is simpler

### `copy_bidirectional`

Use only for symmetric duplex forwarding where the Go path does not require
protocol-aware flushing, custom close ordering, or frame visibility.

Rules:

- verify half-close behavior explicitly
- verify shutdown ordering explicitly

### `interval` / timers

Use `interval` for maintenance loops and repeated background work.

Rules:

- cancellation must be checked every cycle
- timer ownership must be explicit
- choose between periodic cadence and deadline budget consciously

## Supervisor Ownership Model

Supervisors, orchestrators, and other long-lived coordinators should follow
this shape:

1. one owner task owns the authoritative mutable state
2. child work runs in owned tasks tracked by `JoinSet`
3. commands arrive through bounded `mpsc`
4. replies and completion acknowledgements use `oneshot` where needed
5. cancellation flows from parent token to child tokens
6. shutdown joins children in a defined order

This is actor-inspired control, but not a repo-wide actor framework.

## Request/Reply Patterns

Prefer these shapes:

- bounded `mpsc` command + `oneshot` reply
- direct async call when no queue boundary is needed
- explicit typed result enums for owner-managed state machines

Do not default to:

- shared mutable reply state
- multiplexed unbounded response queues
- generic mailbox wrappers for simple direct calls

## Backpressure Rules

Backpressure is part of compatibility, not just performance.

Rules:

- every queue is bounded
- every bounded queue must have an owner and a saturation strategy
- hot-path data forwarding should prefer direct I/O and semaphores over mailbox chains
- buffering must not silently grow beyond the Go behavior

## Shutdown And Cancellation Rules

Shutdown must be explicit and hierarchical.

Rules:

- every long-lived task has a clear owner
- every owner has a shutdown path
- every child task is either joined or deliberately detached with written justification
- cancellation tokens express ownership hierarchy
- graceful shutdown ordering must be tested, not assumed

## Timeouts And Deadlines

Do not flatten all Go timeout behavior into `timeout(...)` wrappers.

Rules:

- distinguish per-operation timeout from session lifetime budget
- distinguish shutdown deadline from cancellation signal
- carry deadlines explicitly when the Go behavior carries them explicitly
- test timeout paths for externally visible behavior, not only local errors

## Shared-State Rules

Default to single-owner state machines for long-lived mutable coordinator state.

Use shared-state primitives only when:

- ownership transfer is impractical
- the protected data is genuinely shared
- the lock scope is small and behaviorally clear

If a state machine can own the state directly, prefer ownership over shared
locking.

## Go Migration Traps

These Go patterns are the easiest to mistranslate.

1. unbuffered channel rendezvous treated as ordinary queue send
2. buffered channels replaced with unbounded queues
3. `context.Context` reduced to cancellation only, losing deadline semantics
4. goroutines translated to detached tasks without ownership
5. supervisor loops translated into many independent retries rather than one state machine
6. close-driven shutdown translated into dropped tasks without join order
7. `io.Copy` style bridges translated without validating half-close behavior
8. ticker loops translated without equivalent cancellation and cadence semantics

## Test Implications

Parity tests must reflect this doctrine.

Every subsystem test plan should include, where relevant:

- queue saturation and backpressure behavior
- shutdown ordering and graceful completion behavior
- cancellation propagation from parent to child tasks
- timeout/deadline behavior at externally visible boundaries
- ownership behavior for coordinators and supervised children
- stream half-close behavior and flush-sensitive paths

A subsystem is not parity-complete merely because it compiles or passes unit
tests under nominal load. It needs evidence that its lifecycle, backpressure,
and shutdown semantics still match `baseline-2026.2.0/old-impl/`.

For synchronous and deterministic work (parsing, config, credentials, ingress),
parity emphasis shifts toward:

- deterministic parse and validation outcomes
- fixture-backed normalization behavior
- preservation of precedence and fallback rules
- explicit non-claiming of runtime behavior that is not yet implemented
