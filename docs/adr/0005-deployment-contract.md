# ADR 0005: Deployment Contract

- Status: Accepted
- Date: 2026-03-10

## Context

The frozen Linux production-alpha lane already fixes the platform, artifact,
transport, Pingora, and FIPS governance direction.

Those decisions still leave one governance gap: the repository needs an
explicit Linux production-alpha deployment contract so later runtime and
validation work does not invent operational assumptions implicitly.

This ADR is governance-level deployment contract only.
It is not runtime implementation.
It is not packaging or installer implementation.
It is not deployment automation.

## Decision

The production-alpha lane adopts a narrow Linux deployment contract.

That contract means:

- the governing deployment baseline is Linux only on the already frozen target
  triple `x86_64-unknown-linux-gnu`
- the governing operational baseline is GNU/glibc, consistent with the already
  frozen shipped GNU artifact scope
- the governing service model is a narrow host-supervised service model rather
  than a broad multi-init or platform-agnostic support claim
- the governing deployment stance is bare-metal-first rather than
  container-first for the alpha contract
- the governing filesystem/layout expectations must be explicit enough that
  later packaging or deployment work can be judged against them

This ADR is normative for the Linux production-alpha deployment contract.

## Platform Baseline

The deployment contract is Linux only.

For the production-alpha lane, that means:

- the governing platform remains `x86_64-unknown-linux-gnu`
- the governing operational baseline is GNU/glibc, not musl and not a
  multi-platform contract
- shipped artifacts remain exactly the already frozen GNU lanes:
  - `x86-64-v2`
  - `x86-64-v4`
- this ADR does not widen distro support into a broad Linux-anything claim; it
  only freezes the contract around the existing GNU/glibc alpha lane

## Service/Supervisor Expectations

The deployment contract assumes a supervised long-running service environment
on Linux hosts.

For the alpha contract, that means:

- the runtime is expected to operate under an external service supervisor
  rather than as an ad hoc one-shot packaging target
- systemd is the governing service expectation for the alpha contract
- this expectation does not mean systemd units or service files already exist
- broad multi-init support is not part of the governing alpha contract
- the contract is about operational expectations, not about shipped service
  assets in the current repository state

## Deployment Stance

The governing alpha deployment stance is bare-metal-first, not container-first.

That means:

- the deployment contract is defined first around host deployment assumptions
  on the Linux GNU/glibc lane
- container execution is not forbidden, but it is not the governing contract
  for alpha acceptance
- this ADR does not claim that container images, container deployment flows, or
  container-specific support assets already exist
- later container support, if admitted, must be evaluated as an explicit
  extension of the contract rather than assumed by default

## Filesystem/Layout Expectations

The deployment contract requires explicit filesystem and layout expectations at
the contract level.

For the alpha contract, that means:

- the executable is expected to live at a stable operator-managed host path
- configuration, credentials, logs, and runtime state must be treated as
  explicit operator-managed filesystem concerns rather than implicit packaging
  side effects
- filesystem/layout expectations must remain compatible with a supervised host
  service model and with the current narrow first-slice config and credential
  surfaces
- this ADR freezes the need for explicit layout expectations without claiming
  that final package-owned paths, installer behavior, or updater behavior
  already exist

## Rejected Alternatives

### Deferring Deployment Assumptions Until Implementation

Rejected because that would allow runtime, packaging, or environment choices to
smuggle in operational assumptions before the repo defines the governing
contract.

### Container-First As The Governing Alpha Contract

Rejected because the alpha lane is being frozen around a narrow Linux host
contract, not around container image delivery as the primary operational story.

### Broad Multi-Init Support In Alpha

Rejected because the alpha contract must stay narrow enough to evaluate.
Broad init-system support is widening work for later phases, not part of the
governing alpha contract.

### Broad Distro/Platform Support In Alpha

Rejected because the lane is already frozen around Linux, GNU/glibc, and the
existing shipped GNU artifact policy.

### Leaving Filesystem/Layout Expectations Implicit

Rejected because later packaging and deployment work cannot be reviewed
honestly if the contract never states what host-side layout assumptions are
allowed.

## Explicit Non-Goals

This ADR does not:

- admit any new dependencies
- implement runtime behavior
- implement packaging or installers
- implement deployment automation
- implement container images
- widen platform or artifact scope

## Consequences

- future runtime, packaging, and operational work must stay consistent with a
  Linux GNU/glibc host contract instead of inventing deployment assumptions ad
  hoc
- future service assets, if added, must be judged against the narrow systemd /
  supervised-service expectation frozen here
- future container support, if any, must be admitted explicitly and must not be
  described as the governing alpha contract unless governance changes
- the repository must remain explicit that this ADR defines deployment
  assumptions only; it does not prove deployment tooling, packaging, container
  support, or automation already exists

## Deferred Follow-Ups

- Big Phase 3: realize the minimum runnable alpha against the frozen Linux GNU/
  glibc host contract without widening platform or artifact scope
- Big Phase 4: validate the deployment contract with real operational evidence,
  hardening work, and measured runtime behavior in actual supervised use
- Big Phase 5: consider any broader distro, init-system, packaging, or
  container widening only through explicit governance change and evidence
