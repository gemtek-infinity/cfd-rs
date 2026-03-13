# Cloudflared Rust Rewrite Status

This file is the short index for current repository state.
Read it first, then load only the focused file that matches the question.

## Current Summary

This repository is a real but partial Rust rewrite of cloudflared.

- **Active lane**: Linux only, `x86_64-unknown-linux-gnu`, quiche + BoringSSL
- **Active phase**: Big Phase 5 — Production-Alpha Completion
- **Compatibility baseline**: frozen Go `2026.2.0` in [baseline-2026.2.0/old-impl/](baseline-2026.2.0/old-impl/)
- **Current workspace version**: `2026.2.0-alpha.202603`

### What exists now

- config types, credentials, ingress, and error taxonomy ([crates/cfdrs-shared/](crates/cfdrs-shared/))
- filesystem config discovery IO ([crates/cfdrs-his/](crates/cfdrs-his/))
- binary entrypoint, runtime composition, lifecycle orchestration ([crates/cfdrs-bin/](crates/cfdrs-bin/))
- CLI command surface: argument parsing, help text, dispatch ([crates/cfdrs-cli/](crates/cfdrs-cli/))
- Cloudflare-facing RPC contracts: registration and stream types ([crates/cfdrs-cdc/](crates/cfdrs-cdc/))
- narrow QUIC tunnel core with Pingora proxy seam, wire/protocol boundary,
  and origin dispatch (in [crates/cfdrs-bin/](crates/cfdrs-bin/))
- host interaction services — filesystem config discovery IO ([crates/cfdrs-his/](crates/cfdrs-his/))
- cross-domain shared types — config, credentials, ingress, error taxonomy,
  discovery types, artifact conversion ([crates/cfdrs-shared/](crates/cfdrs-shared/))
- observability, performance validation, failure-mode proof, and deployment
  proof surfaces for the admitted alpha path
- complete Stage 1 parity audit across three domains (150 rows total):
  - CLI: 32 rows — [docs/parity/cli/implementation-checklist.md](docs/parity/cli/implementation-checklist.md)
  - CDC: 44 rows — [docs/parity/cdc/implementation-checklist.md](docs/parity/cdc/implementation-checklist.md)
  - HIS: 74 rows — [docs/parity/his/implementation-checklist.md](docs/parity/his/implementation-checklist.md)
- 12 feature-group audit documents under [docs/parity/](docs/parity/)
- baseline evidence captures in [docs/parity/cli/captures/](docs/parity/cli/captures/)
- governance, policy, and frozen Go baseline and design-audit references
- target 5-crate workspace structure: `cfdrs-bin`, `cfdrs-cli`, `cfdrs-cdc`,
  `cfdrs-his`, `cfdrs-shared` (Stage 3.2 complete)

### What does not exist yet

- Cap'n Proto registration RPC and full stream round-trip through origin
- broad proxy completeness (WebSocket, TCP streaming, HTTP origin proxying)
- management service, log streaming, Cloudflare REST API client
- Linux service install/uninstall, systemd integration, updater
- local HTTP endpoints (metrics, readiness, diagnostics)
- config reload and file watcher
- broad CLI parity (4 commands vs 9 families, 1 flag vs 50+)

For the full ranked gap inventory, see [docs/status/phase-5-overhaul.md](docs/status/phase-5-overhaul.md).

## Focused Status Files

- [docs/status/rewrite-foundation.md](docs/status/rewrite-foundation.md) — baseline, lane, workspace shape
- [docs/status/active-surface.md](docs/status/active-surface.md) — current crate content and admitted scope
- [docs/status/first-slice-parity.md](docs/status/first-slice-parity.md) — first-slice closure (historical)
- [docs/status/porting-rules.md](docs/status/porting-rules.md) — first-slice porting rules (historical)
- [docs/status/phase-5-overhaul.md](docs/status/phase-5-overhaul.md) — Big Phase 5 execution tracker

## Routing

- parity progress → three implementation checklists under [docs/parity/](docs/parity/)
- overhaul stage tracking → [docs/status/phase-5-overhaul.md](docs/status/phase-5-overhaul.md)
- phase model and promotion → [docs/promotion-gates.md](docs/promotion-gates.md)
- scope and non-negotiables → [REWRITE_CHARTER.md](REWRITE_CHARTER.md)
- behavior and parity truth → frozen Go baseline first
