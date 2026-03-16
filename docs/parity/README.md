# Parity Tracking

This directory holds the live parity ledgers, feature-group documents, and
source-routing data for the rewrite.

Parity is tracked against the frozen Go baseline `2026.2.0` across three
domains.

## Domains

### CLI

User-visible command surface: command tree, help text, flags, environment
bindings, exit codes, compatibility paths, and output formatting.

- [`cli/implementation-checklist.md`](cli/implementation-checklist.md)
- [`cli/root-and-global-flags.md`](cli/root-and-global-flags.md)
- [`cli/tunnel-subtree.md`](cli/tunnel-subtree.md)
- [`cli/access-subtree.md`](cli/access-subtree.md)
- [`cli/tail-and-management.md`](cli/tail-and-management.md)
- [`cli/captures/`](cli/captures/)

### CDC

Cloudflare-facing contracts: registration RPC, stream framing, management,
log streaming, readiness, metrics, and REST API boundaries.

- [`cdc/implementation-checklist.md`](cdc/implementation-checklist.md)
- [`cdc/registration-rpc.md`](cdc/registration-rpc.md)
- [`cdc/stream-contracts.md`](cdc/stream-contracts.md)
- [`cdc/management-and-diagnostics.md`](cdc/management-and-diagnostics.md)
- [`cdc/metrics-readiness-and-api.md`](cdc/metrics-readiness-and-api.md)

### HIS

Host interaction services: filesystem effects, service install/uninstall,
watcher/reload, diagnostics, local endpoints, signals, and deployment-facing
behavior.

- [`his/implementation-checklist.md`](his/implementation-checklist.md)
- [`his/service-installation.md`](his/service-installation.md)
- [`his/filesystem-and-layout.md`](his/filesystem-and-layout.md)
- [`his/diagnostics-and-collection.md`](his/diagnostics-and-collection.md)
- [`his/reload-and-watcher.md`](his/reload-and-watcher.md)

### Cross-domain contracts

- [`source-map.csv`](source-map.csv) — exact row-to-baseline routing
- [`logging-compatibility.md`](logging-compatibility.md) — logging, journald/systemd, and upstream log-streaming contract

## Cross-Domain Summary

| Domain | Rows | Critical | High |
| --- | --- | --- | --- |
| CLI | 32 | 9 | 13 |
| CDC | 44 | 10 | 18 |
| HIS | 74 | 13 | 31 |
| Total | 150 | 32 | 62 |

Use [`STATUS.md`](../../STATUS.md) for the current priority queue and [`docs/phase-5/roadmap.md`](../phase-5/roadmap.md)
plus [`docs/phase-5/roadmap-index.csv`](../phase-5/roadmap-index.csv) for implementation order.
Parity tickets are the row IDs; prefer MCP `next_parity_ticket` and
`parity_row_details` before opening full ledgers.

## Source Of Truth

- behavior truth: [`baseline-2026.2.0/`](../../baseline-2026.2.0/)
- derived parity routing: [`docs/parity/source-map.csv`](source-map.csv)
- status truth: [`STATUS.md`](../../STATUS.md)
- roadmap truth: [`docs/phase-5/roadmap.md`](../phase-5/roadmap.md)

Parity claims must be evidence-based. Structure alone is not parity.
