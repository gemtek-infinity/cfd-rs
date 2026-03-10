# Promotion Gates

This document defines the current rewrite phase model, the active task inside
that model, and the gates for promotion from one stage to the next.

A later stage must not begin in substance until the earlier stage's gate is
satisfied or explicitly waived.

## Purpose

The rewrite should progress by evidence, not by excitement.

This file exists so that humans and AI assistants cannot silently skip from
accepted evidence to broader scope without closing the required gates.

## Big Phase 1 — Truth Freeze And Accepted First-Slice Parity

### Purpose

Freeze the rewrite doctrine, accept the first implementation slice, and close
the first-slice Rust-versus-Go compare loop.

### Required conditions

- compatibility baseline is explicitly frozen
- source precedence is explicit
- frozen inputs are explicit
- active Linux production-alpha lane is explicit
- active and deferred scope are explicit
- current workspace honesty rule is explicit
- first accepted implementation slice is explicit
- first-slice Go-truth capture and compare loop is real
- accepted first-slice compare is green
- "done means parity" is explicit

### Evidence

- `STATUS.md`
- `docs/compatibility-scope.md`
- `REWRITE_CHARTER.md`
- `docs/first-slice-freeze.md`

### Exit condition

Big Phase 1 is complete when the rewrite doctrine is frozen and the accepted
first-slice compare is green against checked-in Go truth.

### Not allowed before exit

- broad subsystem porting
- claiming accepted parity without a real compare loop
- treating the rewrite as operationally real beyond the accepted first slice

## Big Phase 2 — Linux Production-Alpha Lane Freeze

### Purpose

Freeze the Linux production-alpha lane before broad implementation resumes.

This big phase is governance-first. It is not the stage for broad runtime,
transport, Pingora, FIPS operational, or deployment implementation work.

### Phase 2.0 — Governance Realignment

#### Purpose

Align repo truth to the chosen lane, remove contradictions, and keep the docs
honest.

#### Required conditions

- charter, status, and compatibility wording agree on the active lane
- the current phase is stated as Big Phase 2 with 2.0 as the active task
- no governance doc implies that broader implementation is already complete
- no governance doc implies broader platform or artifact scope than intended

#### Exit condition

Phase 2.0 is complete when governance docs consistently describe the same Linux
production-alpha lane and current task.

### Phase 2.1 — Build And Artifact Policy

#### Purpose

Define the real production-alpha build and shipped-artifact policy.

#### Required conditions

- CI/build policy reflects the Linux-only alpha lane
- shipped GNU artifacts are defined as exactly:
  - `x86-64-v2`
  - `x86-64-v4`
- artifact naming and checksum naming are explicit
- build matrix policy is explicit

#### Exit condition

Phase 2.1 is complete when build and artifact policy is explicit in governance.

### Phase 2.2 — Transport / TLS / Crypto ADR

#### Purpose

Freeze the transport and crypto direction for the alpha lane.

#### Required conditions

- 0-RTT is explicitly required
- quiche is explicitly first
- quiche + BoringSSL is explicitly chosen for the alpha lane
- quiche + OpenSSL is explicitly out for the alpha lane
- the PQC-compatible QUIC direction is explicit

#### Exit condition

Phase 2.2 is complete when the transport / TLS / crypto lane is frozen in ADRs.

### Phase 2.3 — Pingora Critical-Path ADR

#### Purpose

Define Pingora's place in the production-alpha critical path.

#### Required conditions

- the critical-path relationship between Pingora and quiche is explicit
- Pingora's initial responsibilities are explicit
- the first admitted Pingora crates are explicit

#### Exit condition

Phase 2.3 is complete when Pingora scope is frozen at the governance level.

### Phase 2.4 — FIPS-In-Alpha Definition

#### Purpose

Define what FIPS-in-alpha means in this repository without implying that the
implementation already exists.

#### Required conditions

- FIPS is explicitly part of the production-alpha lane
- runtime crypto boundary is explicit
- build/link boundary is explicit
- validation posture is explicit
- deferred detail beyond the alpha lane is explicit

#### Exit condition

Phase 2.4 is complete when the repo has a clear governance definition for
FIPS-in-alpha.

### Phase 2.5 — Deployment Contract

#### Purpose

Freeze the deployment assumptions for the Linux production-alpha lane.

#### Required conditions

- glibc assumptions are explicit
- systemd/service expectations are explicit
- container vs bare-metal assumptions are explicit
- filesystem/layout expectations are explicit

#### Exit condition

Phase 2.5 is complete when deployment-contract assumptions are explicit in
governance.

### Phase 2.6 — Standard-Format And Workspace-Dependency Admission

#### Purpose

Freeze repository-wide policy for mature standard-format handling and shared
dependency truth without turning this phase into blanket dependency expansion.

#### Required conditions

- mature standard-format handling policy is explicit
- direct-upstream-loader preference is explicit
- `[workspace.dependencies]` is stated as the default truth for shared
  third-party crates
- active-slice ownership and minimum-feature rules are explicit
- exception handling is explicit
- the phase wording makes clear that this is governance, not speculative
  dependency expansion

#### Exit condition

Phase 2.6 is complete when standard-format admission and workspace-dependency
policy are explicit enough to prevent reinvention and scattered manifest truth.

### Exit condition

Big Phase 2 is complete when Phases 2.0 through 2.6 have frozen the Linux
production-alpha lane well enough to start building the minimum runnable alpha.

### Not allowed before exit

- pretending that 2.1 through 2.5 are already done when they are not
- broadening platform scope beyond Linux
- broadening shipped artifact scope beyond GNU `x86-64-v2` and `x86-64-v4`
- treating transport, Pingora, FIPS operational, or deployment implementation
  as already landed
- reinventing mature standard-format handling by default where a direct
  upstream loader or a mature crate is the clearer active-slice choice
- silently scattering shared dependency truth across crate manifests by default
- using convenience dependencies to imply new application-level crypto behavior

## Big Phase 3 — Minimum Runnable Alpha

### Purpose

Build the minimum runnable alpha on the frozen Linux lane.

### Exit condition

The minimum runnable alpha exists on the frozen Linux production-alpha lane.

### Not allowed before exit

- calling the alpha hardened or production-proven
- widening platform or artifact scope as a shortcut around missing evidence

## Big Phase 4 — Hardening, Validation, And Proof

### Purpose

Harden, validate, measure, and prove the alpha in real use.

### Exit condition

The promoted alpha scope is validated well enough to be credible in real use.

## Big Phase 5 — Intentional Widening

### Purpose

Widen scope only after the alpha is credible and the reason for widening is
explicit.

### Exit condition

Additional platforms, artifacts, or subsystem scope are admitted intentionally,
not by drift.

## Promotion Rule

A stage may be promoted only when:

1. its exit condition is met, or
2. an explicit written waiver is added in governance docs explaining why the
   promotion is acceptable despite incomplete evidence

Silently behaving as though a stage is complete is not allowed.

## Current Phase Reading

At the current repo state:

- Big Phase 1 is done
- Big Phase 2 is current
- Phase 2.6 is the active task
- Big Phase 3 begins only after the Linux production-alpha lane is frozen in
  governance
