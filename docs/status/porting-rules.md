# Porting Rules Status

> **Historical record — partially superseded.**
> The first implementation gate described below has been satisfied.
> The recommended first slice is complete and parity-backed.
> The "Done Means" criteria remain valid as a general porting standard.
>
> Broader subsystem porting is now governed by the final-phase program:
>
> - [FINAL_PHASE.md](../../FINAL_PHASE.md) — detailed execution reference
> - [FINAL_PLAN.md](../../FINAL_PLAN.md) — staged execution plan
> - [docs/parity/cli/implementation-checklist.md](../parity/cli/implementation-checklist.md)
> - [docs/parity/cdc/implementation-checklist.md](../parity/cdc/implementation-checklist.md)
> - [docs/parity/his/implementation-checklist.md](../parity/his/implementation-checklist.md)

This file captures the current implementation gate, the recommended first slice,
and the repository's definition of done.

## First Implementation Gate

No large-scale subsystem implementation should begin until all of the following
are true:

1. [docs/compatibility-scope.md](../compatibility-scope.md) is accepted
2. [docs/go-rust-semantic-mapping.md](../go-rust-semantic-mapping.md) is accepted
3. [docs/dependency-policy.md](../dependency-policy.md) is accepted
4. [docs/allocator-runtime-baseline.md](../allocator-runtime-baseline.md) is accepted
5. [docs/adr/0001-hybrid-concurrency-model.md](../adr/0001-hybrid-concurrency-model.md) is accepted
6. the workspace skeleton is accepted
7. the first subsystem slice and its parity checks are frozen

## Recommended First Slice

Port config, credentials, and ingress normalization first.

Reason:

- the behavior is heavily documented
- the inputs and outputs are comparatively deterministic
- the slice is lower risk than transports, supervisors, or streaming bridges
- it freezes CLI/config/env precedence and credential parsing early
- it freezes ingress validation and default no-ingress behavior early
- it unlocks later runtime assembly work without starting in the highest-risk
  concurrency areas

Scope boundary for the first slice:

- include config discovery, parsing, validation, and credential handling
- include ingress parsing, validation, normalization, and deterministic rule
  matching
- include CLI-origin synthesis only to the extent needed to normalize
  single-origin ingress inputs
- exclude proxying, transports, supervisor logic, metrics servers, management,
  and orchestration

Current scaffold implication:

- narrow first-slice implementation code is now present
- the crate layout still reserves the correct boundaries for the remainder of
  this first slice
- manifests should stay sparse while the remaining slice behavior lands

## Done Means

A subsystem should not be called "ported" unless:

- behavior matches [baseline-2026.2.0/old-impl/](../../baseline-2026.2.0/old-impl/)
- relevant config or CLI surface matches [baseline-2026.2.0/old-impl/](../../baseline-2026.2.0/old-impl/)
- relevant wire bytes match [baseline-2026.2.0/old-impl/](../../baseline-2026.2.0/old-impl/) where applicable
- documented quirks are either preserved or explicitly waived
- parity tests are documented and passing
