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

Freeze repository-wide policy for mature standard-format handling and normal
workspace-managed dependency truth without turning this phase into blanket
dependency expansion.

#### Required conditions

- mature standard-format handling policy is explicit
- direct-upstream-loader preference is explicit
- `[workspace.dependencies]` is stated as the default truth and first review
  surface for normal workspace-managed third-party dependencies
- crate-local dependency declarations are explicit exceptions when isolation is
  intentional and documented
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
- silently scattering normal workspace-managed dependency truth across crate
  manifests by default
- using convenience dependencies to imply new application-level crypto behavior

## Big Phase 3 — Minimum Runnable Alpha

### Purpose

Build the minimum runnable alpha on the frozen Linux lane.

This big phase starts by making the executable boundary real without
pretending that later runtime, transport, Pingora, FIPS operational, or
packaging slices already exist.

### Phase 3.1 — CLI And Process Surface

#### Purpose

Make the admitted executable boundary real for the alpha lane.

#### Required conditions

- a real Linux alpha process entrypoint exists
- admitted command behavior is explicit and narrow
- admitted startup/help/error behavior is explicit
- admitted flags, environment, and defaults are explicit only for the current
  alpha path
- the executable stays honest about what is implemented now versus later
  deferred slices

#### Exit condition

Phase 3.1 is complete when `cloudflared` has a narrow executable surface that
can resolve config, validate startup inputs, and fail honestly before later
runtime work.

### Phase 3.2 — Runtime / Lifecycle Core

#### Purpose

Land the minimum runtime and lifecycle core required after the entry boundary.

#### Required conditions

- `run` enters a real runtime/lifecycle owner rather than only a CLI shell
- runtime-owned config handoff exists after startup validation
- startup sequencing is explicit
- shutdown sequencing is explicit
- supervision and restart policy boundaries are explicit
- later transport and proxy slices plug into explicit runtime-owned service
  boundaries instead of ad hoc process logic

#### Exit condition

Phase 3.2 is complete when the binary owns a real runtime/lifecycle shell that
can supervise deferred service boundaries, shut down cleanly, and fail honestly
before 3.3+ subsystem work exists.

### Phase 3.3 — QUIC Tunnel Core

#### Purpose

Realize the frozen quiche-first tunnel core without widening scope.

#### Required conditions

- the runtime-owned primary service is a real quiche-based QUIC transport
- connection and session ownership are explicit under the runtime boundary
- dial, establish, and teardown behavior are explicit
- transport failures map back into runtime supervision honestly
- the transport shape preserves the quiche-first, 0-RTT-required lane
- later Pingora and broader wire/protocol behavior remain explicitly deferred

#### Exit condition

Phase 3.3 is complete when the binary owns a real QUIC tunnel core that can
establish the quiche lane, report transport lifecycle honestly, and stop
explicitly before 3.4+ layers are implemented.

### Phase 3.4 — Pingora Integration Path

#### Purpose

Realize the admitted Pingora integration path above the frozen transport lane.

### Phase 3.5 — Wire / Protocol Boundary

#### Purpose

Realize the wire and protocol boundary required beyond the launch surface
without implying broader runtime or transport completion.

### Phase 3.6 — Security / Compliance Operational Boundary

#### Purpose

Realize the admitted security and compliance operational boundary without
claiming proof or enforcement that is not yet earned.

### Phase 3.7 — Standard-Format Crate Integration Boundary

#### Purpose

Admit later standard-format crate integration only where an active later slice
truly requires it and the Phase 2.6 dependency-policy baseline is still
honored.

### Exit condition

The minimum runnable alpha exists on the frozen Linux production-alpha lane.

### Not allowed before exit

- calling the alpha hardened or production-proven
- widening platform or artifact scope as a shortcut around missing evidence

## Big Phase 4 — Hardening, Validation, And Proof

### Purpose

Harden, validate, measure, and prove the alpha in real use.

### Phase 4.1 — Observability And Operability

#### Purpose

Make the admitted alpha operable and inspectable in real use without widening
scope into performance proof, failure-mode proof, or deployment proof.

#### Required conditions

- runtime-owned lifecycle and readiness truth are visible while `run` executes
- transport, protocol, and proxy report their own state transitions through
  explicit owned seams
- startup, restart, shutdown, and failure boundaries are inspectable without
  implying broader subsystem completeness
- minimal counters exist for the current alpha path without turning this phase
  into a broad telemetry platform

#### Exit condition

Phase 4.1 is complete when the current alpha can be run, inspected, and
debugged honestly through narrow logs, readiness, and minimal operability
reporting.

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
- Big Phase 2 is closed and frozen
- Big Phase 3 runnable-alpha admission remains intact
- Phase 3.3 QUIC tunnel core is admitted
- Phase 3.4 Pingora proxy seam (3.4a–c) is admitted
- Phase 3.5 wire/protocol boundary is admitted
- Phase 3.6 security/compliance operational boundary is admitted
- Phase 3.7 standard-format crate integration boundary is admitted
- Phase 4.1 observability and operability is admitted
- 4.2 performance proof, 4.3 failure-mode proof, and 4.4 deployment proof are later
