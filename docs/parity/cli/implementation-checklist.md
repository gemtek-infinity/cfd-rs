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
| CLI-001 | root invocation | `cmd/cloudflared/main.go` `action()` | empty invocation enters service mode via `handleServiceMode()`: creates config file watcher, initializes `FileManager`, creates `AppManager` and `AppService`, runs daemonically. Non-empty invocation without a recognized subcommand falls through to `tunnel.TunnelCommand(c)` (see CLI-008). Not help. | current CLI surface | audited, partial | partial | open gap | blackbox empty invocation capture, stdout and stderr capture, exit-code compare, service-mode behavior test, non-empty fallthrough test | critical | Rust parses empty invocation as `ServiceMode` with explicit match arm returning guidance to use `tunnel run --config` or `--token`; integration test verifies non-zero exit and stderr guidance; full service-mode runtime depends on HIS-041/042/043 (watcher/reload) |
| CLI-002 | root help text | root app `--help` output | root help exposes 9 top-level command families: `update`, `version`, `tunnel`, `login` (compat), `proxy-dns` (removed), `access` (alias `forward`), `tail`, `management` (hidden), `service` (Linux). Frozen wording, ordering, spacing from urfave/cli | current CLI surface | audited, parity-backed | baseline-backed tests | closed | exact help snapshot compare, top-level command inventory capture | critical | Rust help matches Go baseline `--help` output: NAME/USAGE/VERSION/DESCRIPTION/COMMANDS/GLOBAL OPTIONS/COPYRIGHT sections; COMMANDS section matches urfave/cli tabwriter alignment (column 22) with Go VisibleCategories sort (uncategorized first, then `Access:`, `Tunnel:` categories); GLOBAL OPTIONS section renders all 17 Go baseline app-level flags with urfave/cli tabwriter alignment (column 48, computed from `max(flag_name_with_indent) + padding`); `management` correctly hidden; `forward` alias shown; `COPYRIGHT` matches Go `app.Copyright`; env var `[$VAR]` format and `(default: ...)` annotations match Go flag String() output; 13 parity tests cover section presence, categories, command snapshot, global options alignment (column 48), flag count (17), hidden management, copyright, aliases, inventory, env vars; capture at [`docs/parity/cli/captures/rust-current-surface.txt`](captures/rust-current-surface.txt) |
| CLI-003 | root global flags | `cmd/cloudflared/tunnel/cmd.go` `Flags()` | 50+ global flags including: `--config`, `--credentials-file`/`-cred-file` (env `TUNNEL_CRED_FILE`), `--credentials-contents` (env `TUNNEL_CRED_CONTENTS`), `--token` (env `TUNNEL_TOKEN`), `--token-file` (env `TUNNEL_TOKEN_FILE`), `--origincert` (env `TUNNEL_ORIGIN_CERT`), `--loglevel` (env `TUNNEL_LOGLEVEL`, default `info`), `--transport-loglevel`/`--proto-loglevel` (hidden), `--logfile`, `--log-directory`, `--output` (json/default), `--trace-output`, `--edge` (hidden, env `TUNNEL_EDGE`), `--region` (env `TUNNEL_REGION`), `--edge-ip-version` (env `TUNNEL_EDGE_IP_VERSION`, default `4`), `--edge-bind-address`, `--metrics`, `--metrics-update-freq` (default 5s), `--protocol`/`-p` (hidden, env `TUNNEL_TRANSPORT_PROTOCOL`), `--post-quantum`/`-pq` (hidden, env `TUNNEL_POST_QUANTUM`), `--features`/`-F` (env `TUNNEL_FEATURES`), `--no-autoupdate`, `--autoupdate-freq`, `--name`/`-n` (env `TUNNEL_NAME`), `--hostname` (hidden), `--lb-pool`, `--url`, `--hello-world`, `--pidfile`, `--tag` (hidden), `--ha-connections` (hidden, default 4), `--retries` (default 5), `--max-edge-addr-retries` (hidden, default 8), `--rpc-timeout` (hidden, default 5s), `--grace-period` (default 30s), `--label`, `--max-active-flows`, `--management-hostname` (hidden, default `management.argotunnel.com`), `--service-op-ip` (hidden), `--version`/`-v`/`-V`, `--api-url` (hidden, default `https://api.cloudflare.com/client/v4`), `--is-autoupdated` (hidden), `--api-key`/`--api-email`/`--api-ca-key` (all hidden, deprecated), `--ui` (hidden, deprecated), plus proxy-origin flags (`--unix-socket`, `--http-host-header`, `--origin-server-name`, `--origin-ca-pool`, `--no-tls-verify`, `--no-chunked-encoding`, `--http2-origin`), plus ICMP flags (`--icmpv4-src`, `--icmpv6-src`), plus proxy-dns flags (removed feature) | current CLI surface | audited, parity-backed | baseline-backed tests | closed | flag inventory capture, env-binding tests, default-value tests, hidden-flag tests, alias tests | critical | Rust parses 40+ flags into `GlobalFlags` struct with full inventory: 13 boolean + 63 value flags verified in `all_boolean_flags_parse_without_error` and `all_value_flags_parse_without_error`; `apply_env_defaults()` maps 50+ env vars via injectable reader matching Go `EnvVars` bindings with precedence chain CLI > env > defaults (37 env_defaults tests); `apply_defaults()` fills 16 baseline constants verified in `apply_defaults_fills_baseline_values`; multi-env first-match for `TUNNEL_PROTO_LOGLEVEL`/`TUNNEL_TRANSPORT_LOGLEVEL` and `TUNNEL_MANAGEMENT_OUTPUT`/`TUNNEL_LOG_OUTPUT`; CSV-split for `TUNNEL_EDGE`/`TUNNEL_TAG`; `parse_go_bool()` matches Go `strconv.ParseBool` rules; alias tests verify `--cred-file`, `-n`, `-p`, `-F`, `--cacert`, `-pq`; hidden-flag parity via exact help snapshot (17 visible `GLOBAL_FLAGS` entries matching Go baseline, all hidden flags excluded); full precedence chain tested (CLI > env > defaults); `url` intentionally not defaulted (dispatch checks `is_some()` matching Go `c.IsSet("url")`); logging runtime behavior deferred to HIS-063 through HIS-068 |
| CLI-004 | help command behavior | root help command | explicit `help` command and `--help`/`-h` flag routing for root and subcommands; urfave/cli generates command-local help automatically | current CLI surface | audited, parity-backed | local tests | closed | help-command snapshot tests, subcommand help-routing tests, exit-code tests | high | Rust has `help`/`--help`/`-h` with exit code 0; root help output matches Go baseline format with all sections and GLOBAL OPTIONS alignment; subcommand help routing (`cloudflared tunnel --help`, `cloudflared access --help`, `cloudflared help tunnel`, `cloudflared help access`) implemented with `HelpTarget` enum; `render_tunnel_help_text()` and `render_access_help_text()` render NAME, USAGE, DESCRIPTION, COMMANDS sections matching Go `SubcommandHelpTemplate`; 9 parser routing tests and 10 content tests cover all help paths |
| CLI-005 | version command | `cmd/cloudflared/main.go` app version config | format: `{Version} (built {BuildTime}{BuildTypeMsg})`; `--short`/`-s` flag outputs version number only; `--version`/`-v`/`-V` flags also trigger version output | current CLI surface | audited, parity-backed | local tests | closed | exact stdout snapshot compare, `--short`/`-s` flag tests, exit-code tests | high | Rust version output now matches Go baseline format `cloudflared version {version} (built {build_time}{build_type_msg})`; `BUILD_TIME` from `option_env!("CFDRS_BUILD_TIME")` with `"unknown"` fallback; `BUILD_TYPE` from `option_env!("CFDRS_BUILD_TYPE")` with `""` fallback; `build_type_msg()` returns `" with {BUILD_TYPE}"` or empty, matching Go `GetBuildTypeMsg()`; 12 parity tests cover format, build-time injection, short flag, build-type suffix, constants, and integration version compare |
| CLI-006 | update command | `cmd/cloudflared/updater/update.go` | `update` command with flags: `--beta`, `--force` (hidden), `--staging` (hidden), `--version`; returns exit code 11 if update occurred, exit code 10 on error, exit 0 if no update needed | current CLI surface | audited, partial | local tests | open gap | help capture, update-behavior tests, exit-code tests (exit 11 on success, exit 10 on error) | high | Rust parses and dispatches to stub; update logic not implemented; 1 parse-dispatch test in `top_level_commands` |
| CLI-007 | service command | `cmd/cloudflared/linux_service.go` | `service` command with subcommands `install` and `uninstall`; flag `--no-update-service` (default false); systemd: creates `/etc/systemd/system/cloudflared.service`, `/etc/systemd/system/cloudflared-update.service`, `/etc/systemd/system/cloudflared-update.timer`; SysV fallback: `/etc/init.d/cloudflared` | current CLI surface | audited, parity-backed | baseline-backed tests | closed | help capture, service install/uninstall tests, generated-asset tests, exit-code tests | critical | CLI dispatches `Service(Install/Uninstall)` to real `install_linux_service()`/`uninstall_linux_service()` (not stubs) with `ProcessRunner`; HIS wires full service pipeline: `CommandRunner` trait, systemd unit/timer templates, `--no-update-service` → `auto_update`, token-vs-config resolution in `build_service_install_request()`; 3 parse tests (`service_install`, `service_uninstall`, `no_update_service_flag`) + 13 HIS service tests (install/uninstall sequences, template rendering, timer skip, config args); root help shows `service` with correct usage text; end-to-end host `systemctl` verification deferred to HIS-015, SysV fallback to HIS-016 |

