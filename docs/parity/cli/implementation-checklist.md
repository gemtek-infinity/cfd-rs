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
- compare-backed
- local tests

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
in [baseline-2026.2.0/cmd/cloudflared/](../../../baseline-2026.2.0/cmd/cloudflared/) and comparison against
the current Rust CLI surface in [crates/cfdrs-cli/src/](../../../crates/cfdrs-cli/src/).

The frozen Go CLI uses `urfave/cli` v2. The current Rust CLI uses a custom
hand-written parser (no clap or structopt).

### Root And Global Surface

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-001 | root invocation | `cmd/cloudflared/main.go` `action()` | empty invocation enters service mode via `handleServiceMode()`: creates config file watcher, initializes `FileManager`, creates `AppManager` and `AppService`, runs daemonically. Not help. | current CLI surface | audited, partial | minimal | open gap | blackbox empty invocation capture, stdout and stderr capture, exit-code compare, service-mode behavior test | critical | Rust now parses empty invocation as `ServiceMode` and dispatches to stub; runtime behavior not yet implemented |
| CLI-002 | root help text | root app `--help` output | root help exposes 9 top-level command families: `update`, `version`, `tunnel`, `login` (compat), `proxy-dns` (removed), `access` (alias `forward`), `tail`, `management` (hidden), `service` (Linux). Frozen wording, ordering, spacing from urfave/cli | current CLI surface | audited, partial | minimal | open gap | exact help snapshot compare, top-level command inventory capture | critical | Rust parses and dispatches all 9 top-level commands; help text not yet upstream-format-matched |
| CLI-003 | root global flags | `cmd/cloudflared/tunnel/cmd.go` `Flags()` | 50+ global flags including: `--config`, `--credentials-file`/`-cred-file` (env `TUNNEL_CRED_FILE`), `--credentials-contents` (env `TUNNEL_CRED_CONTENTS`), `--token` (env `TUNNEL_TOKEN`), `--token-file` (env `TUNNEL_TOKEN_FILE`), `--origincert` (env `TUNNEL_ORIGIN_CERT`), `--loglevel` (env `TUNNEL_LOGLEVEL`, default `info`), `--logfile`, `--log-directory`, `--output` (json/default), `--edge` (hidden, env `TUNNEL_EDGE`), `--region` (env `TUNNEL_REGION`), `--edge-ip-version` (env `TUNNEL_EDGE_IP_VERSION`, default `4`), `--edge-bind-address`, `--metrics`, `--metrics-update-freq` (default 5s), `--protocol`/`-p` (hidden, env `TUNNEL_TRANSPORT_PROTOCOL`), `--post-quantum`/`-pq` (hidden, env `TUNNEL_POST_QUANTUM`), `--features`/`-F` (env `TUNNEL_FEATURES`), `--no-autoupdate`, `--autoupdate-freq`, `--tunnel`/`--name` (env `TUNNEL_NAME`), `--hostname` (hidden), `--lb-pool`, `--url`, `--hello-world`, `--pidfile`, `--tag` (hidden), `--ha-connections` (hidden, default 4), `--retries` (default 5), `--max-edge-addr-retries` (hidden, default 8), `--rpc-timeout` (hidden, default 5s), `--grace-period` (default 30s), `--label`, `--max-active-flows`, `--quiet`/`-q`, `--version`/`-v`/`-V`, `--api-url` (hidden, default `https://api.cloudflare.com/client/v4`), `--is-autoupdated` (hidden), `--api-key`/`--api-email`/`--api-ca-key` (all hidden, deprecated), `--profile` (hidden), `--workers` (hidden), plus proxy-origin flags (`--unix-socket`, `--http-host-header`, `--origin-server-name`, `--origin-ca-pool`, `--no-tls-verify`, `--no-chunked-encoding`, `--http2-origin`), plus ICMP flags (`--icmpv4-src`, `--icmpv6-src`), plus proxy-dns flags (removed feature) | current CLI surface | audited, partial | minimal | open gap | flag inventory capture, env-binding tests, default-value tests, hidden-flag tests, alias tests | critical | Rust now parses 40+ flags into GlobalFlags struct; env-var bindings and default-value semantics not yet implemented |
| CLI-004 | help command behavior | root help command | explicit `help` command and `--help`/`-h` flag routing for root and subcommands; urfave/cli generates command-local help automatically | current CLI surface | audited, partial | minimal | open gap | help-command snapshot tests, subcommand help-routing tests, exit-code tests | high | current Rust has `help` and `--help`/`-h`, exit code 0; but output is alpha-only, not upstream-parity-backed |
| CLI-005 | version command | `cmd/cloudflared/main.go` app version config | format: `{Version} (built {BuildTime}{BuildTypeMsg})`; `--short`/`-s` flag outputs version number only; `--version`/`-v`/`-V` flags also trigger version output | current CLI surface | audited, partial | minimal | open gap | exact stdout snapshot compare, `--short`/`-s` flag tests, exit-code tests | high | Rust now supports `--short`/`-s` flag; build-time format not yet matched |
| CLI-006 | update command | `cmd/cloudflared/updater/update.go` | `update` command with flags: `--beta`, `--force` (hidden), `--staging` (hidden), `--version`; returns exit code 11 if update occurred; otherwise 0 | current CLI surface | audited, partial | minimal | open gap | help capture, update-behavior tests, exit-code tests (exit 11 on success) | high | Rust parses and dispatches to stub; update logic not implemented |
| CLI-007 | service command | `cmd/cloudflared/linux_service.go` | `service` command with subcommands `install` and `uninstall`; flag `--no-update-service` (default false); systemd: creates `/etc/systemd/system/cloudflared.service`, `/etc/systemd/system/cloudflared-update.service`, `/etc/systemd/system/cloudflared-update.timer`; SysV fallback: `/etc/init.d/cloudflared` | current CLI surface | audited, partial | minimal | open gap | help capture, service install/uninstall tests, generated-asset tests, exit-code tests | critical | Rust parses `service install/uninstall` and `--no-update-service` flag; host effects are HIS-owned |

