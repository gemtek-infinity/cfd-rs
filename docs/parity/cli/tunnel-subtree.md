# Tunnel Subtree — CLI Parity Audit

This document inventories the `tunnel` command family from the frozen Go
baseline ([baseline-2026.2.0/cmd/cloudflared/tunnel/](../../../baseline-2026.2.0/cmd/cloudflared/tunnel/)) and records
current Rust coverage.

Authoritative checklist rows: CLI-008 through CLI-021, CLI-032.

## Tunnel root behavior

`tunnel` is both a command namespace and a runnable decision surface.

- `tunnel` with no subcommand invokes `TunnelCommand()` which enters the
  tunnel runtime (not help)
- `tunnel` with a recognized subcommand dispatches to the subcommand
- category: `Tunnel`
- usage: `Use Cloudflare Tunnel to expose private services to the Internet or
  to Cloudflare connected private users.`

Rust coverage: parity-backed via
[`tunnel_commands.rs`](../../../crates/cfdrs-bin/src/tunnel_commands.rs).
`execute_tunnel_bare()` now implements the Go `TunnelCommand()` five-branch
decision surface: adhoc named tunnel, quick tunnel, config-driven named
tunnel handoff, classic tunnel deprecation error, and the final runnable
error. Evidence: 6 integration tests in `cfdrs-bin`.

## Subcommand inventory

### `tunnel login` (CLI-009)

Source: `tunnel/login.go`

Creates origin certificate via browser authentication flow.

Also exposed as top-level `login` for backward compatibility (hidden when built
as subcommand).

Flags:

| Flag | Aliases | Type | Default | Usage |
| --- | --- | --- | --- | --- |
| `--loginURL` | | string | `https://dash.cloudflare.com/argotunnel` | The URL used to login |
| `--callbackURL` | | string | `https://login.cloudflareaccess.org/` | The URL used for the callback |
| `--fedramp` | `-f` | bool | false | Login with FedRAMP High environment |

Rust coverage: parity-backed via
[`tunnel_login.rs`](../../../crates/cfdrs-bin/src/tunnel_login.rs).
`execute_tunnel_login()` runs the browser-auth flow, supports FedRAMP URLs,
polls the callback store, decodes the origin cert, and writes `cert.pem`
with mode `0600`. Evidence: 8 unit tests in `cfdrs-bin` plus root/tunnel
dispatch integration coverage.

### `tunnel create` (CLI-010)

Source: `tunnel/subcommands.go`

Creates a new tunnel, producing a tunnel UUID and credentials file.

Usage: `tunnel create NAME`

Flags:

| Flag | Aliases | Type | Usage |
| --- | --- | --- | --- |
| `--output` | `-o` | string | Render output in FORMAT (json or yaml) |
| `--credentials-file` | `--cred-file` | string | Filepath to write tunnel credentials |
| `--secret` | `-s` | string | Base64 encoded secret (min 32 bytes decoded) |

Rust coverage: parity-backed via
[`tunnel_commands.rs`](../../../crates/cfdrs-bin/src/tunnel_commands.rs).
`execute_tunnel_create()` loads the origin cert, creates the tunnel through
the API, writes the credential file, and renders JSON/YAML output. Evidence:
1 parse-dispatch test, 2 NArg tests, and 8 unit tests.

### `tunnel list` (CLI-011)

Source: `tunnel/subcommands.go`

Lists existing tunnels with filtering and sorting.

Flags:

| Flag | Aliases | Type | Default | Env var | Usage |
| --- | --- | --- | --- | --- | --- |
| `--output` | `-o` | string | | | Render output in FORMAT (json or yaml) |
| `--show-deleted` | `-d` | bool | | | Include deleted tunnels |
| `--name` | `-n` | string | | | List tunnels with name |
| `--name-prefix` | `-np` | string | | | List tunnels starting with prefix |
| `--exclude-name-prefix` | `-enp` | string | | | Exclude tunnels starting with prefix |
| `--when` | `-w` | timestamp | current time | | List tunnels active at TIME (RFC3339) |
| `--id` | `-i` | string | | | List tunnel by ID |
| `--show-recently-disconnected` | `-rd` | bool | | | Include recently disconnected |
| `--sort-by` | | string | `name` | `TUNNEL_LIST_SORT_BY` | Sort field |
| `--invert-sort` | | bool | | `TUNNEL_LIST_INVERT_SORT` | Invert sort order |
| `--max-fetch-size` | | int | | | Max results to fetch |

