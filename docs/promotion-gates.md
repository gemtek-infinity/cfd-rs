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

### Phase 4.2 — Performance Validation

#### Purpose

Validate the admitted alpha path with deterministic stage-transition timing
evidence, cold vs resumed path distinction, and explicit regression thresholds
without implying full transport or end-to-end performance proof.

#### Required conditions

- transport lifecycle stage transitions are timed relative to runtime start
- cold-start (attempt 0) vs resumed (attempt > 0) paths are distinguished in
  evidence output
- machine-readable performance evidence is emitted at runtime finish
- explicit regression thresholds exist for the admitted harness path
- threshold violations are reported as a pass/fail gate in summary output
- evidence is honest about what is measured (in-process harness timing) vs
  what remains deferred (real QUIC wire latency, 0-RTT session resumption
  savings, end-to-end request latency)

#### Exit condition

Phase 4.2 is complete when the admitted alpha path emits deterministic
performance evidence with regression thresholds that can gate CI and detect
stage-transition regressions.

### Phase 4.3 — Failure-Mode And Recovery Proof

#### Purpose

Prove the admitted alpha surface behaves sanely under disruption without
implying broader transport, deployment, or recovery completeness.

#### Required conditions

- reconnect/retry behavior is bounded and visible in evidence output
- shutdown behavior is observable through lifecycle state transitions and
  child-task drain reporting
- dependency-boundary failures are reported with explicit owner and class at
  each failure event
- config-reload is honestly declared as not supported
- malformed YAML, invalid ingress rules, and structurally invalid config
  fields fail at the config boundary with typed error categories
- machine-readable failure evidence is emitted at runtime finish
- evidence is honest about what is proven (in-process harness failure proof)
  vs what remains deferred (real transport reconnect, deployment-level
  recovery, config-reload behavior)

#### Exit condition

Phase 4.3 is complete when the admitted alpha path emits deterministic
failure-mode and recovery evidence with honest scope boundaries.

### Phase 4.4 — Internal Deployment Proof

#### Purpose

Demonstrate that the admitted alpha surface is believable in real operational
use without overstating unsupported behavior.

#### Required conditions

- the deployment contract is validated at runtime startup and the result is
  visible in evidence output
- machine-readable deployment evidence is emitted at runtime finish
- build-to-run flow is repeatable and documented for the declared lane
- known deployment gaps are declared explicitly (no systemd unit, no
  installer, no container image, no updater, no log rotation)
- operational caveats are declared explicitly (alpha-only, narrow origin path,
  no RPC registration, no incoming streams, no config reload)
- evidence scope is honestly bounded to in-process contract validation
- deployment notes exist and match the declared deployment contract
- the CI merge workflow produces lane-specific preview artifacts

#### Exit condition

Phase 4.4 is complete when a reviewer can follow a repeatable build-to-run
flow for the declared lane, deployment notes match the actual contract,
operational caveats are explicit, and known gaps are stated honestly.

### Exit condition

The promoted alpha scope is validated well enough to be credible in real use.

## Big Phase 5 — Production-Alpha Completion And Frozen-Baseline Proof

### Purpose

Complete the remaining frozen-baseline-required feature/surface on the declared
Linux lane and prove production alpha.

Production alpha means:

- feature-complete, 1:1 in behavior/surface to frozen `2026.2.0` on the
  declared lane
- performance proven on that declared lane
- not every edge case necessarily covered yet
- implemented as idiomatic Rust with explicit ownership boundaries, not as
  structural cloning

Big Phase 5 is the phase that completes and proves production alpha.
There is no separate post-alpha validation phase inside this roadmap.

### Scope includes, where required for frozen-lane parity

- remaining feature/surface completion to frozen `2026.2.0`
- broader proxy completeness beyond the first admitted origin path
- registration RPC content and incoming request-stream handling
- broader standard-format and compliance surfaces beyond the already admitted
  active path
- remaining CLI/process/runtime/lifecycle surfaces required for the declared
  lane
- broader performance proof beyond the earlier narrow admitted harness path
- failure/recovery/operability proof across the feature-complete surface
- behavior/contract parity validation and divergence accounting
- production-alpha promotion gate
- admin/control surfaces only where they are actually part of frozen-lane
  parity

### Exit condition

Big Phase 5 is done when the repo can honestly claim production-grade alpha
for the declared Linux lane: feature-complete 1:1 behavior/surface parity to
frozen `2026.2.0`, performance proven, known divergences recorded and
justified, and remaining unknowns narrow, named, and bounded.

### Not allowed before exit

- claiming production alpha without durable behavior/contract parity evidence
- leaving divergences unrecorded or unjustified
- treating structural cloning as equivalent to behavior parity

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
- Phase 4.2 performance validation is admitted
- Phase 4.3 failure-mode and recovery proof is admitted
- Phase 4.4 deployment proof is admitted
- Big Phase 5 completes remaining frozen-baseline feature/surface and proves
  production alpha