### Tunnel Command Surface

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-008 | tunnel root behavior | `cmd/cloudflared/tunnel/cmd.go` | `tunnel` is both a command namespace and a multi-branch runnable decision surface; `tunnel` with a recognized subcommand dispatches to it. `tunnel` without a subcommand invokes `TunnelCommand()` which is a 5-branch decision: (1) `--name` set invokes `runAdhocNamedTunnel()` (create+route+run), (2) `--url` or `--hello-world` with `--quick-service` invokes `RunQuickTunnel()`, (3) config has `TunnelID` produces error directing user to `tunnel run`, (4) `--hostname` set returns `errDeprecatedClassicTunnel`, (5) fallthrough returns `tunnelCmdErrorMessage` error; category `Tunnel`; usage text `Use Cloudflare Tunnel to expose private services to the Internet or to Cloudflare connected private users.` | current CLI surface | audited, parity-backed | baseline-backed tests | closed | blackbox tunnel invocation matrix (all 5 branches), stdout/stderr capture, exit-code tests | critical | Rust `execute_tunnel_bare()` implements the 5-branch Go `TunnelCommand()` dispatch: (1) `--name` → adhoc stub, (2) `--url`/`--hello-world` → quick-tunnel stub, (3) config TunnelID → `execute_runtime_command()`, (4) `--hostname` → `CLASSIC_TUNNEL_DEPRECATED_MSG` error, (5) fallthrough → `TUNNEL_CMD_ERROR_MSG` error; 6 integration tests cover all 5 branches: `tunnel_bare_with_name_flag_returns_stub` (branch 1), `tunnel_bare_with_url_flag_returns_stub` (branch 2), `tunnel_bare_with_config_tunnel_id_reaches_runtime` (branch 3), `tunnel_bare_hostname_deprecated_error` (branch 4), `tunnel_bare_fallthrough_error` (branch 5), and `run_and_tunnel_run_produce_same_exit` routing equivalence; branches 3/4/5 have exact error message and exit code parity with Go baseline; branches 1/2 dispatch layer is complete — actual create+route+run behavior deferred to CDC-033/034 |
| CLI-009 | tunnel login | `cmd/cloudflared/tunnel/login.go` | `tunnel login` generates cert via browser auth; also exposed as top-level `login` for backward compat (hidden at top level when built as subcommand); `--fedramp`/`-f` flag for FedRAMP support | current CLI surface | audited, partial | local tests | open gap | help capture, login-flow tests (browser auth is external), flag tests | high | Rust parses `tunnel login` and top-level `login`; auth flow not implemented; 1 parse-dispatch test (`tunnel_login_subcommand`) |
| CLI-010 | tunnel create | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel create NAME` creates a new tunnel; produces tunnel UUID and credentials file | current CLI surface | audited, partial | local tests | open gap | help capture, creation-flow tests, output-format tests | critical | Rust parses `tunnel create` and dispatches to stub; 1 parse-dispatch test (`tunnel_create_subcommand`) |
| CLI-011 | tunnel list | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel list` lists existing tunnels; supports filtering and sorting flags | current CLI surface | audited, partial | local tests | open gap | help capture, list-output tests, filter-flag tests | high | Rust parses `tunnel list` and dispatches to stub; 1 parse-dispatch test (`tunnel_list_subcommand`) |
| CLI-012 | tunnel run | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel run [TUNNEL]` proxies local web server by running the given tunnel; named-tunnel flow requires credentials | current CLI surface | audited, partial | local tests | open gap | help capture, run invocation matrix, credential-resolution tests | critical | Rust parses `tunnel run` with credential flags and enters runtime shell; 2 parse-dispatch tests (`bare_run_is_tunnel_run`, `tunnel_run_subcommand`) |
| CLI-013 | tunnel delete | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel delete TUNNEL` deletes existing tunnel by UUID or name | current CLI surface | audited, partial | local tests | open gap | help capture, delete-flow tests | high | Rust parses `tunnel delete` and dispatches to stub; 1 parse-dispatch test (`tunnel_delete_subcommand`) |
| CLI-014 | tunnel cleanup | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel cleanup TUNNEL` cleans up tunnel connections; `--connector-id` flag to filter | current CLI surface | audited, partial | local tests | open gap | help capture, cleanup tests | medium | Rust parses `tunnel cleanup` and dispatches to stub; 1 parse-dispatch test (`tunnel_cleanup_subcommand`) |
| CLI-015 | tunnel token | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel token TUNNEL` fetches credential token for existing tunnel by name or UUID | current CLI surface | audited, partial | local tests | open gap | help capture, token-output tests | high | Rust parses `tunnel token` and dispatches to stub; 1 parse-dispatch test (`tunnel_token_subcommand`) |
| CLI-016 | tunnel info | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel info TUNNEL` lists details about active connectors | current CLI surface | audited, partial | local tests | open gap | help capture, info-output tests | medium | Rust parses `tunnel info` and dispatches to stub; 1 parse-dispatch test (`tunnel_info_subcommand`) |
| CLI-017 | tunnel ready | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel ready` calls `/ready` endpoint; requires `--metrics` flag; returns proper exit code | current CLI surface | audited, partial | local tests | open gap | help capture, ready-endpoint tests, exit-code tests | medium | Rust parses `tunnel ready` and dispatches to stub; requires HIS metrics endpoint; 1 parse-dispatch test (`tunnel_ready_subcommand`) |
| CLI-018 | tunnel diag | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel diag` creates diagnostic report from local cloudflared instance | current CLI surface | audited, partial | local tests | open gap | help capture, diagnostic-output tests | medium | Rust parses `tunnel diag` and dispatches to stub; overlaps HIS diagnostics; 1 parse-dispatch test (`tunnel_diag_subcommand`) |
| CLI-019 | tunnel route | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel route` parent command with subcommands `dns`, `lb`, `ip`; `ip` has sub-subcommands `add`, `show`/`list`, `delete`, `get` | current CLI surface | audited, partial | local tests | open gap | help capture, per-subcommand tests | high | Rust parses nested route/dns/lb/ip/add/show/delete/get and dispatches to stubs; 4 parse-dispatch tests (`tunnel_route_bare`, `tunnel_route_dns`, `tunnel_route_ip_add`, `tunnel_route_ip_show`) |
| CLI-020 | tunnel vnet | `cmd/cloudflared/tunnel/vnets_subcommands.go` | `tunnel vnet` with subcommands `add` (with `--default`), `list`, `delete` (with `--force`), `update` (with `--name`, `--comment`) | current CLI surface | audited, partial | local tests | open gap | help capture, per-subcommand tests | medium | Rust parses vnet add/list/delete/update and dispatches to stubs; 3 parse-dispatch tests (`tunnel_vnet_bare`, `tunnel_vnet_add`, `tunnel_vnet_list`) |
| CLI-021 | tunnel ingress | `cmd/cloudflared/tunnel/ingress_subcommands.go` | `tunnel ingress` (hidden) with subcommands `validate` and `rule`; `validate` validates ingress from config; `rule URL` shows which rule matches | current CLI surface | audited, partial | local tests | open gap | help capture, validate/rule tests, hidden-command tests | medium | Rust parses ingress validate/rule and dispatches to stubs; hidden; 3 parse-dispatch tests (`tunnel_ingress_bare`, `tunnel_ingress_validate`, `tunnel_ingress_rule`) |

