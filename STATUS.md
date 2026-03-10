# Cloudflared Rust Rewrite Status

This file is the short index for current repository state.

Use it as the first status read, then load only the focused status file that
matches the question.

## Current Summary

This repository is a real but partial Rust rewrite workspace.

What exists now:

- accepted first-slice config, credentials, and ingress behavior in
  `crates/cloudflared-config/`
- parity-backed accepted first-slice compare closure for the admitted fixture surface
- a narrow Phase 3.3 QUIC tunnel core in `crates/cloudflared-cli/`
- a Phase 3.4 Pingora proxy seam with runtime lifecycle participation and a
  first admitted origin/proxy path (`http_status` routing) in
  `crates/cloudflared-cli/src/proxy.rs`
- a Phase 3.5 wire/protocol boundary between transport and proxy in
  `crates/cloudflared-cli/src/protocol.rs` with explicit transport-to-proxy
  handoff through the runtime-managed protocol bridge
- frozen Go baseline and design-audit references
- governance and policy docs that freeze the Linux production-alpha lane

What does not exist yet:

- broader Pingora proxy completeness beyond the narrow admitted origin path
- registration RPC content (capnp) and incoming request stream handling
- later security/compliance and standard-format integration slices
- parity-complete broader subsystem coverage
- broader platform scope beyond the frozen Linux lane

## Focused Status Files

- `docs/status/rewrite-foundation.md`
  - baseline, lane, source precedence, workspace shape, and runtime baseline

- `docs/status/active-surface.md`
  - current executable surface and immediate deferred scope

- `docs/status/first-slice-parity.md`
  - first-slice implementation history and parity-backed closure state

- `docs/status/porting-rules.md`
  - first implementation gate, recommended first slice, and done definition

## Routing

- for current repository state: start here, then load the smallest focused
  status file above
- for phase model or promotion boundaries: use `docs/promotion-gates.md`
- for scope and non-negotiables: use `REWRITE_CHARTER.md`
- for behavior and parity truth: use the frozen Go baseline first
