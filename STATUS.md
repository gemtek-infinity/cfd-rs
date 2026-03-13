# Cloudflared Rust Rewrite Status

## Active Snapshot

- lane: Linux only, `x86_64-unknown-linux-gnu`, quiche + BoringSSL, 0-RTT required
- compatibility baseline: frozen Go `2026.2.0` in `baseline-2026.2.0/old-impl/`
- workspace version: `2026.2.0-alpha.202603`
- roadmap state: `Program Reset` complete; active implementation milestone: `CDC Contract Foundation`
- highest-risk blockers: `CDC-001`, `CDC-002`, `CDC-011`, `CDC-012`, `CDC-018`, `CLI-001`, `CLI-002`, `CLI-003`, `HIS-012` through `HIS-017`, `HIS-024`, `HIS-025`, `HIS-041`, `HIS-042`
- status rule: this file is the only tracked status source for both humans and AI

## Current Reality

This repository is a real but partial Rust rewrite of `cloudflared`.

What exists now:

- `cfdrs-bin`: binary entrypoint, runtime composition, QUIC tunnel shell, Pingora seam, deployment/performance/failure evidence
- `cfdrs-cli`: current alpha CLI parsing, help, dispatch, and CLI-facing error/output types
- `cfdrs-cdc`: registration and stream contract types
- `cfdrs-his`: filesystem config discovery IO
- `cfdrs-shared`: config, credentials, ingress, discovery constants, error taxonomy, artifact conversion
- live parity ledgers across CLI, CDC, and HIS in `docs/parity/`
- frozen baseline and design-audit references under `baseline-2026.2.0/`

What does not exist yet:

- Cap'n Proto registration RPC and full stream round-trip through origin
- management service, log streaming, Cloudflare REST API client
- broad CLI parity: root invocation, help surface, global flags, tunnel/access/tail/service/update parity
- Linux service install/uninstall and systemd integration
- local HTTP endpoints: metrics, readiness, diagnostics
- config reload and file watcher
- performance-architectural overhaul of the final admitted hot paths

## Active Milestone

### CDC Contract Foundation

Current objective:

- replace JSON/custom wire shortcuts with baseline-backed CDC contracts
- close the lane-blocking registration and stream gaps first
- keep CLI and HIS work unblocked only where CDC dependencies are already explicit

Current milestone exit requires:

- registration schema and wire encoding closure for `CDC-001` through `CDC-006`
- stream framing and round-trip closure for `CDC-011` through `CDC-018`
- baseline-backed CDC ownership in `cfdrs-cdc` rather than runtime-local shortcuts
- matching roadmap and ledger evidence for every closed CDC row

Next milestone after CDC closure:

- `Host and Runtime Foundation`

## Priority Rows

Tier 1 lane-blocking rows, in implementation order:

1. `CDC-001`, `CDC-002` — registration schema and wire encoding
2. `CDC-011`, `CDC-012`, `CDC-018` — stream schema, framing, and round-trip
3. `CLI-001`, `CLI-002`, `CLI-003` — root invocation, help text, global flags
4. `CLI-008`, `CLI-010`, `CLI-012` — tunnel root behavior, create, run
5. `HIS-012` through `HIS-017`, `HIS-022` — service install/uninstall and systemd templates
6. `HIS-024`, `HIS-025`, `HIS-027` — local metrics, readiness, and Prometheus exposure
7. `HIS-041`, `HIS-042`, `HIS-044` — file watcher, reload loop, remote config update
8. `CDC-023`, `CDC-024`, `CDC-026` — management routes, auth, and log streaming
9. `CDC-033`, `CDC-034` — Cloudflare REST API client and response envelope
10. final milestone: `Performance Architecture Overhaul` after proof closure reruns cleanly

## Architecture Contract

Allowed crate dependency direction:

- `cfdrs-bin -> cfdrs-cli, cfdrs-cdc, cfdrs-his, cfdrs-shared`
- `cfdrs-cli -> cfdrs-shared`
- `cfdrs-cdc -> cfdrs-shared`
- `cfdrs-his -> cfdrs-shared`
- `cfdrs-shared` must not depend on domain crates
- CLI, CDC, and HIS must not depend on each other directly

Ownership rules:

- CLI parity work lands in `cfdrs-cli`
- Cloudflare contract work lands in `cfdrs-cdc`
- host/runtime interaction work lands in `cfdrs-his`
- shared types stay in `cfdrs-shared` only when more than one top-level domain needs them
- performance work must preserve these boundaries; it may optimize seams but must not collapse the workspace into a convenience monolith

## Canonical Links

- scope and non-negotiables: `REWRITE_CHARTER.md`
- roadmap: `docs/phase-5/roadmap.md`
- roadmap row map: `docs/phase-5/roadmap-index.csv`
- parity index: `docs/parity/README.md`
- CLI ledger: `docs/parity/cli/implementation-checklist.md`
- CDC ledger: `docs/parity/cdc/implementation-checklist.md`
- HIS ledger: `docs/parity/his/implementation-checklist.md`
- phase model and promotion rules: `docs/promotion-gates.md`
- AI routing: `docs/ai-context-routing.md`