Rust coverage: parity-backed via
[`tunnel_commands.rs`](../../../crates/cfdrs-bin/src/tunnel_commands.rs).
`execute_tunnel_list()` builds the Go-shaped filter set, calls the API, and
renders the tabular listing with per-colo connection formatting. Evidence:
1 parse-dispatch test, 1 NArg test, and 4 formatter unit tests.

### `tunnel run` (CLI-012)

Source: `tunnel/subcommands.go`

Proxies local web server by running the given tunnel.

Usage: `tunnel run [TUNNEL]`

Inherits all tunnel-level flags plus:

| Flag | Aliases | Type | Env var | Usage |
| --- | --- | --- | --- | --- |
| `--credentials-file` | | string | `TUNNEL_CRED_FILE` | Credentials filepath |
| `--credentials-contents` | | string | `TUNNEL_CRED_CONTENTS` | Credentials JSON contents |
| `--post-quantum` | `-pq` | bool | `TUNNEL_POST_QUANTUM` | Post-quantum tunnel |
| `--protocol` | `-p` | string | `TUNNEL_TRANSPORT_PROTOCOL` | Protocol implementation |
| `--features` | `-F` | string slice | | Feature opt-in |
| `--token` | | string | `TUNNEL_TOKEN` | Tunnel token |
| `--token-file` | | string | `TUNNEL_TOKEN_FILE` | Token filepath |
| `--icmpv4-src` | | string | `TUNNEL_ICMPV4_SRC` | ICMPv4 source |
| `--icmpv6-src` | | string | `TUNNEL_ICMPV6_SRC` | ICMPv6 source |
| `--max-active-flows` | | uint64 | `TUNNEL_MAX_ACTIVE_FLOWS` | Max private network flows |
| `--dns-resolver-addrs` | | string slice | `TUNNEL_DNS_RESOLVER_ADDRS` | DNS resolver overrides |

Rust coverage: parity-backed via
[`tunnel_commands.rs`](../../../crates/cfdrs-bin/src/tunnel_commands.rs).
`execute_tunnel_run()` implements the Go precedence chain
`--token` > `--token-file` > positional tunnel > config `tunnel`, handles
inline credentials contents, and hands the resolved identity into the runtime.
Evidence: 2 parse-dispatch tests, 12 unit tests, 3 NArg integration tests,
and 2 credential-discovery tests.

### `tunnel delete` (CLI-013)

Source: `tunnel/subcommands.go`

Deletes existing tunnel by UUID or name.

Usage: `tunnel delete TUNNEL`

Flags:

| Flag | Aliases | Type | Env var | Usage |
| --- | --- | --- | --- | --- |
| `--credentials-file` | `--cred-file` | string | `TUNNEL_CRED_FILE` | Credentials filepath |
| `--force` | `-f` | bool | `TUNNEL_RUN_FORCE_OVERWRITE` | Delete even if connected |

Rust coverage: parity-backed via
[`tunnel_commands.rs`](../../../crates/cfdrs-bin/src/tunnel_commands.rs).
`execute_tunnel_delete()` resolves by UUID or name, enforces the deleted-state
checks, deletes the tunnel through the API, and removes the local credential
file as a non-fatal cleanup step. Evidence: 1 parse-dispatch test and 2 NArg
tests.

### `tunnel cleanup` (CLI-014)

Source: `tunnel/subcommands.go`

Cleans up tunnel connections.

Usage: `tunnel cleanup TUNNEL`

Flags:

| Flag | Aliases | Type | Env var | Usage |
| --- | --- | --- | --- | --- |
| `--connector-id` | `-c` | string | `TUNNEL_CLEANUP_CONNECTOR` | Filter to single connector |

Rust coverage: parity-backed via
[`tunnel_commands.rs`](../../../crates/cfdrs-bin/src/tunnel_commands.rs).
`execute_tunnel_cleanup()` resolves tunnel IDs, honors `--connector-id`, and
calls the API cleanup path. Evidence: 1 parse-dispatch test and 2 NArg tests.

### `tunnel token` (CLI-015)

Source: `tunnel/subcommands.go`

Fetches credential token for existing tunnel.

Usage: `tunnel token TUNNEL`