### Access, Tail, And Management Surface

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-022 | access subtree | `cmd/cloudflared/access/cmd.go` | `access` command (alias `forward`) with subcommands: `login` (browser auth), `curl` (JWT injection), `token` (JWT production), `tcp` (aliases `rdp`, `ssh`, `smb` for TCP/RDP/SSH/SMB proxy), `ssh-config` (print SSH config), `ssh-gen` (generate short-lived cert); `--fedramp` flag | current CLI surface | audited, partial | local tests | open gap | subtree help crawl, alias tests (`forward`), tcp-alias tests (`rdp`, `ssh`, `smb`), per-subcommand behavior tests | high | Rust parses all access subcommands and tcp aliases (rdp/ssh/smb); dispatches to stubs; 6 parse-dispatch tests (`access_bare`, `access_login`, `access_tcp`, `access_rdp_alias`, `access_ssh_config`, `forward_alias_is_access`). See [docs/parity/cli/access-subtree.md](access-subtree.md) |
| CLI-023 | tail subtree | `cmd/cloudflared/tail/cmd.go` | `tail [TUNNEL-ID]` streams remote logs; flags: `--connector-id`, `--event` (filter: cloudflared/http/tcp/udp), `--level` (default `debug`), `--sample` (default 1.0), `--token` (env `TUNNEL_MANAGEMENT_TOKEN`), `--management-hostname` (hidden, default `management.argotunnel.com`), `--trace` (hidden), `--loglevel` (default `info`, env `TUNNEL_LOGLEVEL`), `--origincert` (env `TUNNEL_ORIGIN_CERT`), `--output` (json/default); hidden subcommand `token` gets management JWT | current CLI surface | audited, partial | local tests | open gap | help crawl, filter tests, hidden `token` subcommand tests, output-format tests | high | Rust parses tail and hidden `token` subcommand; dispatches to stubs; 2 parse-dispatch tests (`tail_bare`, `tail_token`). CDC owns log-streaming contract. See [docs/parity/cli/tail-and-management.md](tail-and-management.md) |
| CLI-024 | management subtree | `cmd/cloudflared/management/cmd.go` | `management` (hidden, category `Management`) with hidden subcommand `token`; token subcommand requires `--resource` (values: `logs`, `admin`, `host_details`), `--origincert`, `--loglevel`, `--output` (json/default) | current CLI surface | audited, partial | local tests | open gap | hidden-command help capture, token invocation tests, resource-flag tests | medium | Rust parses management and hidden `token` subcommand; dispatches to stubs; 2 parse-dispatch tests (`management_bare`, `management_token`) |

