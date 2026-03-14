# Rewrite Charter

This file is the shortest statement of the rewrite program's non-negotiables.
When any plan, prompt, or local note conflicts with this file, this file wins.

## Objective

Deliver an idiomatic Rust rewrite of `cloudflared` that reaches 1:1 behavior
and contract parity with the frozen Go `2026.2.0` baseline on the declared
Linux production-alpha lane.

Production-alpha is not claimed until the final `Performance Architecture
Overhaul` milestone closes and its post-overhaul evidence reruns cleanly.

## Frozen Baseline

- behavior truth: [`baseline-2026.2.0/old-impl/`](baseline-2026.2.0/old-impl/)
- derived parity routing: [`docs/parity/`](docs/parity/)
- exact row-to-source routing: [`docs/parity/source-map.csv`](docs/parity/source-map.csv)
- target release baseline: `2026.2.0`
- workspace version rule: `-alpha.YYYYmm`
- current workspace version line: `2026.2.0-alpha.202603`

## Active Lane

- Linux only
- target triple: `x86_64-unknown-linux-gnu`
- shipped GNU artifacts: `x86-64-v2`, `x86-64-v4`
- 0-RTT required
- quiche first
- quiche + BoringSSL
- Pingora is in the production-alpha critical path
- FIPS belongs in the production-alpha lane

## Hard Rules

- do not edit [`baseline-2026.2.0/old-impl/`](baseline-2026.2.0/old-impl/) during normal rewrite work
- do not claim parity from Rust code shape alone
- do not widen scope beyond the declared lane without an explicit governance change
- logging compatibility across CLI flags/envs, local sinks, journald/systemd, and upstream Cloudflare services is a production-alpha blocker
- the final Phase 5 milestone is performance-architectural, not feature-expansion work
- dependency admission is gate-first; do not preload speculative crates
- normal human, AI, and CI command entry goes through [`Justfile`](Justfile)