Flags:

| Flag | Aliases | Type | Usage |
| --- | --- | --- | --- |
| `--credentials-file` | `--cred-file` | string | Credentials filepath |

Rust coverage: parity-backed via
[`tunnel_commands.rs`](../../../crates/cfdrs-bin/src/tunnel_commands.rs).
`execute_tunnel_token()` resolves the tunnel ID, fetches the token, and either
prints it or writes it to `--credentials-file`. Evidence: 1 parse-dispatch
test and 2 NArg tests.

### `tunnel info` (CLI-016)

Source: `tunnel/subcommands.go`

Lists details about active connectors.

Usage: `tunnel info TUNNEL`

Flags:

| Flag | Aliases | Type | Default | Env var | Usage |
| --- | --- | --- | --- | --- | --- |
| `--output` | `-o` | string | | | Render output in FORMAT |
| `--sort-by` | | string | `createdAt` | `TUNNEL_INFO_SORT_BY` | Sort connections |
| `--invert-sort` | | bool | | `TUNNEL_INFO_INVERT_SORT` | Invert sort order |
| `--show-recently-disconnected` | `-rd` | bool | | | Include disconnected |

Rust coverage: parity-backed via
[`tunnel_commands.rs`](../../../crates/cfdrs-bin/src/tunnel_commands.rs).
`execute_tunnel_info()` resolves the tunnel, fetches active clients, and
renders the Go-style header plus connector table. Evidence: 1 parse-dispatch
test and 2 NArg tests.

### `tunnel ready` (CLI-017)

Source: `tunnel/subcommands.go`

Calls `/ready` endpoint to check tunnel readiness. Requires `--metrics` flag
from parent tunnel command.

No subcommand-specific flags.

Rust coverage: parity-backed via `execute_tunnel_ready()` in
[`tunnel_local_commands.rs`](../../../crates/cfdrs-bin/src/tunnel_local_commands.rs).
The command requires an
explicit `--metrics` flag, performs `GET http://{metrics}/ready`, exits 0 on
HTTP 200, and returns the Go-shaped non-200 error including status code and
response body. Evidence: 1 parse-dispatch test in `cfdrs-cli` and 3 behavioral
integration tests in `cfdrs-bin`.

### `tunnel diag` (CLI-018)

Source: `tunnel/subcommands.go`

Creates diagnostic report from local cloudflared instance.

Flags:

| Flag | Type | Default | Usage |
| --- | --- | --- | --- |
| `--metrics` | string | | Metrics server address |
| `--diag-container-id` | string | | Container ID for log collection |
| `--diag-pod-id` | string | | Kubernetes pod for log collection |
| `--no-diag-logs` | bool | false | Skip log collection |
| `--no-diag-metrics` | bool | false | Skip metric collection |
| `--no-diag-system` | bool | false | Skip system info collection |
| `--no-diag-runtime` | bool | false | Skip runtime info collection |
| `--no-diag-network` | bool | false | Skip network diagnostics |

Rust coverage: parity-backed via
[`tunnel_local_commands.rs`](../../../crates/cfdrs-bin/src/tunnel_local_commands.rs)
plus `cfdrs-his` diagnostics. `execute_tunnel_diag()` builds the Go-shaped
diagnostic bundle, supports all `--no-diag-*` toggles, auto-discovers
instances on the known metrics ports when `--metrics` is absent, and
preserves the baseline-facing CLI messages for the no-instance,
multi-instance, success, and partial-success paths. Evidence: 1
parse-dispatch test in `cfdrs-cli`, 5 behavioral integration tests in
`cfdrs-bin`, and ZIP/report tests in `cfdrs-his`.

### `tunnel route` (CLI-019)

Source: `tunnel/subcommands.go`

Parent command with subcommands `dns`, `lb`, `ip`.

#### `tunnel route dns`

| Flag | Aliases | Type | Env var | Usage |
| --- | --- | --- | --- | --- |
| `--overwrite-dns` | `-f` | bool | `TUNNEL_FORCE_PROVISIONING_DNS` | Overwrite existing DNS records |

#### `tunnel route lb`

No additional flags.

#### `tunnel route ip`

Sub-subcommands: `add`, `show`/`list`, `delete`, `get`.

##### `tunnel route ip add`

| Flag | Aliases | Type | Usage |
| --- | --- | --- | --- |
| `--vnet` | `-vn` | string | Virtual network ID or name |