### Compatibility, Formatting, And Error Behavior

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-025 | compatibility: proxy-dns | `cmd/cloudflared/proxydns/cmd.go`, `cmd/cloudflared/tunnel/cmd.go` | both top-level `proxy-dns` and `tunnel proxy-dns` use the same function; returned error is `dns-proxy feature is no longer supported` (no version suffix); a separate log message includes the version and deprecation URL but that is log output only, not the error value; both paths produce identical behavior | current CLI surface | audited, parity-backed | baseline-backed tests | closed | placeholder failure tests, stderr snapshot, exit-code tests, exact error text compare | high | Rust dispatches both `proxy-dns` and `tunnel proxy-dns` to `PROXY_DNS_REMOVED_MSG` matching Go exact text; 4 parity tests verify both paths, exact error constant, and log message; `PROXY_DNS_REMOVED_LOG_MSG` emits the Go `log.Error()` version+URL via `eprintln!` before returning the error; exit code 1 matches Go baseline |
| CLI-026 | compatibility: db-connect | `cmd/cloudflared/tunnel/cmd.go` | `tunnel db-connect` shows removed-command error via `cliutil.RemovedCommand("db-connect")` | current CLI surface | audited, parity-backed | baseline-backed tests | closed | removed-command failure test, stderr snapshot, exit-code test | medium | Rust `DB_CONNECT_REMOVED_MSG` matches Go baseline text from `cliutil.RemovedCommand()`; exit code 255 matches Go `cli.Exit(-1)` (unsigned byte truncation); integration test verifies exact exit code and error text |
| CLI-027 | compatibility: classic tunnels | `cmd/cloudflared/tunnel/cmd.go` | classic tunnel invocation paths produce error: `Classic tunnels have been deprecated, please use Named Tunnels. (https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/install-and-setup/tunnel-guide/)`; in `tunnel run` context, `--hostname` set emits warning: `The property 'hostname' in your configuration is ignored because you configured a Named Tunnel...` | current CLI surface | audited, parity-backed | baseline-backed tests | closed | deprecation-error tests, exact error text compare, hostname warning | medium | `CLASSIC_TUNNEL_DEPRECATED_MSG` constant matches Go `errDeprecatedClassicTunnel` exactly; dispatch branch 4 in `execute_tunnel_bare()` returns the error when `--hostname` is set; `TUNNEL_RUN_HOSTNAME_WARNING_MSG` matches Go `sc.log.Warn()` in `runCommand()` and emits via `tracing::warn!` in `execute_runtime_command()`; exit code 1 matches Go `return err` behavior |
| CLI-028 | compatibility: login at root | `cmd/cloudflared/main.go` | `login` is registered as a top-level command for backward compatibility (delegates to tunnel login); hidden when built as subcommand | current CLI surface | audited, parity-backed | baseline-backed tests | closed | top-level login invocation test, help-visibility test | high | Rust parses `login` as `Command::Login` (compat alias for `tunnel login`); 3 parse tests (`top_level_commands`, `tunnel_login_subcommand`, `login_and_tunnel_login_produce_same_dispatch`) verify parse-layer routing; 3 integration tests verify: (1) `login_at_root_is_recognized` — recognized as valid command, (2) `login_hidden_from_root_help` — not shown in root COMMANDS section matching Go `Hidden: true`, (3) `login_and_tunnel_login_both_dispatch` — both paths dispatch without unknown-command error; auth flow implementation deferred (not a CLI surface concern) |
| CLI-029 | help formatting contract | blackbox output | urfave/cli generates help with specific spacing, wrapping, headings, command ordering, category grouping; exact text is visible contract | current CLI surface | audited, parity-backed | baseline-backed tests | closed | exact text snapshots, width-sensitive capture, no substring-only proofs | critical | Rust help matches urfave/cli tabwriter alignment: COMMANDS at column 22 (computed from `max(name_with_indent) + padding`), GLOBAL OPTIONS at column 48 (computed from longest flag `--credentials-file value, --cred-file value` = 43 chars + 3 indent + 2 padding = 48); command ordering matches Go VisibleCategories sort (uncategorized first, then lexicographic categories); `management` correctly hidden per Go `Hidden: true`; 6 snapshot/alignment/hidden/count tests lock in format; [`docs/parity/cli/captures/rust-current-surface.txt`](captures/rust-current-surface.txt) updated with current output |
| CLI-030 | usage failure behavior | blackbox error output | unknown commands produce urfave/cli error text plus suggestions; bad flags produce flag-specific errors; exit code semantics from urfave/cli | current CLI surface | audited, parity-backed | baseline-backed tests | closed | stderr/stdout capture, exit-code matrix, unknown-command tests, bad-flag tests | high | exit code 2 for usage failures (POSIX convention), exit code 1 for config errors, exit code 0 for success; Go baseline exits 0 for flag/parse errors (discards error in `runApp`) and writes to stdout — Rust deliberately uses exit 2 to stderr following POSIX convention; 7 parity tests verify exit codes, stderr/stdout placement, unknown flag text, unknown command text; Go `"Incorrect Usage."` prefix replaced with `"error:"` prefix; unknown commands rejected explicitly rather than falling through to tunnel handler |