### Tunnel Command Surface

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-008 | tunnel root behavior | `cmd/cloudflared/tunnel/cmd.go` | `tunnel` is both a command namespace and a runnable decision surface; `tunnel` with no subcommand invokes `tunnel.TunnelCommand()` which enters the tunnel runtime; `tunnel` with subcommand dispatches to the subcommand; category `Tunnel`; usage text `Use Cloudflare Tunnel to expose private services to the Internet or to Cloudflare connected private users.` | current CLI surface | audited, partial | minimal | open gap | blackbox tunnel invocation matrix (no-args, with-args, with-subcommand), stdout/stderr capture, exit-code tests | critical | Rust parses tunnel bare/run and all subcommands; runtime behavior not yet equivalent |
| CLI-009 | tunnel login | `cmd/cloudflared/tunnel/login.go` | `tunnel login` generates cert via browser auth; also exposed as top-level `login` for backward compat (hidden at top level when built as subcommand); `--fedramp`/`-f` flag for FedRAMP support | current CLI surface | audited, partial | minimal | open gap | help capture, login-flow tests (browser auth is external), flag tests | high | Rust parses `tunnel login` and top-level `login`; auth flow not implemented |
| CLI-010 | tunnel create | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel create NAME` creates a new tunnel; produces tunnel UUID and credentials file | current CLI surface | audited, partial | minimal | open gap | help capture, creation-flow tests, output-format tests | critical | Rust parses `tunnel create` and dispatches to stub |
| CLI-011 | tunnel list | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel list` lists existing tunnels; supports filtering and sorting flags | current CLI surface | audited, partial | minimal | open gap | help capture, list-output tests, filter-flag tests | high | Rust parses `tunnel list` and dispatches to stub |
| CLI-012 | tunnel run | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel run [TUNNEL]` proxies local web server by running the given tunnel; named-tunnel flow requires credentials | current CLI surface | audited, partial | minimal | open gap | help capture, run invocation matrix, credential-resolution tests | critical | Rust parses `tunnel run` with credential flags and enters runtime shell |
| CLI-013 | tunnel delete | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel delete TUNNEL` deletes existing tunnel by UUID or name | current CLI surface | audited, partial | minimal | open gap | help capture, delete-flow tests | high | Rust parses `tunnel delete` and dispatches to stub |
| CLI-014 | tunnel cleanup | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel cleanup TUNNEL` cleans up tunnel connections; `--connector-id` flag to filter | current CLI surface | audited, partial | minimal | open gap | help capture, cleanup tests | medium | Rust parses `tunnel cleanup` and dispatches to stub |
| CLI-015 | tunnel token | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel token TUNNEL` fetches credential token for existing tunnel by name or UUID | current CLI surface | audited, partial | minimal | open gap | help capture, token-output tests | high | Rust parses `tunnel token` and dispatches to stub |
| CLI-016 | tunnel info | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel info TUNNEL` lists details about active connectors | current CLI surface | audited, partial | minimal | open gap | help capture, info-output tests | medium | Rust parses `tunnel info` and dispatches to stub |
| CLI-017 | tunnel ready | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel ready` calls `/ready` endpoint; requires `--metrics` flag; returns proper exit code | current CLI surface | audited, partial | minimal | open gap | help capture, ready-endpoint tests, exit-code tests | medium | Rust parses `tunnel ready` and dispatches to stub; requires HIS metrics endpoint |
| CLI-018 | tunnel diag | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel diag` creates diagnostic report from local cloudflared instance | current CLI surface | audited, partial | minimal | open gap | help capture, diagnostic-output tests | medium | Rust parses `tunnel diag` and dispatches to stub; overlaps HIS diagnostics |
| CLI-019 | tunnel route | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel route` parent command with subcommands `dns`, `lb`, `ip`; `ip` has sub-subcommands `add`, `show`/`list`, `delete`, `get` | current CLI surface | audited, partial | minimal | open gap | help capture, per-subcommand tests | high | Rust parses nested route/dns/lb/ip/add/show/delete/get and dispatches to stubs |
| CLI-020 | tunnel vnet | `cmd/cloudflared/tunnel/vnets_subcommands.go` | `tunnel vnet` with subcommands `add` (with `--default`), `list`, `delete` (with `--force`), `update` (with `--name`, `--comment`) | current CLI surface | audited, partial | minimal | open gap | help capture, per-subcommand tests | medium | Rust parses vnet add/list/delete/update and dispatches to stubs |
| CLI-021 | tunnel ingress | `cmd/cloudflared/tunnel/ingress_subcommands.go` | `tunnel ingress` (hidden) with subcommands `validate` and `rule`; `validate` validates ingress from config; `rule URL` shows which rule matches | current CLI surface | audited, partial | minimal | open gap | help capture, validate/rule tests, hidden-command tests | medium | Rust parses ingress validate/rule and dispatches to stubs; hidden |

