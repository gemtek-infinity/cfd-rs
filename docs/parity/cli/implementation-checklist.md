# CLI Implementation Checklist

## Purpose

This document is the live parity ledger for the cloudflared CLI surface.

Parity in this document means parity against the frozen Go baseline for:

- visible command structure
- hidden and compatibility command structure
- help and usage text
- flag and environment-variable binding behavior
- exit-code behavior
- stdout and stderr behavior
- formatting details that are blackbox-visible

This document does not claim parity from Rust code shape alone.

It records:

- the frozen CLI behavior or contract that must be matched
- the current Rust owner, if any
- the current Rust implementation state
- the current evidence maturity
- whether a gap or divergence is open
- the tests required before parity can be claimed

## Checklist Field Vocabulary

The table uses three different status fields.

### Rust status now

Use only these values:

- not audited
- audited, absent
- audited, partial
- audited, parity-backed
- audited, intentional divergence
- blocked

### Parity evidence status

Preferred values:

- not present
- minimal
- weak
- partial
- parity-backed
- first-slice evidence exists
- partial local tests only

If a new value is needed later, add it deliberately and keep it short.

### Divergence status

Preferred values:

- none recorded
- open gap
- intentional divergence
- unknown
- blocked

## Audited Checklist

This checklist was produced by source-level audit of the frozen Go baseline
in `baseline-2026.2.0/old-impl/cmd/cloudflared/` and comparison against
the current Rust CLI surface in `crates/cloudflared-cli/src/surface/`.

The frozen Go CLI uses `urfave/cli` v2. The current Rust CLI uses a custom
hand-written parser (no clap or structopt).