### Transitional Rust-Only Commands

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-031 | validate command (Rust-only) | no frozen baseline equivalent | `validate` is a transitional alpha command that resolves config, loads YAML, normalizes ingress, and reports startup readiness; not present in baseline top-level surface | current CLI surface | audited, intentional divergence | local tests | intentional divergence | divergence note, transitional command tests, retirement/rename tracking | medium | may become internal, renamed, or retired; not a parity target |
| CLI-032 | run command (Rust alpha) | partial overlap with frozen `tunnel` and `tunnel run` | current Rust `run` enters QUIC transport core + Pingora proxy seam; only partially overlaps frozen `tunnel` root runnable behavior and `tunnel run` named-tunnel flow; must not be treated as CLI parity | current runtime + current CLI surface | audited, partial | local tests | open gap | command contract tests, compare against frozen `tunnel` root and `tunnel run` behavior | critical | Integration test confirms `run` and `tunnel run` both reach same runtime path (`runtime-owner: initialized`) with same exit code. Parse-layer tests: `--token-file` flag parsing, `--token` + `--token-file` coexistence for precedence, positional tunnel name capture, `--hostname` with tunnel run, bare `run --token` equivalence with `tunnel run --token`, multiple positional args collected for NArg validation. NArg validation: `execute_tunnel_run()` rejects >1 positional args with exact Go baseline error message and exit code 255. Parity constants: `TUNNEL_RUN_NARG_ERROR_MSG`, `TUNNEL_TOKEN_INVALID_MSG`, `TUNNEL_TOKEN_FILE_READ_ERROR_PREFIX`, `TUNNEL_RUN_IDENTITY_ERROR_MSG` match Go `cliutil.UsageError()` messages; `tunnel_run_usage_error()` helper appends Go `WithErrorHandler` suffix. 3 NArg integration tests verify rejection, single-arg acceptance, and zero-arg acceptance. Known remaining gaps: runtime `--token` > `--token-file` precedence dispatch (requires startup path restructuring for token-only mode), invalid-token error, no-identity usage error. |

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
exact help format matching

