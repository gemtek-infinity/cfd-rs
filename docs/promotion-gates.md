# Promotion Gates

This document defines the rewrite phases, their exit conditions, and the rules
for promotion from one phase to the next.

A later phase must not begin in substance until the earlier phase's gate is
satisfied or explicitly waived.

## Purpose

The rewrite should progress by evidence, not by excitement.

This file exists so that humans and AI assistants cannot silently skip from
planning to broad implementation without closing the required gates.

## Phase 0 — Baseline Lock And Doctrine Freeze

### Purpose

Freeze the rewrite's governing doctrine so the program stops renegotiating its
foundations.

### Required conditions

- compatibility baseline is explicitly frozen
- source precedence is explicit
- frozen inputs are explicit
- target platform is explicit
- active and deferred scope are explicit
- current workspace honesty rule is explicit
- first accepted implementation slice is explicit
- dependency posture is explicit
- runtime doctrine is explicit
- "done means parity" is explicit

### Evidence

- `STATUS.md`
- `docs/compatibility-scope.md`
- `docs/go-rust-semantic-mapping.md`
- `docs/dependency-policy.md`
- `docs/allocator-runtime-baseline.md`
- `docs/adr/0001-hybrid-concurrency-model.md`
- `REWRITE_CHARTER.md`
- `docs/first-slice-freeze.md`

### Exit condition

Phase 0 is complete when the rewrite doctrine is frozen in governance docs and
the anti-drift lock layer exists.

### Not allowed before exit

- broad subsystem porting
- dependency expansion based on future intent
- transport/runtime-heavy implementation
- ungoverned crate proliferation

## Phase 1 — Parity Harness Activation

### Purpose

Make the rewrite capable of producing executable parity evidence for the first
slice.

### Required conditions

- first-slice fixture inventory is accepted
- executable parity harness runner exists
- Go truth outputs for first-slice cases are captured
- Rust-side comparison path exists
- first-slice parity cases can pass or fail mechanically
- the FIPS/compliance boundary for current work is explicitly recorded

### Primary owner area

- `crates/cloudflared-config/tests/`

### Exit condition

Phase 1 is complete when first-slice parity can be evaluated mechanically
against captured Go truth.

### Not allowed before exit

- claiming the rewrite process is parity-backed
- broadening into runtime-heavy subsystem work
- using subjective "looks compatible" judgments as a substitute for harness
  evidence

## Phase 2 — First Slice Implementation

### Purpose

Implement the first accepted slice and prove parity for it.

### Required conditions

- config discovery behavior implemented
- config parsing behavior implemented
- credential surface behavior for the slice implemented
- ingress parsing/validation implemented
- ingress normalization implemented
- deterministic matching implemented
- thin CLI-origin synthesis implemented where required
- all accepted first-slice parity tests passing

### Primary owner area

- `crates/cloudflared-config/`
- `crates/cloudflared-config/tests/`

### Exit condition

Phase 2 is complete when the accepted first slice is parity-backed and passing.

### Not allowed before exit

- calling cloudflared "ported"
- widening into transport/runtime orchestration as if external input behavior
  were already frozen
- admitting future-slice dependencies without active owning need

## Phase 3 — Foundation Layer Expansion

### Purpose

Freeze the broader externally visible foundation above the first slice.

### Expected areas

- CLI framework
- logging
- metrics server
- readiness behavior
- validation and TLS-related behavior

### Exit condition

The foundation layer is parity-backed on Linux for the promoted scope.

### Not allowed before exit

- hand-waving over CLI/env/default behavior
- claiming operational readiness without metrics/readiness parity where promoted

## Phase 4 — Network And Protocol Parity

### Purpose

Port wire-visible transports and protocol machinery.

### Expected areas

- edge discovery
- QUIC transport
- HTTP/2 transport
- datagram V2
- datagram V3
- Cap'n Proto RPC
- connection abstraction

### Exit condition

Wire-visible behavior and byte-critical contracts are parity-backed for the
promoted protocol scope.

### Not allowed before exit

- claiming protocol compatibility without byte-level evidence
- hiding wire differences behind internal architecture changes

## Phase 5 — Runtime Core Parity

### Purpose

Port the runtime control plane and data-plane behavior that makes the daemon
operationally real.

### Expected areas

- ingress runtime behavior
- proxy runtime behavior
- orchestrator
- supervisor
- datagram session lifecycle
- flow control
- management service

### Exit condition

Linux runtime behavior is parity-backed for promoted request, reconnect,
shutdown, and management semantics.

### Not allowed before exit

- calling the daemon operationally equivalent
- relying on speculative runtime topology without parity evidence

## Phase 6 — Auxiliary Linux Completeness

### Purpose

Complete Linux-relevant auxiliary behavior needed for a credible production
story.

### Expected areas

- SOCKS proxy
- IP access rules
- carrier
- access commands
- tail command
- Linux service install
- hello server
- tracing, when promoted by owning slices

### Exit condition

The Linux-targeted port is functionally whole enough for the intended deployment
and review story.

## Phase 7 — Production Hardening

### Purpose

Prove that the port is not only parity-oriented but also operationally viable
for the intended Linux deployment story.

### Required conditions

- soak/load/failure testing completed for promoted scope
- operational runbooks exist
- compatibility gaps and waivers are documented
- security-sensitive surfaces reviewed
- deployment evidence exists for the promoted scope

### Exit condition

The promoted Linux scope is trusted for real use with explicit known gaps and
waivers.

## Phase 8 — Review Package

### Purpose

Turn the technical result into a credible, reviewable engineering artifact.

### Required conditions

- parity report exists
- architecture report exists
- production evidence report exists
- explicit compatibility matrix exists
- demo path exists for the promoted scope
- known waivers and deferred items are stated clearly

### Exit condition

A serious technical reviewer can see that the project is parity-governed,
production-shaped, and externally reviewable.

## Promotion Rule

A phase may be promoted only when:

1. its exit condition is met, or
2. an explicit written waiver is added in governance docs explaining why the
   promotion is acceptable despite incomplete evidence

Silently behaving as though a phase is complete is not allowed.

## Current Phase Reading

At the current repo state:

- Phase 0 is effectively frozen in doctrine
- the anti-drift lock layer is what closes it cleanly
- Phase 1 is the next active execution phase
- Phase 2 begins only after the parity harness is truly operational