### Access, Tail, And Management Surface

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-022 | access subtree | `cmd/cloudflared/access/cmd.go` | `access` command (alias `forward`) with subcommands: `login` (browser auth), `curl` (JWT injection), `token` (JWT production), `tcp` (aliases `rdp`, `ssh`, `smb` for TCP/RDP/SSH/SMB proxy), `ssh-config` (print SSH config), `ssh-gen` (generate short-lived cert); `--fedramp` flag | current CLI surface | audited, partial | minimal | open gap | subtree help crawl, alias tests (`forward`), tcp-alias tests (`rdp`, `ssh`, `smb`), per-subcommand behavior tests | high | Rust parses all access subcommands and tcp aliases (rdp/ssh/smb); dispatches to stubs. See [docs/parity/cli/access-subtree.md](access-subtree.md) |
| CLI-023 | tail subtree | `cmd/cloudflared/tail/cmd.go` | `tail [TUNNEL-ID]` streams remote logs; flags: `--connector-id`, `--event` (filter: cloudflared/http/tcp/udp), `--level` (default `debug`), `--sample` (default 1.0), `--token` (env `TUNNEL_MANAGEMENT_TOKEN`), `--management-hostname` (hidden, default `management.argotunnel.com`), `--trace` (hidden); hidden subcommand `token` gets management JWT | current CLI surface | audited, partial | minimal | open gap | help crawl, filter tests, hidden `token` subcommand tests, output-format tests | high | Rust parses tail and hidden `token` subcommand; dispatches to stubs. CDC owns log-streaming contract. See [docs/parity/cli/tail-and-management.md](tail-and-management.md) |
| CLI-024 | management subtree | `cmd/cloudflared/management/cmd.go` | `management` (hidden, category `Management`) with hidden subcommand `token`; token subcommand requires `--resource` (values: `logs`, `admin`, `host_details`), `--origincert`, `--loglevel` | current CLI surface | audited, partial | minimal | open gap | hidden-command help capture, token invocation tests, resource-flag tests | medium | Rust parses management and hidden `token` subcommand; dispatches to stubs |

