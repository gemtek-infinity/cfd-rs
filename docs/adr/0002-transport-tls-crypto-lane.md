# ADR 0002: Transport / TLS / Crypto Lane

- Status: Accepted
- Date: 2026-03-10

## Context

The frozen Linux production-alpha lane already has non-negotiable top-level
decisions for transport direction, TLS/crypto posture, and artifact scope.

Those decisions need to be frozen in ADR form before implementation work begins
so the repository stops drifting between incompatible transport and TLS stories.

The repository must also stay honest about the difference between:

- what is decided for the alpha lane, and
- what has actually been implemented so far

This ADR is governance-level lane freeze only.
It is not dependency admission.
It is not runtime implementation.

## Decision

The production-alpha lane adopts the following transport / TLS / crypto
decisions:

- 0-RTT is required for the alpha lane
- quiche is the first transport implementation direction
- quiche + BoringSSL is the chosen transport / TLS / crypto lane for the alpha
  lane
- quiche + OpenSSL is out for the alpha lane
- the PQC-compatible QUIC direction is part of the chosen lane

This ADR is normative for alpha-lane transport / TLS / crypto decisions.

## Rationale

- the lane needs one explicit governing transport and TLS story before runtime
  implementation begins
- 0-RTT is a lane requirement and should not be left implicit until code lands
- quiche first keeps the transport direction narrow enough to prevent drift into
  multiple competing alpha stacks
- choosing quiche + BoringSSL now prevents later ambiguity about the governing
  crypto posture for the alpha lane
- freezing the PQC-compatible QUIC direction now keeps the lane aligned with the
  selected production-alpha story without claiming implementation already exists

## Rejected Alternatives

### Rustls As The Governing Alpha-Lane Choice

Rejected because the alpha lane is being frozen around quiche first and
quiche + BoringSSL, not around rustls as the governing transport / TLS choice.

### Quiche + OpenSSL

Rejected because the chosen alpha lane is quiche + BoringSSL.
Keeping OpenSSL as an equal candidate would re-open the lane decision this ADR
is supposed to freeze.

### Deferring The Transport / TLS / Crypto Choice Until Implementation

Rejected because implementation-first decision making would allow the repo to
drift between incompatible lane stories and would make later dependency or
runtime changes harder to evaluate honestly.

### Broad Multi-Stack Support In The Alpha Lane

Rejected because the alpha lane must stay narrow enough to be reviewable.
Broad multi-stack support is widening work for later phases, not for the frozen
alpha lane.

## Explicit Non-Goals

This ADR does not:

- admit any new dependencies
- implement runtime behavior
- claim that 0-RTT behavior is already implemented
- define Pingora scope beyond the already frozen lane-level critical-path note
- define FIPS-in-alpha operational behavior
- define the deployment contract

## Consequences

- future transport and TLS changes must be evaluated against this lane decision
- dependency admission for quiche, BoringSSL bindings, or adjacent crates is a
  later step and is not implied by this ADR alone
- the repository must not describe quiche, BoringSSL, or 0-RTT as implemented
  merely because they are now governance-level lane decisions
- later ADRs for Pingora, FIPS, and deployment must stay consistent with this
  transport / TLS / crypto lane

## Deferred Follow-Ups

- keep Pingora, FIPS, and deployment ADRs aligned with this lane decision
- realize the chosen lane in actual transport, runtime, and validation work without widening
  platform or artifact scope