### Root And Global Surface

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-001 | root invocation | `cmd/cloudflared/main.go` `action()` | empty invocation enters service mode via `handleServiceMode()`: creates config file watcher, initializes `FileManager`, creates `AppManager` and `AppService`, runs daemonically. Not help. | current CLI surface | audited, absent | not present | open gap | blackbox empty invocation capture, stdout and stderr capture, exit-code compare, service-mode behavior test | critical | current Rust defaults to `help` on empty invocation; Go enters service mode |
| CLI-002 | root help text | root app `--help` output | root help exposes 9 top-level command families: `update`, `version`, `tunnel`, `login` (compat), `proxy-dns` (removed), `access` (alias `forward`), `tail`, `management` (hidden), `service` (Linux). Frozen wording, ordering, spacing from urfave/cli | current CLI surface | audited, absent | not present | open gap | exact help snapshot compare, top-level command inventory capture | critical | current Rust help only exposes `validate`, `run`, `help`, `version` |
| CLI-003 | root global flags | `cmd/cloudflared/tunnel/cmd.go` `Flags()` | 50+ global flags including: `--config`, `--credentials-file`/`-cred-file` (env `TUNNEL_CRED_FILE`), `--credentials-contents` (env `TUNNEL_CRED_CONTENTS`), `--token` (env `TUNNEL_TOKEN`), `--token-file` (env `TUNNEL_TOKEN_FILE`), `--origincert` (env `TUNNEL_ORIGIN_CERT`), `--loglevel` (env `TUNNEL_LOGLEVEL`, default `info`), `--logfile`, `--log-directory`, `--output` (json/default), `--edge` (hidden, env `TUNNEL_EDGE`), `--region` (env `TUNNEL_REGION`), `--edge-ip-version` (env `TUNNEL_EDGE_IP_VERSION`, default `4`), `--edge-bind-address`, `--metrics`, `--metrics-update-freq` (default 5s), `--protocol`/`-p` (hidden, env `TUNNEL_TRANSPORT_PROTOCOL`), `--post-quantum`/`-pq` (hidden, env `TUNNEL_POST_QUANTUM`), `--features`/`-F` (env `TUNNEL_FEATURES`), `--no-autoupdate`, `--autoupdate-freq`, `--tunnel`/`--name` (env `TUNNEL_NAME`), `--hostname` (hidden), `--lb-pool`, `--url`, `--hello-world`, `--pidfile`, `--tag` (hidden), `--ha-connections` (hidden, default 4), `--retries` (default 5), `--max-edge-addr-retries` (hidden, default 8), `--rpc-timeout` (hidden, default 5s), `--grace-period` (default 30s), `--label`, `--max-active-flows`, `--quiet`/`-q`, `--version`/`-v`/`-V`, `--api-url` (hidden, default `https://api.cloudflare.com/client/v4`), `--is-autoupdated` (hidden), `--api-key`/`--api-email`/`--api-ca-key` (all hidden, deprecated), `--profile` (hidden), `--workers` (hidden), plus proxy-origin flags (`--unix-socket`, `--http-host-header`, `--origin-server-name`, `--origin-ca-pool`, `--no-tls-verify`, `--no-chunked-encoding`, `--http2-origin`), plus ICMP flags (`--icmpv4-src`, `--icmpv6-src`), plus proxy-dns flags (removed feature) | current CLI surface | audited, absent | not present | open gap | flag inventory capture, env-binding tests, default-value tests, hidden-flag tests, alias tests | critical | current Rust only supports `--config`; all other 50+ flags are absent. See `docs/parity/cli/root-and-global-flags.md` |
| CLI-004 | help command behavior | root help command | explicit `help` command and `--help`/`-h` flag routing for root and subcommands; urfave/cli generates command-local help automatically | current CLI surface | audited, partial | minimal | open gap | help-command snapshot tests, subcommand help-routing tests, exit-code tests | high | current Rust has `help` and `--help`/`-h`, exit code 0; but output is alpha-only, not upstream-parity-backed |
| CLI-005 | version command | `cmd/cloudflared/main.go` app version config | format: `{Version} (built {BuildTime}{BuildTypeMsg})`; `--short`/`-s` flag outputs version number only; `--version`/`-v`/`-V` flags also trigger version output | current CLI surface | audited, partial | minimal | open gap | exact stdout snapshot compare, `--short`/`-s` flag tests, exit-code tests | high | current Rust outputs `cloudflared 2026.2.0-alpha.202603` (no build time, no short mode, no `-s` flag) |
| CLI-006 | update command | `cmd/cloudflared/updater/update.go` | `update` command with flags: `--beta`, `--force` (hidden), `--staging` (hidden), `--version`; returns exit code 11 if update occurred; otherwise 0 | none | audited, absent | not present | open gap | help capture, update-behavior tests, exit-code tests (exit 11 on success) | high | includes behavior when update is attempted, not just command presence |
| CLI-007 | service command | `cmd/cloudflared/linux_service.go` | `service` command with subcommands `install` and `uninstall`; flag `--no-update-service` (default false); systemd: creates `/etc/systemd/system/cloudflared.service`, `/etc/systemd/system/cloudflared-update.service`, `/etc/systemd/system/cloudflared-update.timer`; SysV fallback: `/etc/init.d/cloudflared` | none | audited, absent | not present | open gap | help capture, service install/uninstall tests, generated-asset tests, exit-code tests | critical | command-surface parity is CLI-owned; host effects are HIS-owned |