##### `tunnel route ip show` / `tunnel route ip list`

| Flag | Type | Usage |
| --- | --- | --- |
| `--output` | string | Render output in FORMAT (json or yaml) |

Plus IP route filter flags from `cfapi.IpRouteFilterFlags`.

##### `tunnel route ip delete`

| Flag | Aliases | Type | Usage |
| --- | --- | --- | --- |
| `--vnet` | `-vn` | string | Virtual network to delete route from |

##### `tunnel route ip get`

| Flag | Aliases | Type | Usage |
| --- | --- | --- | --- |
| `--vnet` | `-vn` | string | Virtual network to query |

Rust coverage: parity-backed via
[`route_vnet_commands.rs`](../../../crates/cfdrs-bin/src/route_vnet_commands.rs).
`tunnel route dns`, `lb`, and `ip` subcommands now resolve vnets, build the
Go-shaped API payloads, and render the expected tabular outputs. Evidence:
14 parse-dispatch and NArg tests in `cfdrs-cli` plus render/resolve unit
tests in `cfdrs-bin`.

### `tunnel vnet` (CLI-020)

Source: `tunnel/vnets_subcommands.go`

Virtual network management.

#### `tunnel vnet add`

| Flag | Aliases | Type | Usage |
| --- | --- | --- | --- |
| `--default` | `-d` | bool | Set as default virtual network |

#### `tunnel vnet list`

| Flag | Type | Usage |
| --- | --- | --- |
| `--output` | string | Render output in FORMAT |

Plus virtual network filter flags from `cfapi.VnetFilterFlags`.

#### `tunnel vnet delete`

| Flag | Aliases | Type | Usage |
| --- | --- | --- | --- |
| `--force` | `-f` | bool | Force deletion |

#### `tunnel vnet update`

| Flag | Aliases | Type | Usage |
| --- | --- | --- | --- |
| `--name` | `-n` | string | New name |
| `--comment` | `-c` | string | New comment |
| `--default` | `-d` | bool | Set as default |

Rust coverage: parity-backed via
[`route_vnet_commands.rs`](../../../crates/cfdrs-bin/src/route_vnet_commands.rs).
`tunnel vnet add`, `list`, `delete`, and `update` now resolve names/UUIDs,
pass through `--default` and `--force`, and render the expected table output.
Evidence: 9 parse-dispatch and NArg tests in `cfdrs-cli` plus 3 render and
resolver tests in `cfdrs-bin`.

### `tunnel ingress` (CLI-021)

Source: `tunnel/ingress_subcommands.go`

Hidden command for ingress inspection.

#### `tunnel ingress validate`

| Flag | Aliases | Type | Env var | Usage |
| --- | --- | --- | --- | --- |
| `--json` | `-j` | string | `TUNNEL_INGRESS_VALIDATE_JSON` | Accept JSON input |

#### `tunnel ingress rule`

Usage: `tunnel ingress rule URL`

No flags. Shows which ingress rule matches the given URL.

Rust coverage: parity-backed. Bare `tunnel ingress` now renders hidden-command
help, `validate` supports `--json` and file discovery, surfaces unknown-key
warnings, and rejects empty configs or `--url` with Go-matching behavior.
`rule URL` resolves the matching rule using strict ingress parsing instead of
the runtime's default `http_status:503` fallback, then renders the Go-style
multi-line rule body. Evidence: 3 parse-dispatch tests in `cfdrs-cli`, 3 NArg
tests + 6 behavioral integration tests in `cfdrs-bin`, and 5 unit tests in
[`tunnel_local_commands.rs`](../../../crates/cfdrs-bin/src/tunnel_local_commands.rs).

## Removed/deprecated subcommands

### `tunnel proxy-dns` (CLI-025)

Removed feature. Shows error: `dns-proxy feature is no longer supported since
version 2026.2.0`.

### `tunnel db-connect` (CLI-026)

Removed via `cliutil.RemovedCommand("db-connect")`.

## Coverage summary

- All admitted tunnel subcommands are now parity-backed, including `ready`,
  `diag`, hidden `ingress`, `route`, and `vnet`
- Removed subcommands `proxy-dns` and `db-connect` match baseline removal
  behavior
- Multi-level nesting depth remains 4 (`tunnel route ip add`)
