# Access Subtree — CLI Parity Audit

This document inventories the `access` command family from the frozen Go
baseline ([baseline-2026.2.0/cmd/cloudflared/access/cmd.go](../../../baseline-2026.2.0/cmd/cloudflared/access/cmd.go)) and
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

Rust coverage: parse, dispatch, and help complete. Bare `access`
and `forward` alias both show access help text. All subcommands
dispatch explicitly through `cfdrs-bin`; `login`, `curl`, `token`,
`tcp`, and `ssh-gen` now return command-specific deferred-boundary
errors, while `ssh-config` renders a real SSH config snippet.

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

Rust coverage: parsed as `AccessSubcommand::Login`; dispatch to
an explicit deferred boundary that names the missing browser auth
and token-storage runtime.

### `access curl`

Passes requests through Access with JWT injection.

- `SkipFlagParsing` is enabled (curl gets raw argument passthrough)
- special argument prefix `--allow-request` / `-ar` handled by
  `parseAllowRequest()`

No formally defined flags due to flag-parsing skip.

Rust coverage: parsed as `AccessSubcommand::Curl`; dispatch to
an explicit deferred boundary that names the missing curl wrapper,
token flow, and JWT injection runtime.

### `access token`

Produces a JWT for the given Access application.

Flags:

| Flag | Type | Usage |
| --- | --- | --- |
| `--app` | string | application URL |

Rust coverage: parsed as `AccessSubcommand::Token`; dispatch to
an explicit deferred boundary that names the missing token-storage
runtime.

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

Rust coverage: parsed as `AccessSubcommand::Tcp`; all four names
(`tcp`, `rdp`, `ssh`, `smb`) parse to the same variant; dispatch to
an explicit deferred boundary that names the missing carrier proxy
runtime. Integration tests verify alias equivalence.

### `access ssh-config`

Prints SSH config snippet for the given Access application.

Flags:

| Flag | Type | Default | Usage |
| --- | --- | --- | --- |
| `--hostname` | string | | hostname of your application |
| `--short-lived-cert` | bool | false | generate short-lived certs |

Rust coverage: parsed as `AccessSubcommand::SshConfig`; dispatch to
real output. Rust renders the SSH config snippet locally, including
the `--short-lived-cert` variant and `--hostname` aliases.

### `access ssh-gen`

Generates short-lived SSH certificate.

Flags:

| Flag | Type | Usage |
| --- | --- | --- |
| `--hostname` | string | hostname of your application |

Rust coverage: parsed as `AccessSubcommand::SshGen`; dispatch to
an explicit deferred boundary that names the missing short-lived
certificate runtime.

## Coverage summary

- Total access subcommands: 6 (login, curl, token, tcp, ssh-config, ssh-gen)
- Plus 3 TCP aliases: rdp, ssh, smb
- CLI surface coverage: complete (parse, dispatch, help, aliases)
- Behavioral coverage: 5 explicit deferred boundaries + 1 real
  implementation (`ssh-config`)
- Total subcommand-specific flags: approximately 20 (not yet parsed
  per-subcommand; Go baseline handles these inside handler functions)

## Test evidence

- 14 access/help-surface tests in `cfdrs-cli` covering parse dispatch,
  alias routing, help routing, access-help rendering, and root-help alias
  visibility
- 13 integration tests in `cfdrs-bin`:
  - `access_bare_shows_help` — bare `access` shows help text, exit 0
  - `forward_alias_shows_access_help` — `forward` shows access help
  - `access_bare_and_forward_produce_same_output` — alias equivalence
  - `access_login_reaches_explicit_deferred_boundary` — deferred
    browser-flow message verified
  - `access_curl_reaches_explicit_deferred_boundary` — deferred curl/JWT
    message verified
  - `access_token_reaches_explicit_deferred_boundary` — deferred
    token-storage message verified
  - `access_tcp_reaches_explicit_deferred_boundary` — deferred carrier
    message verified
  - `access_rdp_alias_dispatches_same_as_tcp` — alias equivalence
  - `access_ssh_alias_dispatches_same_as_tcp` — alias equivalence
  - `access_smb_alias_dispatches_same_as_tcp` — alias equivalence
  - `access_ssh_config_renders_real_output` — SSH config snippet verified
  - `access_ssh_config_supports_short_lived_cert_flag` — short-lived cert
    template verified
  - `access_ssh_gen_reaches_explicit_deferred_boundary` — deferred SSH cert
    message verified
- 4 unit tests in [`crates/cfdrs-bin/src/access_commands.rs`](../../../crates/cfdrs-bin/src/access_commands.rs)
  covering deferred messaging, default hostname rendering, short-lived
  cert output, and hostname alias parsing
