# ADR 0004: FIPS-In-Alpha Definition

- Status: Accepted
- Date: 2026-03-10

## Context

The frozen Linux production-alpha lane already states that FIPS belongs in the
production-alpha lane.

ADR 0002 freezes the transport / TLS / crypto lane around quiche first and
quiche + BoringSSL. ADR 0003 freezes Pingora as an application-layer
critical-path component above that lane.

Those decisions still leave one governance gap: the repository needs an
explicit definition of what "FIPS-in-alpha" means without implying that a
working FIPS implementation, certification, or deployment contract already
exists.

This ADR is governance-level definition only.
It is not dependency admission.
It is not runtime implementation.
It is not a certification claim.

## Decision

FIPS is part of the production-alpha lane, but only as a bounded governance
definition for the alpha crypto surface.

The repository adopts the following meaning for FIPS-in-alpha:

- FIPS belongs in the production-alpha lane and must be evaluated as part of
  that lane rather than deferred to a later widening phase
- the alpha lane must carry an explicit runtime crypto boundary rather than a
  vague whole-program FIPS story
- the alpha lane must carry an explicit build/link boundary for that crypto
  surface rather than treating the entire build as implicitly in scope
- the alpha lane must carry an explicit validation posture that distinguishes
  governance intent from implementation proof or certification
- the repository must not describe FIPS implementation or certification as
  existing merely because this ADR now defines the boundary

This ADR is normative for the governance meaning of FIPS-in-alpha.

## Runtime Crypto Boundary

The runtime crypto boundary for FIPS-in-alpha is bounded to the crypto surface
that directly serves the frozen production-alpha transport / TLS / crypto lane.

For the current alpha lane, that means:

- the governing runtime crypto surface is the cryptographic and TLS machinery
  directly required by the chosen quiche + BoringSSL lane
- the boundary is defined as a bounded crypto surface, not as the whole
  program, whole runtime, or every crate in the workspace
- Pingora's application-layer responsibilities stay above the transport lane
  and do not become a blanket claim that the entire request path is already a
  FIPS-implemented surface
- config loading, credentials parsing, ingress normalization, supervisors,
  orchestrators, management surfaces, and other non-crypto program regions are
  not implicitly treated as part of a working FIPS runtime boundary merely by
  existing in the same binary

This boundary definition does not claim that the bounded runtime crypto surface
is implemented yet.

## Build/Link Boundary

The build/link boundary for FIPS-in-alpha is bounded to the admitted runtime
crypto surface and its explicit crypto provider linkage, not to the whole
program by association.

For governance purposes, that means:

- any future FIPS-in-alpha implementation must make the crypto-provider
  linkage for the bounded runtime crypto surface explicit
- the repository must not treat generic Linux builds, generic local builds, or
  non-crypto crates as sufficient evidence of a FIPS-bounded build
- artifact scope remains unchanged: Linux only, `x86_64-unknown-linux-gnu`,
  GNU `x86-64-v2`, and GNU `x86-64-v4`
- this ADR does not widen artifact scope, platform scope, or crate admission
  scope

This boundary definition does not claim that a working FIPS build/link path
already exists.

## Validation Posture

The validation posture for FIPS-in-alpha is governance-first and evidence-bound.

That means:

- the repository may define the required boundary before implementation exists
- future alpha-lane work must validate the bounded crypto surface explicitly,
  not by broad whole-program inference
- future validation must distinguish build/link evidence, runtime behavior
  evidence, and any later compliance or certification evidence
- this ADR alone is not proof of a working FIPS implementation
- this ADR alone is not proof of certification
- broad operational compliance claims remain out of scope until later phases
  define and validate them explicitly

## Rejected Alternatives

### Saying "FIPS Belongs In Alpha" Without Defining The Boundary

Rejected because that would preserve ambiguity about what part of the runtime,
build, and validation surface is actually governed.

### Deferring All FIPS Meaning Until Implementation

Rejected because that would let later implementation work invent the boundary
implicitly and would make repo claims harder to review for honesty.

### Treating FIPS-In-Alpha As Equivalent To Already-Certified Implementation

Rejected because governance acceptance is not the same thing as implementation
evidence or certification.

### Allowing FIPS Scope To Spread Across The Whole Program Without A Bounded Crypto Surface

Rejected because an unbounded whole-program claim would silently widen scope,
hide the real crypto boundary, and make later validation and deployment claims
less credible.

## Explicit Non-Goals

This ADR does not:

- admit any new dependencies
- implement runtime behavior
- claim that a working FIPS implementation already exists
- claim that certification already exists
- define the deployment contract
- make a broad operational compliance claim beyond the alpha governance
  definition

## Consequences

- future runtime, dependency, and validation work must preserve a bounded FIPS
  crypto surface for the alpha lane instead of treating FIPS as a whole-program
  label
- future implementation work must make the build/link boundary explicit for the
  admitted crypto surface
- the repository must remain explicit that FIPS is part of the production-alpha
  lane as a governance commitment, not as proof of working implementation or
  certification
- later deployment and validation phases must stay consistent with the runtime
  and build/link boundaries frozen here

## Deferred Follow-Ups

- Phase 2.5: define the Linux deployment contract that any later FIPS-capable
  alpha would have to honor operationally
- Big Phase 3: realize the bounded runtime crypto surface and explicit
  build/link path without widening platform or artifact scope
- Big Phase 4: validate the implemented boundary with real runtime evidence,
  operational evidence, and any certification-adjacent proof required for
  credible claims
- any broader compliance, certification, or post-alpha widening work: only
  after explicit governance change and evidence, not by implication from this
  ADR
