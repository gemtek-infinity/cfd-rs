# cfdrs-bin

Binary entrypoint for cloudflared.

## Ownership

This crate owns:

- the `cloudflared` binary entrypoint
- process startup wiring (allocator, signal handling, exit codes)
- top-level runtime composition (Tokio runtime bootstrap)
- lifecycle orchestration between CLI, CDC, and HIS subsystems
- state-machine and supervision composition
- config startup orchestration (discovery invocation, config loading sequence)

This crate does not own:

- CLI command tree semantics, help text, or flag parsing (`cfdrs-cli`)
- Cloudflare-facing RPC, wire, or stream contracts (`cfdrs-cdc`)
- transport or proxy implementation details (`cfdrs-cdc`)
- host-facing service behavior, filesystem layout, or local endpoints (`cfdrs-his`)
- cross-domain shared types or error plumbing (`cfdrs-shared`)
- generic shared utility or dumping-ground behavior

## Governing parity docs

This crate is not a parity domain itself. It composes the three parity
domains and owns the seam between them.

Relevant execution docs:

- `FINAL_PLAN.md` § Target Crate Map
- `FINAL_PHASE.md` § Ownership Definitions → cfdrs-bin
- `docs/status/stage-3.1-scope-triage.md` § Crate Ownership Map

## Baseline surfaces

Process startup, allocator setup, exit code handling, and runtime bootstrap
from the frozen Go baseline's `cmd/cloudflared/main.go` and
`cmd/cloudflared/run.go`.

## Current status

Fully populated. Contains:

- `src/main.rs` — binary entrypoint and execute logic
- `src/runtime/` — lifecycle state machine, supervision, shutdown
- `src/startup/` — config orchestration and startup resolution
- `src/protocol.rs` — wire protocol bridge types (CDC-owned, temporarily here)
- `src/transport/` — QUIC transport lifecycle (CDC-owned, temporarily here)
- `src/proxy/` — Pingora proxy seam (CDC-owned, temporarily here)
- `tests/cli_surface.rs` — CLI integration tests

## Known gaps and next work

- protocol.rs, transport/, proxy/ are temporarily housed here due to tight
  coupling with runtime types (RuntimeService, RuntimeCommand, ChildTask);
  they compose CDC type boundaries and will be extracted to cfdrs-cdc when
  runtime interface types are formalized