### Tunnel Command Surface

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-008 | tunnel root behavior | `cmd/cloudflared/tunnel/cmd.go` | `tunnel` is both a command namespace and a runnable decision surface; `tunnel` with no subcommand invokes `tunnel.TunnelCommand()` which enters the tunnel runtime; `tunnel` with subcommand dispatches to the subcommand; category `Tunnel`; usage text `Use Cloudflare Tunnel to expose private services to the Internet or to Cloudflare connected private users.` | none | audited, absent | not present | open gap | blackbox tunnel invocation matrix (no-args, with-args, with-subcommand), stdout/stderr capture, exit-code tests | critical | not just a static command tree |
| CLI-009 | tunnel login | `cmd/cloudflared/tunnel/login.go` | `tunnel login` generates cert via browser auth; also exposed as top-level `login` for backward compat (hidden at top level when built as subcommand); `--fedramp`/`-f` flag for FedRAMP support | none | audited, absent | not present | open gap | help capture, login-flow tests (browser auth is external), flag tests | high | top-level `login` is backward-compat alias |
| CLI-010 | tunnel create | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel create NAME` creates a new tunnel; produces tunnel UUID and credentials file | none | audited, absent | not present | open gap | help capture, creation-flow tests, output-format tests | critical | API interaction owned by CDC |
| CLI-011 | tunnel list | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel list` lists existing tunnels; supports filtering and sorting flags | none | audited, absent | not present | open gap | help capture, list-output tests, filter-flag tests | high | |
| CLI-012 | tunnel run | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel run [TUNNEL]` proxies local web server by running the given tunnel; named-tunnel flow requires credentials | none | audited, absent | not present | open gap | help capture, run invocation matrix, credential-resolution tests | critical | current Rust `run` partially overlaps but is not equivalent |
| CLI-013 | tunnel delete | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel delete TUNNEL` deletes existing tunnel by UUID or name | none | audited, absent | not present | open gap | help capture, delete-flow tests | high | |
| CLI-014 | tunnel cleanup | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel cleanup TUNNEL` cleans up tunnel connections; `--connector-id` flag to filter | none | audited, absent | not present | open gap | help capture, cleanup tests | medium | |
| CLI-015 | tunnel token | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel token TUNNEL` fetches credential token for existing tunnel by name or UUID | none | audited, absent | not present | open gap | help capture, token-output tests | high | |
| CLI-016 | tunnel info | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel info TUNNEL` lists details about active connectors | none | audited, absent | not present | open gap | help capture, info-output tests | medium | |
| CLI-017 | tunnel ready | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel ready` calls `/ready` endpoint; requires `--metrics` flag; returns proper exit code | none | audited, absent | not present | open gap | help capture, ready-endpoint tests, exit-code tests | medium | requires local metrics endpoint (HIS dependency) |
| CLI-018 | tunnel diag | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel diag` creates diagnostic report from local cloudflared instance | none | audited, absent | not present | open gap | help capture, diagnostic-output tests | medium | overlaps HIS diagnostics collection |
| CLI-019 | tunnel route | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel route` parent command with subcommands `dns`, `lb`, `ip`; `ip` has sub-subcommands `add`, `show`/`list`, `delete`, `get` | none | audited, absent | not present | open gap | help capture, per-subcommand tests | high | multi-level nesting: `tunnel route ip add` |
| CLI-020 | tunnel vnet | `cmd/cloudflared/tunnel/vnets_subcommands.go` | `tunnel vnet` with subcommands `add` (with `--default`), `list`, `delete` (with `--force`), `update` (with `--name`, `--comment`) | none | audited, absent | not present | open gap | help capture, per-subcommand tests | medium | |
| CLI-021 | tunnel ingress | `cmd/cloudflared/tunnel/ingress_subcommands.go` | `tunnel ingress` (hidden) with subcommands `validate` and `rule`; `validate` validates ingress from config; `rule URL` shows which rule matches | none | audited, absent | not present | open gap | help capture, validate/rule tests, hidden-command tests | medium | hidden command |

### Access, Tail, And Management Surface

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-022 | access subtree | `cmd/cloudflared/access/cmd.go` | `access` command (alias `forward`) with subcommands: `login` (browser auth), `curl` (JWT injection), `token` (JWT production), `tcp` (aliases `rdp`, `ssh`, `smb` for TCP/RDP/SSH/SMB proxy), `ssh-config` (print SSH config), `ssh-gen` (generate short-lived cert); `--fedramp` flag | none | audited, absent | not present | open gap | subtree help crawl, alias tests (`forward`), tcp-alias tests (`rdp`, `ssh`, `smb`), per-subcommand behavior tests | high | see `docs/parity/cli/access-subtree.md` |
| CLI-023 | tail subtree | `cmd/cloudflared/tail/cmd.go` | `tail [TUNNEL-ID]` streams remote logs; flags: `--connector-id`, `--event` (filter: cloudflared/http/tcp/udp), `--level` (default `debug`), `--sample` (default 1.0), `--token` (env `TUNNEL_MANAGEMENT_TOKEN`), `--management-hostname` (hidden, default `management.argotunnel.com`), `--trace` (hidden); hidden subcommand `token` gets management JWT | none | audited, absent | not present | open gap | help crawl, filter tests, hidden `token` subcommand tests, output-format tests | high | CDC owns the log-streaming contract; CLI owns entry semantics. See `docs/parity/cli/tail-and-management.md` |
| CLI-024 | management subtree | `cmd/cloudflared/management/cmd.go` | `management` (hidden, category `Management`) with hidden subcommand `token`; token subcommand requires `--resource` (values: `logs`, `admin`, `host_details`), `--origincert`, `--loglevel` | none | audited, absent | not present | open gap | hidden-command help capture, token invocation tests, resource-flag tests | medium | entirely hidden from normal help output |

