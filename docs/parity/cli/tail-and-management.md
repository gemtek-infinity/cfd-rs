# Tail And Management — CLI Parity Audit

This document inventories the `tail` and `management` command families from the
frozen Go baseline and records current Rust coverage.

Authoritative checklist rows: CLI-023, CLI-024.

## Tail command (CLI-023)

Source: [baseline-2026.2.0/cmd/cloudflared/tail/cmd.go](../../../baseline-2026.2.0/cmd/cloudflared/tail/cmd.go)

Streams remote logs from a running tunnel connector.

Usage: `tail [TUNNEL-ID]`

Category: `Tunnel`

### Flags

| Flag | Aliases | Type | Default | Env var | Hidden | Usage |
| --- | --- | --- | --- | --- | --- | --- |
| `--connector-id` | | string | | `TUNNEL_MANAGEMENT_CONNECTOR` | no | access a specific connector by ID |
| `--event` | | string slice | | `TUNNEL_MANAGEMENT_FILTER_EVENTS` | no | filter by events (cloudflared, http, tcp, udp) |
| `--level` | | string | `debug` | `TUNNEL_MANAGEMENT_FILTER_LEVEL` | no | filter by log level (debug, info, warn, error) |
| `--sample` | | float64 | 1.0 | `TUNNEL_MANAGEMENT_FILTER_SAMPLE` | no | sample events by percentage (0.0 to 1.0) |
| `--token` | | string | | `TUNNEL_MANAGEMENT_TOKEN` | no | access token for tunnel |
| `--management-hostname` | | string | `management.argotunnel.com` | `TUNNEL_MANAGEMENT_HOSTNAME` | yes | management hostname |
| `--trace` | | string | | | yes | set cf-trace-id for request |
| `--loglevel` | | string | `info` | `TUNNEL_LOGLEVEL` | no | application logging level |
| `--origincert` | | string | `FindDefaultOriginCertPath()` | `TUNNEL_ORIGIN_CERT` | no | origin certificate path |
| `--output` | | string | | | no | log output format |

### Hidden subcommand: `tail token`

Gets management JWT for a tunnel. Inherits parent `tail` flags, no additional
flags.

Rust coverage: `tail` and hidden `tail token` are parity-backed. Bare `tail`
opens the management WebSocket and streams log events; `tail token` acquires a
management JWT and prints the Go-compatible JSON envelope.

### Ownership note

CDC owns the log-streaming protocol contract. CLI owns the entry semantics,
flag parsing, and command-level help.

## Management command (CLI-024)

Source: [baseline-2026.2.0/cmd/cloudflared/management/cmd.go](../../../baseline-2026.2.0/cmd/cloudflared/management/cmd.go)

Hidden top-level command for management operations.

- hidden: yes
- category: `Management`

### Hidden subcommand: `management token`

Gets management token with specified resource scope.

Flags:

| Flag | Type | Default | Env var | Hidden | Usage |
| --- | --- | --- | --- | --- | --- |
| `--resource` | string | | | no | **required** — resource type: `logs`, `admin`, or `host_details` |
| `--origincert` | string | `FindDefaultOriginCertPath()` | `TUNNEL_ORIGIN_CERT` | no | origin certificate path |
| `--loglevel` | string | `info` | `TUNNEL_LOGLEVEL` | no | application logging level |
| `--output` | string | | | no | log output format |

Rust coverage: parity-backed for the hidden management subtree.

- bare `management` now renders the hidden command help text with exit 0,
  matching urfave/cli command behavior
- `management token` acquires the JWT through the admitted API client path
- `management --help`, `help management`, and `management token --help`
  route to hidden help text rather than falling through to root help

### Visibility note

Both the `management` command and its `token` subcommand are hidden. They do
not appear in normal help output but must be present and functional for
internal callers and tooling.

## Coverage summary

- Total tail flags: 10
- Total management token flags: 4
- Total with Rust coverage: all 3 hidden command paths (`tail`, `tail token`,
  `management token`) plus bare `management` help routing
- Hidden commands: 3 (management, management token, tail token)
