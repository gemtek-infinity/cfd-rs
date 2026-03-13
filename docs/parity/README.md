# Parity Tracking

This directory holds the live parity audit and tracking documents for the
Rust rewrite.

Parity is tracked across three domains against the frozen Go baseline
`2026.2.0`.

## Domains

### CLI — command surface

The blackbox user and operator command surface: command tree, help text, flags,
env bindings, exit codes, hidden and compatibility commands, formatting.

- [cli/implementation-checklist.md](cli/implementation-checklist.md) — master ledger (32 rows)
- [cli/root-and-global-flags.md](cli/root-and-global-flags.md) — root invocation, global flags, env bindings
- [cli/tunnel-subtree.md](cli/tunnel-subtree.md) — tunnel command family
- [cli/access-subtree.md](cli/access-subtree.md) — access command family and forward alias
- [cli/tail-and-management.md](cli/tail-and-management.md) — tail, management, service, update commands
- [cli/captures/](cli/captures/) — baseline evidence captures from frozen Go binary

### CDC — Cloudflare contracts

Interactions with Cloudflare-managed services: registration RPC, stream
contracts, management service, log streaming, metrics, readiness, Cloudflare
REST API.

- [cdc/implementation-checklist.md](cdc/implementation-checklist.md) — master ledger (44 rows)
- [cdc/registration-rpc.md](cdc/registration-rpc.md) — registration schema, wire encoding, RPC methods
- [cdc/stream-contracts.md](cdc/stream-contracts.md) — ConnectRequest/Response, framing, round-trip
- [cdc/management-and-diagnostics.md](cdc/management-and-diagnostics.md) — management HTTP service, log streaming
- [cdc/metrics-readiness-and-api.md](cdc/metrics-readiness-and-api.md) — metrics, readiness, Cloudflare API client

### HIS — host interactions

Interactions with the local host: filesystem, config discovery, service
install/uninstall, systemd, watcher/reload, diagnostics, local endpoints,
privilege and environment assumptions.

- [his/implementation-checklist.md](his/implementation-checklist.md) — master ledger (74 rows)
- [his/service-installation.md](his/service-installation.md) — service install, uninstall, systemd templates
- [his/filesystem-and-layout.md](his/filesystem-and-layout.md) — paths, file creation, permissions
- [his/diagnostics-and-collection.md](his/diagnostics-and-collection.md) — diagnostic collectors and output
- [his/reload-and-watcher.md](his/reload-and-watcher.md) — config reload, file watcher, SIGHUP

## Cross-domain summary

| Domain | Rows | Critical | High | Medium | Low |
| --- | --- | --- | --- | --- | --- |
| CLI | 32 | 9 | 13 | 10 | 0 |
| CDC | 44 | 10 | 18 | 15 | 1 |
| HIS | 74 | 13 | 31 | 25 | 5 |
| **Total** | **150** | **32** | **62** | **50** | **6** |

For the cross-domain gap ranking and implementation order, see
[docs/status/phase-5-overhaul.md](../status/phase-5-overhaul.md).

## Source of truth

The frozen Go baseline under [baseline-2026.2.0/old-impl/](../../baseline-2026.2.0/old-impl/) is the primary
behavior reference. The design-audit documents under
[baseline-2026.2.0/design-audit/](../../baseline-2026.2.0/design-audit/) are the secondary reference.

Parity claims must be evidence-based. Structure alone is not parity.
