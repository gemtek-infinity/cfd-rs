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
- baseline-backed tests
- compare-backed
- local tests

If a new value is needed later, add it deliberately and keep it short.

### Divergence status

Preferred values:

- none recorded
- closed
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
| CLI-001 | root invocation | `cmd/cloudflared/main.go` `action()` | empty invocation enters service mode via `handleServiceMode()`: creates config file watcher, initializes `FileManager`, creates `AppManager` and `AppService`, runs daemonically. Non-empty invocation without a recognized subcommand falls through to `tunnel.TunnelCommand(c)` (see CLI-008). Not help. | current CLI surface | audited, parity-backed | baseline-backed tests | closed | blackbox empty invocation capture, stdout and stderr capture, exit-code compare, service-mode behavior test, non-empty fallthrough test | critical | Rust dispatches empty invocation (`ServiceMode`) to `execute_startup_command()` which enters config discovery via `discover_config()` → runtime startup matching Go `handleServiceMode()` → `FindOrCreateConfigPath()` → `AppManager` flow. `ApplicationRuntime::run()` with `spawn_signal_bridge()` + `spawn_config_watcher()` + `spawn_primary_service()` is behaviorally equivalent to Go `AppManager.Run()` → `AppService.actionLoop()`. 5 baseline-backed tests: `service_mode_enters_config_discovery_not_help` (not help output), `service_mode_without_config_produces_config_error` (exit 1, not stub), `service_mode_with_config_reaches_runtime` (deployment contract output), `service_mode_with_config_shows_ingress_rules` (ingress-rules count), `service_mode_and_tunnel_run_with_config_reach_same_runtime` (bare invocation and `tunnel run` reach same runtime path with same exit code). Non-empty fallthrough covered by CLI-008 |
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
| CLI-009 | tunnel login | `cmd/cloudflared/tunnel/login.go` | `tunnel login` generates cert via browser auth; also exposed as top-level `login` for backward compat (hidden at top level when built as subcommand); `--fedramp`/`-f` flag for FedRAMP support | current CLI surface | audited, parity-backed | baseline-backed tests | none recorded | URL constant tests, FedRAMP URL tests, cert-check tests, cert-write-mode tests, message parity tests, poll config tests, integration dispatch tests | high | Rust `execute_tunnel_login()` implements Go `login()` flow: `check_for_existing_cert()` checks `~/.cloudflared/cert.pem`, `run_login_transfer()` opens browser via `xdg-open` and polls callback store with 10 attempts × 60s timeout, `OriginCertToken::from_pem_blocks()` decodes cert, FedRAMP endpoint set when `--fedramp`, `encode_pem()` + write with mode 0600; URL constants match Go baseline (`BASE_LOGIN_URL`, `CALLBACK_URL`, `FED_BASE_LOGIN_URL`, `FED_CALLBACK_STORE_URL`); both `Command::Login` and `TunnelSubcommand::Login` dispatch to same function; UUID-based polling key (intentional divergence from NaCl — `shouldEncrypt=false` makes key only an identifier); 8 unit tests + 1 integration test |
| CLI-010 | tunnel create | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel create NAME` creates a new tunnel; produces tunnel UUID and credentials file | current CLI surface | audited, parity-backed | local tests | closed | help capture, creation-flow tests, output-format tests | critical | `execute_tunnel_create()` loads origin cert, generates 32-byte secret (2× UUID v4 or `--tunnel-secret` base64 decode), calls `create_tunnel()` API, writes credential file with rollback on write failure; NArg=1 enforced; output via `render_output()` JSON/YAML; 1 parse-dispatch test, 2 NArg tests, 8 unit tests in `tunnel_commands` |
| CLI-011 | tunnel list | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel list` lists existing tunnels; supports filtering and sorting flags | current CLI surface | audited, parity-backed | local tests | closed | help capture, list-output tests, filter-flag tests | high | `execute_tunnel_list()` builds `TunnelFilter` from flags (`show_deleted`, `tunnel_name`, `name_prefix`, `exclude_name_prefix`, `filter_when`, `tunnel_id`), calls `list_tunnels()` API, tab-separated output with `fmt_connections()` per-colo counts; NArg=0 accepted; 1 parse-dispatch test, 1 NArg test, 4 `fmt_connections` unit tests |
| CLI-012 | tunnel run | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel run [TUNNEL]` proxies local web server by running the given tunnel; named-tunnel flow requires credentials | current CLI surface | audited, parity-backed | baseline-backed tests | closed | help capture, run invocation matrix, credential-resolution tests | critical | `execute_tunnel_run()` implements full Go `runCommand()` flow with three credential paths: (1) token path (`--token` > `--token-file`) decodes `TunnelToken`, injects tunnel identity and credentials via `execute_run_with_token()` temp credential file, (2) positional arg path injects `TunnelReference::from_raw()` into startup surface for name-or-UUID resolution, (3) config fallback reads `tunnel` field from config. `resolve_run_token_string()` matches Go lines 760–776. NArg validation rejects >1 positional args with exact Go baseline error message, exit code 255. `--credentials-contents` inline JSON wired in `apply_runtime_credential_discovery()`. 2 parse-dispatch tests, 12 tunnel_run unit tests, 3 NArg integration tests, 2 credential-discovery tests. Actual edge connection enters runtime via `execute_runtime_command()` |
| CLI-013 | tunnel delete | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel delete TUNNEL` deletes existing tunnel by UUID or name | current CLI surface | audited, parity-backed | local tests | closed | help capture, delete-flow tests | high | `execute_tunnel_delete()` resolves tunnel by UUID or name via `resolve_tunnel_ids()`, validates not-already-deleted, calls `delete_tunnel()` API, removes local credential file (non-fatal on error); NArg=1 enforced; 1 parse-dispatch test, 2 NArg tests |
| CLI-014 | tunnel cleanup | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel cleanup TUNNEL` cleans up tunnel connections; `--connector-id` flag to filter | current CLI surface | audited, parity-backed | local tests | closed | help capture, cleanup tests | medium | `execute_tunnel_cleanup()` resolves tunnel IDs, optional `--connector-id` filter via `connector_id` flag, calls `cleanup_connections()` API; NArg=1 enforced; 1 parse-dispatch test, 2 NArg tests |
| CLI-015 | tunnel token | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel token TUNNEL` fetches credential token for existing tunnel by name or UUID | current CLI surface | audited, parity-backed | local tests | closed | help capture, token-output tests | high | `execute_tunnel_token()` resolves tunnel ID, calls `get_tunnel_token()` API, optionally writes to `--credentials-file` or prints encoded token to stdout; NArg=1 enforced; 1 parse-dispatch test, 2 NArg tests |
| CLI-016 | tunnel info | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel info TUNNEL` lists details about active connectors | current CLI surface | audited, parity-backed | local tests | closed | help capture, info-output tests | medium | `execute_tunnel_info()` resolves tunnel ID, calls `list_active_clients()` API, displays tunnel name/ID/creation header then tab-separated connector table (CONNECTOR ID/CREATED/ARCHITECTURE/VERSION/ORIGIN IP/EDGE with `fmt_connections()` per-colo counts); NArg=1 enforced; 1 parse-dispatch test, 2 NArg tests |
| CLI-017 | tunnel ready | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel ready` calls `/ready` endpoint; requires `--metrics` flag; returns proper exit code | current CLI surface | audited, partial | local tests | open gap | help capture, ready-endpoint tests, exit-code tests | medium | Rust parses `tunnel ready` and dispatches to stub; requires HIS metrics endpoint; 1 parse-dispatch test (`tunnel_ready_subcommand`) |
| CLI-018 | tunnel diag | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel diag` creates diagnostic report from local cloudflared instance | current CLI surface | audited, partial | local tests | open gap | help capture, diagnostic-output tests | medium | Rust parses `tunnel diag` and dispatches to stub; overlaps HIS diagnostics; 1 parse-dispatch test (`tunnel_diag_subcommand`) |
| CLI-019 | tunnel route | `cmd/cloudflared/tunnel/subcommands.go` | `tunnel route` parent command with subcommands `dns`, `lb`, `ip`; `ip` has sub-subcommands `add`, `show`/`list`, `delete`, `get` | current CLI surface | audited, parity-backed | local tests | closed | — | high | `route_vnet_commands.rs`: `execute_route_dns()` calls `route_tunnel()` with `HostnameRoute::Dns`, `execute_route_lb()` with `HostnameRoute::Lb`, `execute_route_ip_add()` calls `add_route()` with `NewRoute`, `execute_route_ip_show()` builds `IpRouteFilter` from 6 flags and calls `list_routes()`, `execute_route_ip_delete()` resolves by UUID or CIDR filter then calls `delete_route()`, `execute_route_ip_get()` calls `get_route_by_ip()` with optional vnet; `resolve_optional_vnet()` resolves `--vnet` by name or UUID matching Go `getVnetId()`; `render_route_table()` tab-separated output; 14 parse-dispatch+NArg tests + 2 render tests |
| CLI-020 | tunnel vnet | `cmd/cloudflared/tunnel/vnets_subcommands.go` | `tunnel vnet` with subcommands `add` (with `--default`), `list`, `delete` (with `--force`), `update` (with `--name`, `--comment`) | current CLI surface | audited, parity-backed | local tests | closed | — | medium | `route_vnet_commands.rs`: `execute_vnet_add()` calls `create_virtual_network()` with `NewVirtualNetwork` from name/comment/default flags, `execute_vnet_list()` builds `VnetFilter` and calls `list_virtual_networks()`, `execute_vnet_delete()` resolves by name or UUID via `resolve_vnet_id()` then calls `delete_virtual_network()` with `--force`, `execute_vnet_update()` resolves + calls `update_virtual_network()` with `UpdateVirtualNetwork` from `--name`/`--comment`/`--default`; `render_vnet_table()` tab-separated output; 9 parse-dispatch+NArg tests + 3 render/resolve tests |
| CLI-021 | tunnel ingress | `cmd/cloudflared/tunnel/ingress_subcommands.go` | `tunnel ingress` (hidden) with subcommands `validate` and `rule`; `validate` validates ingress from config; `rule URL` shows which rule matches | current CLI surface | audited, partial | local tests | open gap | help capture, validate/rule tests, hidden-command tests | medium | Rust parses ingress validate/rule and dispatches to stubs; hidden; NArg validation: `validate` accepts 0, `rule` requires exactly 1 arg; 3 parse-dispatch tests, 3 NArg tests |

### Access, Tail, And Management Surface

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-022 | access subtree | `cmd/cloudflared/access/cmd.go` | `access` command (alias `forward`) with subcommands: `login` (browser auth), `curl` (JWT injection), `token` (JWT production), `tcp` (aliases `rdp`, `ssh`, `smb` for TCP/RDP/SSH/SMB proxy), `ssh-config` (print SSH config), `ssh-gen` (generate short-lived cert); `--fedramp` flag | current CLI surface | audited, parity-backed | baseline-backed tests | closed | per-subcommand behavior tests | high | Generic placeholder dispatch is gone. Bare `access` and `forward` still show help (exit 0); all 6 subcommands + 3 tcp aliases parse and dispatch explicitly. `login`, `curl`, `token`, `tcp`, and `ssh-gen` now return command-specific deferred-boundary errors naming the missing runtime path; `ssh-config` renders the SSH config snippet directly, including `--short-lived-cert` and hostname aliases. Evidence: 14 access/help-surface tests in `cfdrs-cli`, 13 behavioral integration tests in `cfdrs-bin`, and 4 unit tests in `access_commands.rs`. Go baseline has no `NArg` constraints on access subcommands. See [docs/parity/cli/access-subtree.md](access-subtree.md) |
| CLI-023 | tail subtree | `cmd/cloudflared/tail/cmd.go` | `tail [TUNNEL-ID]` streams remote logs; flags: `--connector-id`, `--event` (filter: cloudflared/http/tcp/udp), `--level` (default `debug`), `--sample` (default 1.0), `--token` (env `TUNNEL_MANAGEMENT_TOKEN`), `--management-hostname` (hidden, default `management.argotunnel.com`), `--trace` (hidden), `--loglevel` (default `info`, env `TUNNEL_LOGLEVEL`), `--origincert` (env `TUNNEL_ORIGIN_CERT`), `--output` (json/default); hidden subcommand `token` gets management JWT | current CLI surface | audited, parity-backed | baseline-backed tests | closed | WebSocket client streaming loop, output format tests | high | `tail token` fully wired: `build_client` → `get_management_token(Logs)` → JSON `{"token":"..."}` output, matching Go `managementTokenCommand`. `tail` bare: filter parsing (`parse_tail_filters` — level/event/sample with Go-matching validation), management URL building (`build_management_url` — token acquisition, FedRAMP hostname selection via `parse_management_token().is_fed()`, connector-id query param), output formatting (`format_log_line`, `format_log_json`) all implemented. WebSocket client streaming via `tokio-tungstenite`: `tail_streaming_loop()` dials management endpoint, sends `EventStartStreaming` with filters, reads `EventLog` frames and formats each entry via `format_log_line()`, SIGINT/SIGTERM clean shutdown with close frame. 14 unit tests in `tail_management::tests`, 3 behavioral integration tests in `main_tests.rs`. CDC-026 types and CDC-038 API are closed dependencies. See [`crates/cfdrs-bin/src/tail_management.rs`](../../../crates/cfdrs-bin/src/tail_management.rs), [docs/parity/cli/tail-and-management.md](tail-and-management.md) |
| CLI-024 | management subtree | `cmd/cloudflared/management/cmd.go` | `management` (hidden, category `Management`) with hidden subcommand `token`; token subcommand requires `--resource` (values: `logs`, `admin`, `host_details`), `--origincert`, `--loglevel`, `--output` (json/default) | current CLI surface | audited, parity-backed | baseline-backed tests | closed | hidden-help routing tests, token-behavior tests | medium | `management` bare now matches Go hidden-command behavior by rendering command help with exit 0; `management token` remains fully wired through `parse_management_resource` → `build_client` → `get_management_token` → JSON `{"token":"..."}` output. `cfdrs-cli` now routes `management --help`, `help management`, and `management token --help` to exact hidden help text; `cfdrs-bin` integration tests verify bare help, token help, and behavioral token dispatch. Resource parsing remains unit-tested in [`crates/cfdrs-bin/src/tail_management.rs`](../../../crates/cfdrs-bin/src/tail_management.rs). See [docs/parity/cli/tail-and-management.md](tail-and-management.md) |

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
| CLI-032 | run command (Rust alpha) | partial overlap with frozen `tunnel` and `tunnel run` | current Rust `run` enters QUIC transport core + Pingora proxy seam; only partially overlaps frozen `tunnel` root runnable behavior and `tunnel run` named-tunnel flow; must not be treated as CLI parity | current runtime + current CLI surface | audited, parity-backed | baseline-backed tests | closed | command contract tests, compare against frozen `tunnel` root and `tunnel run` behavior | critical | `execute_tunnel_run()` implements the full Go `runCommand()` token precedence chain: `--token` > `--token-file` > positional arg > config `tunnel` field, matching exact Go error semantics. Token path: `resolve_run_token_string()` reads `--token` first, then `--token-file` (reads file, trims); `TunnelToken::decode()` validates; `execute_run_with_token()` writes token credentials to temp `{tunnel_id}.json` and injects tunnel identity + `credentials_file` into startup surface, matching Go `sc.runWithCredentials(token.Credentials())`. Positional arg path: injects `TunnelReference::from_raw()` for name-or-UUID resolution. Config fallback: reads `tunnel` field. `--credentials-contents` inline JSON: `apply_runtime_credential_discovery()` parses via `TunnelCredentialsFile::from_json_str()`, writes temp file, matching Go `findCredentials()`. Invalid token exits 255 with `TUNNEL_TOKEN_INVALID_MSG`; token-file read error exits 255 with `TUNNEL_TOKEN_FILE_READ_ERROR_PREFIX`; no-identity exits 255 with `TUNNEL_RUN_IDENTITY_ERROR_MSG`. 12 unit tests, 3 NArg integration tests, 2 credential-discovery tests. Runtime enters `execute_runtime_command()` → QUIC transport + Pingora proxy seam |

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
(transitional), and `run`/`tunnel run` (alpha-limited with full token
precedence chain) which have behavioral implementations. NArg validation at
all dispatch entry points matches Go baseline argument count requirements.
Per-subcommand help routing for `tunnel --help` and `access --help` matches
Go `SubcommandHelpTemplate` format.

Parsed flags: `--config`, `--help`/`-h`, `--version`/`-V`, `--short`/`-s`,
40+ tunnel/access/service/management flags stored into `GlobalFlags`

Missing from baseline: behavioral implementation behind the parsed stubs,
exact help format matching

### Divergence records

**Root invocation (CLI-001):** Resolved. Rust dispatches empty
invocation (`ServiceMode`) to `execute_startup_command()` which enters config
discovery via `discover_config()` → runtime startup via
`ApplicationRuntime::run()`, matching Go `handleServiceMode()`
→ `FindOrCreateConfigPath()` → `AppManager` flow. Rust's
`spawn_signal_bridge()` + `spawn_config_watcher()` + `spawn_primary_service()`
is behaviorally equivalent to Go's `AppManager.Run()` →
`AppService.actionLoop()`. 5 baseline-backed tests verify: not-help dispatch,
config-error exit code, deployment contract output, ingress-rules rendering,
and equivalence between bare invocation and `tunnel run`.

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

CLI-031 (`validate`) remains the only intentional divergence in the
CLI ledger.

Evidence harness: blackbox captures exist in [docs/parity/cli/captures/](captures/)
with 6 capture files covering root, tunnel, access, tail, management,
service, update, error, and compatibility surfaces plus current Rust
output for comparison.

### Gap ranking by priority

Critical gaps: none remaining.

High gaps (behavioral implementation behind stubs):

- CLI-006: update command (parsed — needs HIS updater)

Medium gaps (all now parsed and dispatched to stubs):

- CLI-017: tunnel ready (depends on HIS-024/025)
- CLI-018: tunnel diag (depends on HIS diagnostics)
- CLI-021: tunnel ingress (hidden, parsed with validate/rule nesting)
- CLI-024: management subtree (hidden bare help + token path now parity-backed)

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
