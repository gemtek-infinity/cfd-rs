# Rewrite Charter

This file is the shortest possible statement of the rewrite program's
non-negotiables.

If any other planning note, prompt, AI output, or human interpretation drifts
from this file, this file wins unless a governance update explicitly changes it.

## Objective

The objective of this repository is to produce a Rust rewrite of cloudflared
that is production-grade, parity-backed, and credible for internal deployment
and serious external technical review.

This objective is subordinate to behavioral correctness. A rewrite that looks
impressive but is not parity-backed does not satisfy the objective.

## Compatibility Baseline

- Behavioral baseline: `baseline-2026.2.0/old-impl/`
- Derived reference layer: `baseline-2026.2.0/design-audit/`
- Target release baseline: `2026.2.0`
- Rust workspace version rule: `-alpha.YYYYmm`
- Current workspace version line: `2026.2.0-alpha.202603`

The Rust rewrite is coupled to the Go compatibility baseline. It is not an
independent product line.

## Source Precedence

If sources disagree, use this order:

1. `baseline-2026.2.0/old-impl/` code and tests
2. `baseline-2026.2.0/design-audit/`
3. `AGENTS.md`
4. `SKILLS.md`

Do not smooth over conflicts. Resolve them explicitly.

## Frozen Inputs

The following directories are frozen inputs and must not be modified during
normal rewrite work:

- `baseline-2026.2.0/old-impl/`
- `baseline-2026.2.0/design-audit/`

If those inputs appear inconsistent, fix the Rust workspace or the governance
documents instead of editing the frozen baseline.

## Primary Platform

The primary target platform is:

- `x86_64-unknown-linux-gnu`

Linux compatibility is the active target. Other platforms are deferred unless
explicitly promoted.

## Active Compatibility Scope

Phase 1 compatibility includes externally visible behavior for:

- command names, flags, env bindings, defaults, and empty-invocation behavior
- config discovery, parsing, validation, and credential handling
- wire formats and protocol constants
- transport behavior that is externally visible
- ingress, routing, and proxy behavior
- metrics, readiness, and management behavior
- graceful shutdown and reconnect behavior
- Linux-visible runtime semantics

## Deferred Scope

The following are deferred unless explicitly promoted:

- macOS parity
- Windows parity
- packaging and installer parity
- release automation parity
- updater workflow parity
- distro-specific packaging parity
- FIPS artifact and compliance parity

## First Accepted Implementation Slice

The first accepted implementation slice is narrower than all of Phase 1.

It includes:

- config discovery, parsing, validation, and credential handling
- ingress parsing, validation, normalization, and deterministic matching
- CLI-origin synthesis only to the extent required to normalize single-origin
  ingress inputs

It excludes:

- proxying and request forwarding
- transports and protocol selection
- supervisor and reconnect logic
- orchestration and watcher behavior
- metrics, readiness, and management servers
- wire and RPC implementation

## Current Workspace Honesty Rule

The current Rust workspace is a scaffold.

Therefore:

- manifests may prepare accepted work, but must not imply completed subsystem
  ports that do not exist
- placeholder crates must remain clearly placeholder crates
- no dependency should be admitted solely because it will "probably be needed
  later"

## Dependency Admission Rule

Dependencies may be added only when all of the following are true:

1. the owning slice has started
2. the owning crate is clear
3. the dependency is needed by code that exists now
4. the dependency is consistent with `docs/dependency-policy.md`

Current baseline rule:

- `mimalloc` belongs only at the runnable binary boundary
- allocator settings must remain governed by
  `docs/allocator-runtime-baseline.md`

## Runtime Doctrine

The accepted runtime doctrine is:

- actor-inspired control plane
- Tokio structured-async data plane
- bounded queues only
- explicit ownership and cancellation

However, the accepted first slice should remain primarily synchronous and
deterministic. Async/runtime machinery should not be introduced merely to mimic
future daemon structure.

## Done Means Parity

A subsystem must not be called "ported" unless:

- behavior matches `baseline-2026.2.0/old-impl/`
- relevant CLI/config surface matches `baseline-2026.2.0/old-impl/`
- relevant wire bytes match `baseline-2026.2.0/old-impl/` where applicable
- documented quirks are either preserved or explicitly waived
- parity tests are documented and passing

## Anti-Drift Rule

No plan, patch, prompt, AI output, or implementation note may silently widen
scope, weaken the baseline, or bypass parity requirements.

If a proposed change touches scope, baseline, phase order, dependency posture,
or platform priority, it must update the appropriate governance document first.