### Compatibility, Formatting, And Error Behavior

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-025 | compatibility: proxy-dns | `cmd/cloudflared/proxydns/cmd.go`, `cmd/cloudflared/tunnel/cmd.go` | top-level `proxy-dns` prints deprecation error with link to DNS-over-HTTPS alternative; `tunnel proxy-dns` shows error `dns-proxy feature is no longer supported since version 2026.2.0` | none | audited, absent | not present | open gap | placeholder failure tests, stderr snapshot, exit-code tests | high | two separate removal paths: top-level and under tunnel |
| CLI-026 | compatibility: db-connect | `cmd/cloudflared/tunnel/cmd.go` | `tunnel db-connect` shows removed-command error via `cliutil.RemovedCommand("db-connect")` | none | audited, absent | not present | open gap | removed-command failure test, stderr snapshot, exit-code test | medium | |
| CLI-027 | compatibility: classic tunnels | `cmd/cloudflared/tunnel/cmd.go` | classic tunnel invocation paths produce error: `Classic tunnels have been deprecated, please use Named Tunnels` | none | audited, absent | not present | open gap | deprecation-error tests | medium | |
| CLI-028 | compatibility: login at root | `cmd/cloudflared/main.go` | `login` is registered as a top-level command for backward compatibility (delegates to tunnel login); hidden when built as subcommand | none | audited, absent | not present | open gap | top-level login invocation test, help-visibility test | high | must be present at top level for compat |
| CLI-029 | help formatting contract | blackbox output | urfave/cli generates help with specific spacing, wrapping, headings, command ordering, category grouping; exact text is visible contract | current CLI surface | audited, partial | minimal | open gap | exact text snapshots, width-sensitive capture, no substring-only proofs | critical | current Rust help is custom-generated with different format than urfave/cli |
| CLI-030 | usage failure behavior | blackbox error output | unknown commands produce urfave/cli error text plus suggestions; bad flags produce flag-specific errors; exit code semantics from urfave/cli | current CLI surface | audited, partial | minimal | open gap | stderr/stdout capture, exit-code matrix, unknown-command tests, bad-flag tests | high | current Rust has usage failure logic with exit code 2, but output does not match urfave/cli format |

### Transitional Rust-Only Commands

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-031 | validate command (Rust-only) | no frozen baseline equivalent | `validate` is a transitional alpha command that resolves config, loads YAML, normalizes ingress, and reports startup readiness; not present in baseline top-level surface | current CLI surface | audited, intentional divergence | partial local tests only | intentional divergence | divergence note, transitional command tests, retirement/rename tracking | medium | may become internal, renamed, or retired; not a parity target |
| CLI-032 | run command (Rust alpha) | partial overlap with frozen `tunnel` and `tunnel run` | current Rust `run` enters QUIC transport core + Pingora proxy seam; only partially overlaps frozen `tunnel` root runnable behavior and `tunnel run` named-tunnel flow; must not be treated as CLI parity | current runtime + current CLI surface | audited, partial | partial local tests only | open gap | command contract tests, compare against frozen `tunnel` root and `tunnel run` behavior | critical | must be reconciled against upstream `tunnel` root and `tunnel run`, not treated as equivalent by name alone |

## Audit Summary

### Baseline command inventory (frozen Go)