### Compatibility, Formatting, And Error Behavior

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-025 | compatibility: proxy-dns | `cmd/cloudflared/proxydns/cmd.go`, `cmd/cloudflared/tunnel/cmd.go` | top-level `proxy-dns` prints deprecation error with link to DNS-over-HTTPS alternative; `tunnel proxy-dns` shows error `dns-proxy feature is no longer supported since version 2026.2.0` | current CLI surface | audited, partial | minimal | open gap | placeholder failure tests, stderr snapshot, exit-code tests | high | Rust parses both `proxy-dns` and `tunnel proxy-dns` and dispatches to removal error messages |
| CLI-026 | compatibility: db-connect | `cmd/cloudflared/tunnel/cmd.go` | `tunnel db-connect` shows removed-command error via `cliutil.RemovedCommand("db-connect")` | current CLI surface | audited, partial | minimal | open gap | removed-command failure test, stderr snapshot, exit-code test | medium | Rust parses `tunnel db-connect` and dispatches to removed-command error |
| CLI-027 | compatibility: classic tunnels | `cmd/cloudflared/tunnel/cmd.go` | classic tunnel invocation paths produce error: `Classic tunnels have been deprecated, please use Named Tunnels` | current CLI surface | audited, partial | minimal | open gap | deprecation-error tests | medium | Rust parses classic tunnel token; deprecation message not yet wired |
| CLI-028 | compatibility: login at root | `cmd/cloudflared/main.go` | `login` is registered as a top-level command for backward compatibility (delegates to tunnel login); hidden when built as subcommand | current CLI surface | audited, partial | minimal | open gap | top-level login invocation test, help-visibility test | high | Rust parses top-level `login` and dispatches to stub |
| CLI-029 | help formatting contract | blackbox output | urfave/cli generates help with specific spacing, wrapping, headings, command ordering, category grouping; exact text is visible contract | current CLI surface | audited, partial | minimal | open gap | exact text snapshots, width-sensitive capture, no substring-only proofs | critical | current Rust help is custom-generated with different format than urfave/cli |
| CLI-030 | usage failure behavior | blackbox error output | unknown commands produce urfave/cli error text plus suggestions; bad flags produce flag-specific errors; exit code semantics from urfave/cli | current CLI surface | audited, partial | minimal | open gap | stderr/stdout capture, exit-code matrix, unknown-command tests, bad-flag tests | high | current Rust has usage failure logic with exit code 2, but output does not match urfave/cli format |

### Transitional Rust-Only Commands

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-031 | validate command (Rust-only) | no frozen baseline equivalent | `validate` is a transitional alpha command that resolves config, loads YAML, normalizes ingress, and reports startup readiness; not present in baseline top-level surface | current CLI surface | audited, intentional divergence | local tests | intentional divergence | divergence note, transitional command tests, retirement/rename tracking | medium | may become internal, renamed, or retired; not a parity target |
| CLI-032 | run command (Rust alpha) | partial overlap with frozen `tunnel` and `tunnel run` | current Rust `run` enters QUIC transport core + Pingora proxy seam; only partially overlaps frozen `tunnel` root runnable behavior and `tunnel run` named-tunnel flow; must not be treated as CLI parity | current runtime + current CLI surface | audited, partial | local tests | open gap | command contract tests, compare against frozen `tunnel` root and `tunnel run` behavior | critical | must be reconciled against upstream `tunnel` root and `tunnel run`, not treated as equivalent by name alone |

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

Parsed and dispatched commands: all 40+ baseline command paths including
all 9 top-level commands, all tunnel subcommands (with nested route/vnet/ingress),
access subcommands and aliases, tail/management (with hidden `token`),
service install/uninstall, and compatibility paths (proxy-dns, db-connect,
classic tunnels). All dispatch to stubs except `help`, `version`, `validate`
(transitional), and `run` (alpha-limited) which have behavioral implementations.

Parsed flags: `--config`, `--help`/`-h`, `--version`/`-V`, `--short`/`-s`,
40+ tunnel/access/service/management flags stored into `GlobalFlags`

Missing from baseline: behavioral implementation behind the parsed stubs,
env-var binding semantics, default-value wiring, exact help format matching

### Divergence records

**Root invocation (CLI-001):** Partially resolved. Rust now parses empty
invocation as `ServiceMode` and dispatches to a stub. The parsing-level
divergence (help vs service mode) is fixed. Full runtime service-mode
behavior (config watcher, AppManager, daemonic run) is not yet implemented.

**Version format (CLI-005):** Partially resolved. Rust supports `--short`/`-s`
and outputs version-only string via `render_short_version()`. Full version
format (`cloudflared version {Version} (built {BuildTime}{BuildTypeMsg})`)
still uses alpha placeholder format. Confirmed from unit tests.

**db-connect removal (CLI-026):** Go exits 255, not 1. Confirmed from blackbox.

**proxy-dns removal (CLI-025):** Error message includes full deprecation URL
(`https://developers.cloudflare.com/1.1.1.1/dns-over-https/cloudflared-proxy/`).
Confirmed from blackbox.

**forward alias (CLI-022):** `forward` produces identical output to `access --help`.
Confirmed from blackbox.

