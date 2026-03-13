# Active Surface Status

This file describes the currently implemented surface and what remains absent.

For the complete gap inventory with ranks and evidence status, see the parity
ledgers:

- [docs/parity/cli/implementation-checklist.md](../parity/cli/implementation-checklist.md) (32 rows)
- [docs/parity/cdc/implementation-checklist.md](../parity/cdc/implementation-checklist.md) (44 rows)
- [docs/parity/his/implementation-checklist.md](../parity/his/implementation-checklist.md) (74 rows)

For the cross-domain gap ranking and implementation order, see
[docs/status/phase-5-overhaul.md](phase-5-overhaul.md).

## Current Crate Content

### cfdrs-bin

The binary entrypoint and runtime composition owner. Contains:

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

### cfdrs-cli

CLI command surface owner. Contains:

- argument parsing and dispatch (parse_args, Cli, Command)
- help text rendering (render_help)
- CLI output formatting (CliOutput)
- CLI error taxonomy (CliError)

### cfdrs-cdc

Cloudflare-facing RPC contracts owner. Contains:

- ConnectRequest, ConnectResponse, ConnectionType, Metadata
- registration RPC type boundaries (TunnelAuth, ConnectionOptions,
  ConnectionDetails)

### cfdrs-his

Host interaction services owner. Contains:

- filesystem config discovery IO (`find_default_config_path`,
  `find_or_create_config_path`, `discover_config`)

New HIS code lands directly here.

### cfdrs-shared

Config types, credentials, ingress, and error taxonomy owner. Contains:

- config types, raw and normalized config loading
- credentials file and origin-cert decoding (with PEM handling via owned
  credential adapters)
- ingress rule normalization, ordering, and matching
- discovery types and constants
- error types (ConfigError, ErrorCategory)
- first-slice parity harness and Go-truth compare fixtures

### Retired crates

- **cloudflared-cli** — removed from workspace; code moved to cfdrs-bin and
  cfdrs-cli
- **cloudflared-proto** — removed from workspace; code moved to cfdrs-cdc
- **cloudflared-core** — removed from workspace; was empty skeleton
- **cloudflared-config** — dissolved; shared config types to cfdrs-shared,
  filesystem discovery IO to cfdrs-his

## Major Absent Surfaces

These are the highest-impact gaps. For the complete ranked list, see
[docs/status/phase-5-overhaul.md](phase-5-overhaul.md) § Cross-Domain Gap Ranking.

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