Top-level commands: `update`, `version`, `tunnel`, `login` (compat), `proxy-dns` (removed), `access` (alias `forward`), `tail`, `management` (hidden), `service` (Linux)

Tunnel subcommands: `login`, `create`, `list`, `ready`, `info`, `delete`, `run`, `cleanup`, `token`, `route` (with `dns`/`lb`/`ip` sub-subcommands), `vnet` (with `add`/`list`/`delete`/`update`), `ingress` (hidden, with `validate`/`rule`), `diag`, `proxy-dns` (removed), `db-connect` (removed)

Access subcommands: `login`, `curl`, `token`, `tcp` (aliases `rdp`/`ssh`/`smb`), `ssh-config`, `ssh-gen`

Tail subcommands: `token` (hidden)

Management subcommands: `token` (hidden)

Service subcommands: `install`, `uninstall`

Total callable command paths: 40+

### Current Rust CLI surface

Implemented commands: `help`, `version`, `validate` (transitional), `run` (alpha-limited)

Implemented flags: `--config`, `--help`/`-h`, `--version`/`-V`

Missing from baseline: 36+ command paths, 50+ global flags, all subcommand trees

### Gap ranking by priority

Critical gaps:

- CLI-001: root invocation behavior (service mode vs help)
- CLI-002: root help text (9 command families vs 4)
- CLI-003: root global flags (50+ vs 1)
- CLI-007: service command
- CLI-008: tunnel root behavior
- CLI-010: tunnel create
- CLI-012: tunnel run
- CLI-029: help formatting contract
- CLI-032: run command reconciliation

High gaps:

- CLI-004: help command subcommand routing
- CLI-005: version short mode
- CLI-006: update command
- CLI-009: tunnel login (compat alias)
- CLI-011: tunnel list
- CLI-013: tunnel delete
- CLI-015: tunnel token
- CLI-019: tunnel route (multi-level nesting)
- CLI-022: access subtree (6 subcommands + aliases)
- CLI-023: tail subtree (hidden token subcommand)
- CLI-025: proxy-dns compatibility
- CLI-028: login at root compatibility
- CLI-030: usage failure behavior

## Immediate Work Queue

1. ~~create `docs/parity/cli/root-and-global-flags.md`~~ — done
2. ~~create `docs/parity/cli/tunnel-subtree.md`~~ — done
3. ~~create `docs/parity/cli/access-subtree.md`~~ — done
4. ~~create `docs/parity/cli/tail-and-management.md`~~ — done
5. ~~capture frozen Go help output for all callable paths~~ — done;
   captures in `docs/parity/cli/captures/`:
   - `root-surface.txt` — root help, empty invocation, version
   - `tunnel-subtree.txt` — tunnel and all tunnel subcommand help
   - `access-subtree.txt` — access subtree and forward alias
   - `tail-management-service-update.txt` — tail, management, service, update
   - `error-and-compat.txt` — unknown commands, bad flags, proxy-dns, db-connect
   - `rust-current-surface.txt` — current Rust binary outputs for comparison

### Confirmed Divergences From Captures

**Root invocation (CLI-001):** Go empty invocation enters service mode
(`unable to find config file`, exit 1). Rust empty invocation prints help
text, exit 0. This is the highest-priority behavioral divergence.

**Version format (CLI-005):** Go outputs `cloudflared version DEV (built unknown)`;
`--short`/`-s` outputs `DEV` only. Rust outputs `cloudflared 2026.2.0-alpha.202603`
with no `--short` or `-s` support.

**db-connect removal (CLI-026):** Go exits 255, not 1. Confirmed from blackbox.

**proxy-dns removal (CLI-025):** Error message includes full deprecation URL
(`https://developers.cloudflare.com/1.1.1.1/dns-over-https/cloudflared-proxy/`).

**forward alias (CLI-022):** `forward` produces identical output to `access --help`.
Confirmed from blackbox.

### Remaining Work (Post-Audit Stages)

1. replace substring-only Rust CLI tests with snapshot-grade parity tests
   where a surface is implemented — owned by Stage 3 refactor
2. root invocation divergence is now documented above and in captures
3. version format divergence is now documented above and in captures
