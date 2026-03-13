# ADR 0003: Pingora Critical Path

- Status: Accepted
- Date: 2026-03-10

## Context

The frozen Linux production-alpha lane already states that Pingora is in the
production-alpha critical path.

ADR 0002 freezes the transport / TLS / crypto lane around quiche first and
quiche + BoringSSL. That leaves one governance gap: the repository still needs
an explicit statement of where Pingora sits relative to that transport lane and
what Pingora initially owns in the production-alpha path.

This ADR is governance-level scope freeze only.
It does not add Pingora crates.
It does not implement Pingora integration.

## Decision

Pingora is part of the production-alpha critical path, but it does not replace
the quiche transport lane frozen in ADR 0002.

The governing relationship is:

- quiche remains the first transport implementation direction
- quiche + BoringSSL remains the governing transport / TLS / crypto lane
- Pingora sits above that lane in the production-alpha path
- Pingora is the initial application-layer proxy path, not the transport owner

This ADR is normative for Pingora scope in the production-alpha lane.

## Initial Responsibilities

Pingora's initial production-alpha responsibilities are:

- request handling and proxy-path orchestration above the frozen quiche
  transport lane
- origin-facing HTTP request/response forwarding responsibilities in the alpha
  critical path
- connection reuse, pooling, and related request-path lifecycle concerns at the
  application layer
- request-path policy hooks, backpressure points, and middleware-style control
  points that belong above transport ownership

## Explicit Non-Responsibilities

Pingora does not own yet:

- the quiche transport lane itself
- TLS or crypto ownership for the alpha lane
- 0-RTT implementation ownership as a completed behavior claim
- datagram or non-HTTP transport behavior
- supervisor, orchestrator, or whole-program runtime governance
- FIPS-in-alpha operational behavior
- deployment-contract assumptions

## First Admitted Pingora Crates

The first admitted Pingora crates for the production-alpha path are:

- `pingora-core`
- `pingora-http`
- `pingora-proxy`

`pingora-http` is now admitted in workspace dependencies and used by the
admitted Pingora proxy seam. The remaining crates in this set are admitted at
the governance level and may enter manifests when their owning implementation
slice starts.

## Rejected Alternatives

### Pingora As Optional / Not Critical In The Alpha Lane

Rejected because the frozen lane already states that Pingora is in the
production-alpha critical path.

### Pingora Replacing The Quiche Transport Lane

Rejected because ADR 0002 already freezes quiche first and quiche + BoringSSL
as the governing transport / TLS / crypto lane.

### Pingora As A Whole-Program Governing Framework At This Stage

Rejected because the alpha lane still needs a narrow, explicit scope boundary.
Treating Pingora as the whole-program governing framework would silently widen
its role before runtime implementation evidence exists.

### Deferring All Pingora Decisions Until Runtime Implementation

Rejected because that would leave the critical-path story ambiguous and allow
the repo to drift between incompatible ownership models during later runtime
work.

## Consequences

- future runtime and dependency work must treat Pingora as an application-layer
  critical-path component above the quiche lane, not as a transport replacement
- future dependency admission for Pingora crates must stay within the first
  admitted crate set unless governance is changed explicitly
- the repository must not describe Pingora integration as implemented merely
  because this ADR freezes scope
- later FIPS and deployment decisions must stay consistent with this Pingora ↔
  quiche relationship

## Deferred Follow-Ups

- keep FIPS and deployment governance aligned with the Pingora-to-quiche critical-path split
- realize the Pingora critical path above the frozen quiche lane without widening platform or
  artifact scope
