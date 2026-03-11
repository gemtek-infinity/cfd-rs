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
  `crates/cloudflared-cli/src/proxy/`
- a Phase 3.5 wire/protocol boundary between transport and proxy in
  `crates/cloudflared-cli/src/protocol.rs` with explicit transport-to-proxy
  handoff through the runtime-managed protocol bridge
- a Phase 3.6 security/compliance operational boundary that reports the bounded
  crypto surface (quiche + BoringSSL lane only) and enforces Linux GNU/glibc
  deployment-contract assumptions at runtime startup
- a Phase 3.7 standard-format crate integration boundary that admits
  workspace-managed PEM handling for the active origin-cert runtime path
  through owned credential adapters in `crates/cloudflared-config/`
- a Phase 4.1 observability and operability surface in
  `crates/cloudflared-cli/` that emits live runtime reporting, owner-scoped
  transport/protocol/proxy state, narrow readiness truth, and minimal
  operability counters for the admitted alpha path
- a Phase 4.2 performance validation surface in `crates/cloudflared-cli/`
  that emits deterministic stage-transition timing evidence, cold vs resumed
  path distinction, 0-RTT lane configuration truth, pipeline latency
  measurement, machine-readable performance evidence with explicit honesty
  scope, and regression thresholds for the admitted alpha harness path
- a Phase 4.3 failure-mode and recovery proof surface in
  `crates/cloudflared-cli/` that emits deterministic failure/recovery
  evidence, bounded reconnect/retry proof, shutdown proof, malformed-input
  boundary handling, dependency-boundary failure visibility, and honest
  config-reload non-support declaration for the admitted alpha harness path
- a Phase 4.4 internal deployment proof surface in `crates/cloudflared-cli/`
  that emits machine-readable deployment evidence, validates the deployment
  contract at runtime startup, declares known deployment gaps and operational
  caveats explicitly, and provides a documented repeatable build-to-run flow
  for the declared Linux production-alpha lane
- a Phase 5.1 broader proxy, wire-format, and stream-serving surface:
  wire-format types in `crates/cloudflared-proto/` (ConnectRequest,
  ConnectionType, Metadata, registration types), origin service dispatch
  beyond http\_status-only (HelloWorld, HTTP-origin dispatch wiring,
  unimplemented-service honest labelling), bounded credentials-file
  registration request/response exchange on the control stream, incoming
  QUIC data stream acceptance with ConnectRequest parsing, and stream-to-proxy
  forwarding through the protocol bridge
- frozen Go baseline and design-audit references
- governance and policy docs that freeze the Linux production-alpha lane

What does not exist yet:

- Cap'n Proto registration RPC parity, origin-cert registration content,
  and full request stream round-trip through origin and back to edge
- broader Pingora proxy completeness beyond the admitted origin-dispatch
  surface (WebSocket upgrade, TCP streaming, actual HTTP origin proxying)
- broader standard-format integration beyond the active origin-cert path and
  broader compliance proof work
- broad admin APIs and broader performance proof beyond the admitted harness
  path
- real systemd unit files, installers, container images, updaters, and
  log-rotation integration beyond the admitted deployment proof surface
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
