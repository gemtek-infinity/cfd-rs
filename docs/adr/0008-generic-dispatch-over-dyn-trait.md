# ADR 0008: Generic Dispatch Over dyn Trait For Closed Service Sets

- Status: Accepted
- Date: 2026-03-16

## Context

Three `dyn Trait` dispatch patterns were introduced during early parity work,
following Go's open interface conventions. In Go, open interfaces are idiomatic
and cheap. In Rust, `Box<dyn Trait>` introduces heap allocation, vtable
indirection, and type erasure — appropriate for open extension points, but
wrong for sets of types that are closed at compile time.

The three patterns and why they are closed:

- `ServiceManager { services: HashMap<String, Box<dyn Service>> }` in
  `cfdrs-his` — the set of managed service types is determined by
  `cfdrs-bin` at composition time. `cfdrs-his` has no extension point.
- `RuntimeServiceFactory::create_primary() -> Box<dyn RuntimeService>` in
  `cfdrs-bin` — transport variants are `Protocol::Quic` and `Protocol::Http2`.
  The protocol enum is already closed. A factory trait that returns an erased
  service type adds indirection without adding extensibility.
- `FileWatcher::start(notifier: Box<dyn WatcherNotification>)` in `cfdrs-his`
  — `WatcherNotification` has two methods and one caller. Single-method (or
  few-method) trait objects used only as callbacks are the canonical closure
  use case in Rust.

## Decision

For closed service sets and callback dispatch, this repository uses:

1. **Generic structs over `Box<dyn Trait>`** for managers holding heterogeneous
   service collections. `ServiceManager<S: ManagedService>` holds `S` values
   directly — no heap allocation per service, exhaustive match possible at the
   call site.

2. **Enums over factory traits returning `Box<dyn Trait>`** for closed variant
   sets. `enum TransportService { Quic(QuicTunnelService) }` makes protocol
   variants explicit and exhaustively matched. `ApplicationRuntime` is no
   longer parameterised over a factory trait.

3. **`impl Fn` closures over `Box<dyn SingleMethodTrait>`** for callback
   dispatch. `FileWatcher::start(on_change: impl Fn(&Path), on_error: impl Fn)`
   is zero-allocation, inlineable, and requires no trait definition for the
   caller to implement.

The rule of thumb: use `dyn Trait` only when the set of implementors is
genuinely open — i.e. when third-party or downstream code must be able to
provide new implementations at runtime without a recompile. For everything
closed in this workspace, prefer generics or enums.

## Consequences

- `ServiceManager<S>` in `cfdrs-his` is generic over `S: ManagedService`.
  `cfdrs-bin` instantiates it with a concrete enum or struct type.
- `ApplicationRuntime` in `cfdrs-bin` is no longer parameterised over
  `F: RuntimeServiceFactory`. Transport selection uses `enum TransportService`.
- `FileWatcher::start` in `cfdrs-his` accepts `impl Fn` closures, not
  `Box<dyn WatcherNotification>`.
- New service types (e.g. the H2 transport) are added as enum variants, not
  new `Box<dyn RuntimeService>` implementors. The compiler enforces exhaustive
  handling.
- Code that needs genuine runtime extensibility (e.g. test harness injection
  via trait bounds on `AppManager`) may still use trait generics — but the
  bound should be a type parameter, not an erased `Box<dyn>`.
