# Tunnel Subtree — CLI Parity Audit

This document inventories the `tunnel` command family from the frozen Go
baseline ([baseline-2026.2.0/old-impl/cmd/cloudflared/tunnel/](../../../baseline-2026.2.0/old-impl/cmd/cloudflared/tunnel/)) and records
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

Rust coverage: the current `run` command partially overlaps `tunnel run` but
does not cover the `tunnel` root runnable behavior, and the `tunnel` command
namespace does not exist in Rust.

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

Rust coverage: absent.

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

Rust coverage: absent.

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

Rust coverage: absent.

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

Rust coverage: current `run` command partially overlaps but is not equivalent.
See CLI-032.

### `tunnel delete` (CLI-013)

Source: `tunnel/subcommands.go`

Deletes existing tunnel by UUID or name.

Usage: `tunnel delete TUNNEL`

Flags:

| Flag | Aliases | Type | Env var | Usage |
| --- | --- | --- | --- | --- |
| `--credentials-file` | `--cred-file` | string | `TUNNEL_CRED_FILE` | Credentials filepath |
| `--force` | `-f` | bool | `TUNNEL_RUN_FORCE_OVERWRITE` | Delete even if connected |

Rust coverage: absent.

### `tunnel cleanup` (CLI-014)

Source: `tunnel/subcommands.go`

Cleans up tunnel connections.

Usage: `tunnel cleanup TUNNEL`

Flags:

| Flag | Aliases | Type | Env var | Usage |
| --- | --- | --- | --- | --- |
| `--connector-id` | `-c` | string | `TUNNEL_CLEANUP_CONNECTOR` | Filter to single connector |

Rust coverage: absent.

### `tunnel token` (CLI-015)

Source: `tunnel/subcommands.go`

Fetches credential token for existing tunnel.

Usage: `tunnel token TUNNEL`

Flags:

| Flag | Aliases | Type | Usage |
| --- | --- | --- | --- |
| `--credentials-file` | `--cred-file` | string | Credentials filepath |

Rust coverage: absent.

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

Rust coverage: absent.

### `tunnel ready` (CLI-017)

Source: `tunnel/subcommands.go`

Calls `/ready` endpoint to check tunnel readiness. Requires `--metrics` flag
from parent tunnel command.

No subcommand-specific flags.

Rust coverage: absent.

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

Rust coverage: absent.

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

Rust coverage: absent for entire route subtree.

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

Rust coverage: absent for entire vnet subtree.

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

Rust coverage: absent for entire ingress subtree. The current Rust `validate`
command has partial overlap with `tunnel ingress validate` but is a
transitional alpha command, not a parity target.

## Removed/deprecated subcommands

### `tunnel proxy-dns` (CLI-025)

Removed feature. Shows error: `dns-proxy feature is no longer supported since
version 2026.2.0`.

### `tunnel db-connect` (CLI-026)

Removed via `cliutil.RemovedCommand("db-connect")`.

## Coverage summary

- Total tunnel subcommands: 13 active + 2 removed
- Total with Rust coverage: 0 (partial overlap via `run` only)
- Total subcommand-specific flags: approximately 40
- Multi-level nesting depth: 4 (`tunnel route ip add`)