### Divergence records

**Root invocation (CLI-001):** Partially resolved. Rust now parses empty
invocation as `ServiceMode` and dispatches to a stub. The parsing-level
divergence (help vs service mode) is fixed. Full runtime service-mode
behavior (config watcher, AppManager, daemonic run) is not yet implemented.

**Version format (CLI-005):** Resolved. Rust version output matches Go
baseline format `cloudflared version {version} (built {build_time}{build_type_msg})`;
`BUILD_TIME` from `option_env!("CFDRS_BUILD_TIME")` with `"unknown"` fallback;
`build_type_msg()` returns `" with {BUILD_TYPE}"` or empty, matching Go
`GetBuildTypeMsg()`. 12 parity tests confirm format, build-time injection,
short flag, build-type suffix, constants, and integration version compare.

**db-connect removal (CLI-026):** Resolved. Rust now exits 255 matching Go `cli.Exit(-1)`. Integration test verifies exact exit code.

**proxy-dns removal (CLI-025):** Resolved. Error text matches Go exactly. `PROXY_DNS_REMOVED_LOG_MSG`
now emits the Go `log.Error()` version+URL message via `eprintln!` before returning.
Confirmed from baseline source and integration tests.

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
- CLI-010: tunnel create (parsed — needs CDC implementation)
- CLI-012: tunnel run (parsed — needs CDC/HIS wiring)
- CLI-032: run command reconciliation

High gaps (behavioral implementation behind stubs):

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
- CLI-025: proxy-dns compatibility (closed)

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

- CLI-025: `proxy-dns` removal — closed; error text, log message, and exit code 1 match baseline
- CLI-026: `db-connect` removal — closed; error text and exit code 255 match baseline
- CLI-027: classic tunnel deprecation — closed; error text and exit code 1 match baseline; hostname warning in tunnel run context matches Go `sc.log.Warn()`

All three compatibility stubs now have exact error text, stderr placement,
and exit code parity with the Go baseline.

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
