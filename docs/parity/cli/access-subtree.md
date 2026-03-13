# Access Subtree — CLI Parity Audit

This document inventories the `access` command family from the frozen Go
baseline ([baseline-2026.2.0/old-impl/cmd/cloudflared/access/cmd.go](../../../baseline-2026.2.0/old-impl/cmd/cloudflared/access/cmd.go)) and
records current Rust coverage.

Authoritative checklist row: CLI-022.

## Command identity

- command name: `access`
- alias: `forward`
- category: `Access`
- top-level flag: `--fedramp` (bool, use when performing operations in FedRAMP
  account)

The `forward` alias means `cloudflared forward login` is equivalent to
`cloudflared access login`.

Rust coverage: absent for entire access subtree.

## Subcommand inventory

### `access login`

Browser-based authentication flow for Access applications.

Flags:

| Flag | Aliases | Type | Default | Usage |
| --- | --- | --- | --- | --- |
| `--quiet` | `-q` | bool | false | do not print the JWT to the command line |
| `--no-verbose` | | bool | false | print only the JWT to stdout |
| `--auto-close` | | bool | false | automatically close the auth interstitial after action |
| `--app` | | string | | application URL |

Rust coverage: absent.

### `access curl`

Passes requests through Access with JWT injection.

- `SkipFlagParsing` is enabled (curl gets raw argument passthrough)
- special argument prefix `--allow-request` / `-ar` handled by
  `parseAllowRequest()`

No formally defined flags due to flag-parsing skip.

Rust coverage: absent.

### `access token`

Produces a JWT for the given Access application.

Flags:

| Flag | Type | Usage |
| --- | --- | --- |
| `--app` | string | application URL |

Rust coverage: absent.

### `access tcp` (aliases: `rdp`, `ssh`, `smb`)

TCP proxy for Access-protected services. The aliases `rdp`, `ssh`, and `smb`
are registered as separate subcommands that delegate to the same `tcp`
implementation with identical flags.

This means all of these are valid:

- `cloudflared access tcp --hostname example.com`
- `cloudflared access rdp --hostname example.com`
- `cloudflared access ssh --hostname example.com`
- `cloudflared access smb --hostname example.com`

Flags:

| Flag | Aliases | Type | Env var | Hidden | Usage |
| --- | --- | --- | --- | --- | --- |
| `--hostname` | `--tunnel-host`, `-T` | string | `TUNNEL_SERVICE_HOSTNAME` | no | hostname of your application |
| `--destination` | | string | `TUNNEL_SERVICE_DESTINATION` | no | destination address of SSH server |
| `--url` | `--listener`, `-L` | string | `TUNNEL_SERVICE_URL` | no | host:port to forward data to edge |
| `--header` | `-H` | string slice | | no | additional headers |
| `--service-token-id` | `--id` | string | `TUNNEL_SERVICE_TOKEN_ID` | no | Access service token ID |
| `--service-token-secret` | `--secret` | string | `TUNNEL_SERVICE_TOKEN_SECRET` | no | Access service token secret |
| `--logfile` | | string | | no | application log file |
| `--log-directory` | | string | | no | application log directory |
| `--log-level` | `--loglevel` | string | | no | logging level (debug, info, warn, error, fatal) |
| `--connect-to` | | string | | yes | alternate connection for testing |
| `--debug-stream` | | uint64 | | yes | max stream payloads to log as debug |

Rust coverage: absent.

### `access ssh-config`

Prints SSH config snippet for the given Access application.

Flags:

| Flag | Type | Default | Usage |
| --- | --- | --- | --- |
| `--hostname` | string | | hostname of your application |
| `--short-lived-cert` | bool | false | generate short-lived certs |

Rust coverage: absent.

### `access ssh-gen`

Generates short-lived SSH certificate.

Flags:

| Flag | Type | Usage |
| --- | --- | --- |
| `--hostname` | string | hostname of your application |

Rust coverage: absent.

## Coverage summary

- Total access subcommands: 6 (login, curl, token, tcp, ssh-config, ssh-gen)
- Plus 3 TCP aliases: rdp, ssh, smb
- Total with Rust coverage: 0
- Total subcommand-specific flags: approximately 20
