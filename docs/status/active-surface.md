# Active Surface Status

This file captures the currently admitted executable surface and the immediate
deferred scope around it.

## Active Phase 3.3 Focus

Phase 3.3 now owns the QUIC tunnel core for the frozen Linux
production-alpha lane.

What it covers now:

- `run` now enters a real quiche-based transport service under the runtime
  boundary
- connection/session ownership and QUIC handshake state are explicit
- runtime-owned config handoff now feeds the transport identity boundary
- reconnect/restart policy remains owned by runtime supervision rather than
  transport internals or the CLI shell
- the current transport core stops honestly after QUIC establishment where the
  later wire/protocol registration and Pingora slices do not yet exist

What it still must not imply:

- that Pingora integration exists
- that wire/protocol behavior beyond the transport-owned boundary exists
- that security/compliance operational behavior exists
- that standard-format crate integration beyond active-slice need exists
- that packaging, installers, updaters, or deployment tooling already exist

## Deferred Within Big Phase 3

The following later Big Phase 3 slices remain intentionally deferred:

- 3.4 Pingora integration path above that transport lane
- 3.5 wire/protocol boundary
- 3.6 security/compliance operational boundary
- 3.7 standard-format crate integration boundary

## Deferred Beyond Big Phase 3

The following remain intentionally out of the current executable-surface task:

- broader platform parity beyond Linux
- broader artifact scope beyond GNU `x86-64-v2` and `x86-64-v4`
- transport, Pingora, wire/protocol, security/compliance, and
  standard-format integration work outside their later owning slices
- packaging, deployment tooling, container support, and
  certification-proving work beyond the current numbered Big Phase 3 slice list
