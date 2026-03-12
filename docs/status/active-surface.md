# Active Surface Status

This file describes the currently implemented surface and what remains absent.

For the complete gap inventory with ranks and evidence status, see the parity
ledgers:

- `docs/parity/cli/implementation-checklist.md` (32 rows)
- `docs/parity/cdc/implementation-checklist.md` (44 rows)
- `docs/parity/his/implementation-checklist.md` (74 rows)

For the cross-domain gap ranking and implementation order, see
`docs/status/phase-5-overhaul.md`.

## Current Crate Content

### cloudflared-cli

The runtime and transport owner. Contains:

- process entrypoint with config-backed startup validation
- runtime lifecycle shell with supervision, shutdown, and restart boundaries
- QUIC transport core (quiche + BoringSSL) with session management
- Pingora proxy seam with origin dispatch (http_status, hello_world routing;
  broader origin types return 502 honestly)
- protocol bridge between transport and proxy
- control stream with bounded credentials-file registration exchange
- incoming QUIC data stream acceptance and ConnectRequest parsing
- observability, performance, failure-mode, and deployment evidence surfaces
- security/compliance operational boundary (crypto surface reporting,
  deployment-contract validation)

### cloudflared-config

Config and credentials owner. Contains:

- config discovery search order and file loading
- credentials file and origin-cert decoding (with PEM handling via owned
  credential adapters)
- ingress rule normalization, ordering, and matching
- first-slice parity harness and Go-truth compare fixtures

### cloudflared-proto

Wire-format types. Contains:

- ConnectRequest, ConnectResponse, ConnectionType, Metadata
- registration RPC type boundaries (TunnelAuth, ConnectionOptions,
  ConnectionDetails)

### cloudflared-core

Minimal. Reserved for future shared types.

## Major Absent Surfaces

These are the highest-impact gaps. For the complete ranked list, see
`docs/status/phase-5-overhaul.md` § Cross-Domain Gap Ranking.

- **Cap'n Proto registration RPC** — current registration uses JSON; Go
  baseline uses Cap'n Proto binary encoding. This is the single highest-risk
  gap.
- **Stream framing and full round-trip** — incoming streams are accepted and
  parsed but not round-tripped through origin and back to edge.
- **Management service** — routes, auth middleware, log streaming WebSocket
  are entirely absent.
- **Cloudflare REST API client** — tunnel CRUD, API response envelope are
  entirely absent.
- **Broad CLI surface** — 4 commands vs 9 families, 1 flag vs 50+.
- **Linux service management** — service install, uninstall, systemd template
  are entirely absent.
- **Local HTTP endpoints** — metrics server, readiness endpoint, Prometheus
  metrics are absent.
- **Config reload and file watcher** — absent, explicitly declared.
- **Diagnostics collection** — absent.
- **Auto-update mechanism** — absent.

## What the current surface does not imply

- Cap'n Proto registration RPC parity is implemented
- incoming streams are round-tripped through origin
- Pingora proxy seam is general proxy completeness
- compliance boundary constitutes certification or FIPS validation
- performance evidence includes real wire latency
- deployment evidence includes real systemd or packaging integration
