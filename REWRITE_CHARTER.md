# Rewrite Charter

This file is the shortest possible statement of the rewrite program's non-negotiables.

If any plan, prompt, note, or AI output drifts from this file, this file wins until governance is explicitly changed.

## Objective

Build a production-grade, parity-backed Rust rewrite on the frozen Linux production-alpha lane.

## Compatibility baseline

- behavioral baseline: `baseline-2026.2.0/old-impl/`
- derived reference layer: `baseline-2026.2.0/design-audit/`
- target release baseline: `2026.2.0`
- Rust workspace version rule: `-alpha.YYYYmm`
- current workspace version line: `2026.2.0-alpha.202603`

## Active lane

- Linux only
- target triple: `x86_64-unknown-linux-gnu`
- shipped GNU artifacts:
  - `x86-64-v2`
  - `x86-64-v4`
- 0-RTT is required
- quiche first
- quiche + BoringSSL
- Pingora is in the production-alpha critical path
- FIPS belongs in the production-alpha lane
- Cloudflare-owned crates are preferred where they genuinely fit, but are not mandatory by default

## Source-of-truth routing

Use the right source for the right question:

- behavior and parity:
  1. `baseline-2026.2.0/old-impl/` code and tests
  2. `baseline-2026.2.0/design-audit/`

- non-negotiables and scope:
  - `REWRITE_CHARTER.md`

- current repository state:
  - `STATUS.md`

- dependency and runtime policy:
  - `docs/*.md`

- workflow notes:
  - `AGENTS.md`
  - `SKILLS.md`

Do not smooth conflicts over.
Resolve them explicitly.

## Frozen inputs

The following directories are frozen inputs and must not be modified during normal rewrite work:

- `baseline-2026.2.0/old-impl/`
- `baseline-2026.2.0/design-audit/`

If those inputs appear inconsistent, fix the Rust workspace or the governance docs instead of editing the frozen baseline.

## First accepted implementation slice

The first accepted implementation slice is narrower than broader Phase 1 scope.

It includes:

- config discovery/loading/normalization
- credentials surface
- ingress normalization/ordering/defaulting

It excludes:

- proxying and request forwarding
- transports and protocol selection
- supervisor and reconnect logic
- orchestration and watcher behavior
- metrics, readiness, and management servers
- wire and RPC implementation

## Workspace honesty rule

The current Rust workspace is real but partial.

Manifests, docs, and claims must describe:

- what exists now, or
- the currently accepted next slice

Do not imply completed subsystem ports that do not exist.

## Done means checked

A subsystem is not "done" unless its relevant behavior is checked against the frozen Go baseline.