No CLI divergences are currently classified as intentional. All divergences
show `open gap` status except CLI-031 (`validate` command, `intentional
divergence` — transitional alpha command not in baseline).

Evidence harness: blackbox captures exist in [docs/parity/cli/captures/](captures/)
with 6 capture files covering root, tunnel, access, tail, management,
service, update, error, and compatibility surfaces plus current Rust
output for comparison.

### Gap ranking by priority

Critical gaps (behavioral implementation behind stubs):

- CLI-001: service mode dispatch (parsed, stubbed — needs real service-mode behavior)
- CLI-002: root help text (structurally present — needs exact format match)
- CLI-003: root global flags (parsed — need behavioral wiring)
- CLI-007: service install/uninstall (parsed — needs HIS implementation)
- CLI-008: tunnel root behavior (parsed — needs full dispatch)
- CLI-010: tunnel create (parsed — needs CDC implementation)
- CLI-012: tunnel run (parsed — needs CDC/HIS wiring)
- CLI-029: help formatting contract
- CLI-032: run command reconciliation

High gaps (behavioral implementation behind stubs):

- CLI-004: help command subcommand routing
- CLI-005: version full format (short mode resolved)
- CLI-006: update command (parsed — needs HIS updater)
- CLI-009: tunnel login (parsed — needs browser auth)
- CLI-011: tunnel list (parsed — needs CDC API)
- CLI-013: tunnel delete (parsed — needs CDC API)
- CLI-015: tunnel token (parsed — needs CDC API)
Medium gaps (all now parsed and dispatched to stubs):

- CLI-014: tunnel cleanup
- CLI-016: tunnel info
- CLI-017: tunnel ready (depends on HIS-024/025)
- CLI-018: tunnel diag (depends on HIS diagnostics)
- CLI-019: tunnel route (parsed with dns/lb/ip nesting)
- CLI-022: access subtree (all 6 subcommands + aliases parsed)
- CLI-023: tail subtree (hidden token subcommand parsed)
- CLI-025: proxy-dns compatibility (dispatch exists)
- CLI-028: login at root compatibility (parsed)
- CLI-030: usage failure behavior

## Scope Classification

Lane classification is recorded directly in this ledger for roadmap and promotion use.

All items not listed below are **lane-required** for the declared Linux
production-alpha lane.

### Deferred (lane-relevant, post-alpha)

- CLI-006: `update` command — requires external update infrastructure
- CLI-016: `tunnel info` — lower priority than core tunnel lifecycle commands
- CLI-017: `tunnel ready` — depends on local metrics endpoint (HIS-024/025)
- CLI-018: `tunnel diag` — diagnostics subsystem deferred as a unit
- CLI-021: `tunnel ingress` (hidden) — debug subcommand, low priority
- CLI-024: `management` subtree (hidden) — hidden admin tooling

### Compatibility-only (deprecated error stubs)

- CLI-025: `proxy-dns` removal — baseline prints deprecation error with URL
- CLI-026: `db-connect` removal — baseline shows removed-command error, exit 255
- CLI-027: classic tunnel deprecation — baseline shows deprecation error

These require exact error text, stderr placement, and exit code parity.
They do not require working implementations of the removed features.

## Immediate Work Queue

1. ~~create [docs/parity/cli/root-and-global-flags.md](root-and-global-flags.md)~~ — done
2. ~~create [docs/parity/cli/tunnel-subtree.md](tunnel-subtree.md)~~ — done
3. ~~create [docs/parity/cli/access-subtree.md](access-subtree.md)~~ — done
4. ~~create [docs/parity/cli/tail-and-management.md](tail-and-management.md)~~ — done
5. ~~capture frozen Go help output for all callable paths~~ — done;
   captures in [docs/parity/cli/captures/](captures/):
   - [root-surface.txt](captures/root-surface.txt) — root help, empty invocation, version
   - [tunnel-subtree.txt](captures/tunnel-subtree.txt) — tunnel and all tunnel subcommand help
   - [access-subtree.txt](captures/access-subtree.txt) — access subtree and forward alias
   - [tail-management-service-update.txt](captures/tail-management-service-update.txt) — tail, management, service, update
   - [error-and-compat.txt](captures/error-and-compat.txt) — unknown commands, bad flags, proxy-dns, db-connect
   - [rust-current-surface.txt](captures/rust-current-surface.txt) — current Rust binary outputs for comparison

### Remaining Work (Post-Audit Stages)

1. replace substring-only Rust CLI tests with snapshot-grade parity tests
   where a surface is implemented — owned by the current implementation milestones
2. root invocation divergence is now documented above and in captures
3. version format divergence is now documented above and in captures
