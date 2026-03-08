# Cloudflared CLI Inventory

This document is the exhaustive command and flag inventory for the repository-visible CLI surface.

It is designed for:

- operator lookup
- scripting review
- breaking-change review
- AI command-surface grounding

Where a command or flag is hidden, deprecated, or compatibility-only, that is called out explicitly.

## 1. Root App

Executable: `cloudflared`

Usage text:

`cloudflared [global options] [command] [command options]`

Primary top-level commands:

- `update`
- `version`
- `tunnel`
- `access`
- `tail`
- `management` hidden
- `service` OS-specific
- removed compatibility commands may still be registered to emit explicit failure text

Primary root behavior:

- Empty invocation enters service-mode behavior through the root action path.
- Non-empty invocation dispatches to tunnel command behavior when no more specific command is selected.

## 2. Root And Shared Global Flags

These are the effective shared flags assembled from tunnel and logging configuration.

| Flag | Env | Default | Notes |
| --- | --- | --- | --- |
| `--config` | none | first existing default config path | YAML config path; if omitted, loader searches known directories |
| `--origincert` | `TUNNEL_ORIGIN_CERT` | first found `cert.pem` | origin certificate for login-managed actions |
| `--autoupdate-freq` | none | updater default interval | periodic update check interval |
| `--no-autoupdate` | `NO_AUTOUPDATE` | false | disables automatic update restarts |
| `--metrics` | `TUNNEL_METRICS` | `localhost:0` or `0.0.0.0:0` in container runtime | local metrics bind address; default path first tries known ports |
| `--pidfile` | `TUNNEL_PIDFILE` | none | writes PID after first successful connection |
| `--url` | `TUNNEL_URL` | `http://localhost:8080` | only meaningful for single-origin CLI mode |
| `--hello-world` | `TUNNEL_HELLO_WORLD` | false | run built-in hello-world service |
| `--socks5` | `TUNNEL_SOCKS` | false | legacy/single-origin tunnel flag |
| `--proxy-connect-timeout` | none | 30s | legacy single-origin HTTP proxy setting |
| `--proxy-tls-timeout` | none | 10s | legacy single-origin HTTP proxy setting |
| `--proxy-tcp-keepalive` | none | 30s | legacy single-origin HTTP proxy setting |
| `--proxy-no-happy-eyeballs` | none | false | legacy single-origin behavior |
| `--proxy-keepalive-connections` | none | 100 | legacy single-origin behavior |
| `--proxy-keepalive-timeout` | none | 90s | legacy single-origin behavior |
| `--proxy-connection-timeout` | none | 90s | deprecated; no effect |
| `--proxy-expect-continue-timeout` | none | 90s | deprecated; no effect |
| `--http-host-header` | `TUNNEL_HTTP_HOST_HEADER` | none | legacy single-origin behavior |
| `--origin-server-name` | `TUNNEL_ORIGIN_SERVER_NAME` | none | legacy single-origin behavior |
| `--unix-socket` | `TUNNEL_UNIX_SOCKET` | none | exclusive with `--url` or positional origin argument |
| `--origin-ca-pool` | `TUNNEL_ORIGIN_CA_POOL` | none | custom CA path for origin validation |
| `--no-tls-verify` | `NO_TLS_VERIFY` | false | disables origin TLS verification |
| `--no-chunked-encoding` | `TUNNEL_NO_CHUNKED_ENCODING` | false | useful for some WSGI-like origins |
| `--http2-origin` | `TUNNEL_ORIGIN_ENABLE_HTTP2` | false | enables HTTP/2 to origin |
| `--management-hostname` | `TUNNEL_MANAGEMENT_HOSTNAME` | `management.argotunnel.com` | hidden in most contexts |
| `--service-op-ip` | `TUNNEL_SERVICE_OP_IP` | `198.41.200.113:80` | hidden fallback service-operation target |
| `--loglevel` | `TUNNEL_LOGLEVEL` | `info` | application log level |
| `--transport-loglevel` | `TUNNEL_PROTO_LOGLEVEL`, `TUNNEL_TRANSPORT_LOGLEVEL` | `info` | transport log level, alias `proto-loglevel` |
| `--logfile` | `TUNNEL_LOGFILE` | none | log file output |
| `--log-directory` | `TUNNEL_LOGDIRECTORY` | none | log directory output |
| `--trace-output` | `TUNNEL_TRACE_OUTPUT` | none | runtime trace written on stop |
| `--output` | `TUNNEL_MANAGEMENT_OUTPUT`, `TUNNEL_LOG_OUTPUT` | `default` | log output format, not tunnel list output format |
| `--is-autoupdated` | none | false | hidden inter-process autoupdate flag |
| `--edge` | `TUNNEL_EDGE` | none | hidden internal testing override |
| `--region` | `TUNNEL_REGION` | empty/global | edge region override |
| `--edge-ip-version` | `TUNNEL_EDGE_IP_VERSION` | `4` | `4`, `6`, or `auto` |
| `--edge-bind-address` | `TUNNEL_EDGE_BIND_ADDRESS` | none | source bind address for edge connections |
| `--cacert` | `TUNNEL_CACERT` | none | hidden CA for edge connection validation |
| `--hostname` | `TUNNEL_HOSTNAME` | none | route helper or deprecated classic tunnel signal depending on path |
| `--id` | `TUNNEL_ID` | none | hidden connector/tunnel instance identifier |
| `--lb-pool` | `TUNNEL_LB_POOL` | none | ad hoc route helper |
| `--api-key` | `TUNNEL_API_KEY` | none | hidden deprecated flag |
| `--api-email` | `TUNNEL_API_EMAIL` | none | hidden deprecated flag |
| `--api-ca-key` | `TUNNEL_API_CA_KEY` | none | hidden deprecated flag |
| `--api-url` | `TUNNEL_API_URL` | `https://api.cloudflare.com/client/v4` | hidden Cloudflare API base URL |
| `--metrics-update-freq` | `TUNNEL_METRICS_UPDATE_FREQ` | 5s | tunnel metrics refresh interval |
| `--tag` | `TUNNEL_TAG` | none | hidden custom tag key-value list |
| `--heartbeat-interval` | none | 5s | hidden heartbeat tuning |
| `--heartbeat-count` | none | 5 | hidden heartbeat tuning |
| `--max-edge-addr-retries` | none | 8 | hidden fallback tuning |
| `--retries` | `TUNNEL_RETRIES` | 5 | retry count for connection/protocol errors |
| `--ha-connections` | none | 4 | hidden but behaviorally important parallel edge connections |
| `--rpc-timeout` | none | 5s | hidden Cap'n Proto RPC timeout |
| `--write-stream-timeout` | `TUNNEL_STREAM_WRITE_TIMEOUT` | 0 | hidden origin/edge stream write timeout |
| `--quic-disable-pmtu-discovery` | `TUNNEL_DISABLE_QUIC_PMTU` | false | hidden QUIC PMTU disable |
| `--quic-connection-level-flow-control-limit` | `TUNNEL_QUIC_CONN_LEVEL_FLOW_CONTROL_LIMIT` | 30 MiB | hidden QUIC tuning |
| `--quic-stream-level-flow-control-limit` | `TUNNEL_QUIC_STREAM_LEVEL_FLOW_CONTROL_LIMIT` | 6 MiB | hidden QUIC tuning |
| `--label` | none | empty | connector label for management/observability |
| `--grace-period` | `TUNNEL_GRACE_PERIOD` | 30s | graceful shutdown wait |
| `--compression-quality` | `TUNNEL_COMPRESSION_LEVEL` | 0 | beta cross-stream compression level |
| `--use-reconnect-token` | `TUNNEL_USE_RECONNECT_TOKEN` | true | hidden reconnect-token path |
| `--dial-edge-timeout` | `DIAL_EDGE_TIMEOUT` | 15s | hidden edge dial timeout |
| `--stdin-control` | `STDIN_CONTROL` | false | hidden stdin control path |
| `--name` | `TUNNEL_NAME` | none | ad hoc create/route/run path |
| `--ui` | none | false | hidden deprecated UI flag |
| `--quick-service` | none | `https://api.trycloudflare.com` | hidden quick-tunnel service URL |
| `--max-fetch-size` | `TUNNEL_MAX_FETCH_SIZE` | none | hidden list pagination limiter |
| `--post-quantum`, `--pq` | `TUNNEL_POST_QUANTUM` | false | enforces PQ path; effectively QUIC-only |
| `--management-diagnostics` | `TUNNEL_MANAGEMENT_DIAGNOSTICS` | true | exposes extra management diagnostics |
| `--protocol`, `-p` | `TUNNEL_TRANSPORT_PROTOCOL` | `auto` | protocol selection |
| `--overwrite-dns`, `-f` | `TUNNEL_FORCE_PROVISIONING_DNS` | false | overwrite existing DNS during route creation |

## 3. Root Commands

### 3.1 `update`

Purpose: update the binary from the official update server.

Flags:

| Flag | Default | Hidden | Notes |
| --- | --- | --- | --- |
| `--beta` | false | no | update to latest beta |
| `--force` | false | yes | force upgrade regardless of current version |
| `--staging` | false | yes | use staging update URL |
| `--version` | none | no | upgrade or downgrade to explicit version |

Special behavior:

- Exit code 11 is used to signal that an update happened.

### 3.2 `version`

Purpose: print version details.

Flags:

| Flag | Default | Notes |
| --- | --- | --- |
| `--short`, `-s` | false | print version number only |

### 3.3 `service`

Purpose: OS-specific service management.

Subcommands:

- `install`
- `uninstall`

Linux-only extra flag:

| Flag | Default | Notes |
| --- | --- | --- |
| `--no-update-service` | false | disables separate auto-update system service/timer |

## 4. `tunnel` Command Tree

### 4.1 `tunnel`

Purpose: main tunnel lifecycle and routing namespace, but also runnable action.

Important behavior when invoked directly without a subcommand:

- with `--name`: create/route/run ad hoc named tunnel workflow
- with quick-tunnel origin flags: quick tunnel workflow
- with config tunnel id only: suggests `tunnel run`
- with legacy classic-tunnel `--hostname`: explicit deprecation error
- otherwise: tunnel command usage error

### 4.2 `tunnel login`

Purpose: obtain local origin certificate for tunnel management.

Notes:

- writes `cert.pem`-style credential material in default config directories
- prerequisite for most named-tunnel management actions that need account auth

### 4.3 `tunnel create NAME`

Purpose: create a named tunnel and write tunnel credentials.

Flags:

| Flag | Env | Notes |
| --- | --- | --- |
| `--output`, `-o` | none | `json` or `yaml` output |
| `--credentials-file`, `--cred-file` | `TUNNEL_CRED_FILE` | target path for tunnel credentials JSON |
| `--secret`, `-s` | `TUNNEL_CREATE_SECRET` | base64 secret; must decode to at least 32 bytes |

### 4.4 `tunnel list`

Purpose: list tunnels and optionally deleted or filtered results.

Flags:

| Flag | Env | Default | Notes |
| --- | --- | --- | --- |
| `--output`, `-o` | none | none | `json` or `yaml` |
| `--show-deleted`, `-d` | none | false | include deleted tunnels |
| `--name`, `-n` | none | empty | exact name filter |
| `--name-prefix`, `-np` | none | empty | prefix filter |
| `--exclude-name-prefix`, `-enp` | none | empty | inverse prefix filter |
| `--when`, `-w` | none | current time | active-at time in RFC3339 |
| `--id`, `-i` | none | empty | exact UUID filter |
| `--show-recently-disconnected`, `-rd` | none | false | include pending reconnects |
| `--sort-by` | `TUNNEL_LIST_SORT_BY` | `name` | `name`, `id`, `createdAt`, `deletedAt`, `numConnections` |
| `--invert-sort` | `TUNNEL_LIST_INVERT_SORT` | false | reverse order |
| `--max-fetch-size` | `TUNNEL_MAX_FETCH_SIZE` | none | hidden pagination limiter inherited from tunnel-global flags |

### 4.5 `tunnel ready`

Purpose: query the local `/ready` endpoint and return a suitable exit code.

Contract:

- Requires explicit `--metrics`.
- Requests `http://<metrics>/ready`.
- Non-200 responses are treated as command failure and body is surfaced in error text.

### 4.6 `tunnel info TUNNEL`

Purpose: list active connector details for a specific tunnel.

Flags:

| Flag | Env | Notes |
| --- | --- | --- |
| `--output`, `-o` | none | `json` or `yaml` |
| `--show-recently-disconnected`, `-rd` | none | include pending reconnects |
| `--sort-by` | `TUNNEL_INFO_SORT_BY` | `id`, `createdAt`, `numConnections`, `version` |
| `--invert-sort` | `TUNNEL_INFO_INVERT_SORT` | reverse order |

### 4.7 `tunnel delete TUNNEL...`

Purpose: delete one or more tunnels.

Flags:

| Flag | Env | Notes |
| --- | --- | --- |
| `--credentials-file`, `--cred-file` | `TUNNEL_CRED_FILE` | file path context for credentials lookup/write |
| `--force`, `-f` | `TUNNEL_RUN_FORCE_OVERWRITE` | delete even with dependencies; still subject to server-side constraints |

### 4.8 `tunnel run [TUNNEL]`

Purpose: run a named tunnel by ID, name, config value, or explicit token.

Flags unique or especially relevant to `run`:

| Flag | Env | Notes |
| --- | --- | --- |
| `--credentials-file`, `--cred-file` | `TUNNEL_CRED_FILE` | credentials JSON path |
| `--credentials-contents` | `TUNNEL_CRED_CONTENTS` | inline JSON credentials; overrides credentials-file |
| `--post-quantum`, `--pq` | `TUNNEL_POST_QUANTUM` | PQ mode |
| `--protocol`, `-p` | `TUNNEL_TRANSPORT_PROTOCOL` | `auto`, `quic`, `http2` |
| `--features`, `-F` | none | opt into in-development/tested features |
| `--token` | `TUNNEL_TOKEN` | tunnel token; overrides credentials and token-file |
| `--token-file` | `TUNNEL_TOKEN_FILE` | read tunnel token from file |
| `--icmpv4-src` | `TUNNEL_ICMPV4_SRC` | ICMPv4 source override |
| `--icmpv6-src` | `TUNNEL_ICMPV6_SRC` | ICMPv6 source/interface override |
| `--max-active-flows` | `TUNNEL_MAX_ACTIVE_FLOWS` | override remote private-flow limit |
| `--dns-resolver-addrs` | `TUNNEL_DNS_RESOLVER_ADDRS` | override virtual DNS resolver targets |

Other inherited tunnel-global and proxy flags also apply.

Token resolution order:

1. explicit `--token`
2. `--token-file`
3. tunnel ID/name/config + credentials path

### 4.9 `tunnel cleanup TUNNEL...`

Purpose: clean up tunnel connection records.

Flags:

| Flag | Env | Notes |
| --- | --- | --- |
| `--connector-id`, `-c` | `TUNNEL_CLEANUP_CONNECTOR` | clean only a specific connector |

### 4.10 `tunnel token TUNNEL`

Purpose: fetch the run token for an existing tunnel.

Flags:

| Flag | Env | Notes |
| --- | --- | --- |
| `--credentials-file`, `--cred-file` | `TUNNEL_CRED_FILE` | if provided, write credentials JSON instead of printing encoded token |

### 4.11 `tunnel route`

Purpose: define how traffic reaches the tunnel.

Subcommands:

- `dns`
- `lb`
- `ip`

#### 4.11.1 `tunnel route dns TUNNEL HOSTNAME`

Flags:

| Flag | Env | Notes |
| --- | --- | --- |
| `--overwrite-dns`, `-f` | `TUNNEL_FORCE_PROVISIONING_DNS` | replace existing DNS record |

#### 4.11.2 `tunnel route lb TUNNEL HOSTNAME LB-POOL`

Purpose: create or reuse load balancer routing to the tunnel.

#### 4.11.3 `tunnel route ip`

Purpose: manage private CIDR routing for WARP/private network users.

Subcommands:

- `add`
- `show` alias `list`
- `delete`
- `get`

Common flag:

| Flag | Aliases | Notes |
| --- | --- | --- |
| `--vnet` | `--vn` | identify virtual network by ID or name |

Route-ip filter flags used by `show`:

| Flag | Aliases | Notes |
| --- | --- | --- |
| `--filter-is-deleted` | none | if false default, show non-deleted only; if true, deleted only |
| `--filter-tunnel-id` | none | show only routes for given tunnel UUID |
| `--filter-network-is-subset-of` | `--nsub` | show routes whose network is subset of supplied network |
| `--filter-network-is-superset-of` | `--nsup` | show routes whose network is superset of supplied network |
| `--filter-comment-is` | none | exact comment filter |
| `--filter-vnet-id` | none | exact virtual network UUID filter |
| `--output`, `-o` | none | `json` or `yaml` |

### 4.12 `tunnel vnet`

Purpose: manage virtual networks used to disambiguate overlapping private CIDRs.

Subcommands:

- `add`
- `list`
- `delete`
- `update`

Flags used across vnet commands:

| Flag | Aliases | Notes |
| --- | --- | --- |
| `--default` | `-d` | make target virtual network account default |
| `--name` | `-n` | update name or filter by name depending on command |
| `--comment` | `-c` | comment text |
| `--force` | `-f` | force delete and migrate/delete dependents |

List filter flags:

| Flag | Notes |
| --- | --- |
| `--id` | exact vnet UUID |
| `--name` | exact vnet name |
| `--is-default` | filter by default status |
| `--show-deleted` | include deleted or show deleted only depending on bool |
| `--output`, `-o` | `json` or `yaml` |

### 4.13 `tunnel ingress`

Purpose: hidden ingress validation and rule-testing helper.

Subcommands:

- `validate`
- `rule`

Flags:

| Flag | Env | Notes |
| --- | --- | --- |
| `--json`, `-j` | `TUNNEL_INGRESS_VALIDATE_JSON` | validate rules from JSON rather than config file |

Important behavior:

- rejects `--url` when ingress rules are used
- validates catch-all and rule syntax
- `rule` prints matched rule index and formatted rule details

### 4.14 `tunnel diag`

Purpose: create a local diagnostic report for one running instance.

Flags:

| Flag | Notes |
| --- | --- |
| `--metrics` | target specific metrics address; otherwise known addresses are searched |
| `--diag-container-id` | collect logs from named container |
| `--diag-pod-id` | collect logs from kubernetes pod |
| `--no-diag-logs` | skip logs |
| `--no-diag-metrics` | skip metrics |
| `--no-diag-system` | skip system info |
| `--no-diag-runtime` | skip runtime info |
| `--no-diag-network` | skip network diagnostics |

## 5. `access` Command Tree

### 5.1 `access`

Aliases:

- `forward`

Flags:

| Flag | Notes |
| --- | --- |
| `--fedramp` | use fedramp account behavior |

Subcommands:

- `login`
- `curl`
- `token`
- `tcp` aliases `rdp`, `ssh`, `smb`
- `ssh-config`
- `ssh-gen`

### 5.2 `access login`

Flags:

| Flag | Notes |
| --- | --- |
| `--quiet`, `-q` | do not print JWT |
| `--no-verbose` | print only JWT to stdout |
| `--auto-close` | auto-close auth interstitial |
| `--app` | application URL |

### 5.3 `access curl`

Purpose: wrap curl and inject Access JWT.

Special behavior:

- uses custom argument parsing rather than normal flag parsing
- supports `--allow-request` compatibility path in argument stream

### 5.4 `access token`

Flags:

| Flag | Notes |
| --- | --- |
| `--app` | application URL |

### 5.5 `access tcp`

Aliases:

- `rdp`
- `ssh`
- `smb`

Flags:

| Flag | Env | Hidden | Notes |
| --- | --- | --- | --- |
| `--hostname`, `--tunnel-host`, `-T` | `TUNNEL_SERVICE_HOSTNAME` | no | target hostname |
| `--destination` | `TUNNEL_SERVICE_DESTINATION` | no | destination address |
| `--url`, `--listener`, `-L` | `TUNNEL_SERVICE_URL` | no | local listener host:port |
| `--header`, `-H` | none | no | extra headers |
| `--service-token-id`, `--id` | `TUNNEL_SERVICE_TOKEN_ID` | no | Access service token ID |
| `--service-token-secret`, `--secret` | `TUNNEL_SERVICE_TOKEN_SECRET` | no | Access service token secret |
| `--logfile` | none | no | save logs |
| `--log-directory` | none | no | save logs to directory |
| `--log-level`, alias `loglevel` | none | no | logging level |
| `--connect-to` | none | yes | alternate location for testing |
| `--debug-stream` | none | yes | log stream payloads for debugging |

### 5.6 `access ssh-config`

Flags:

| Flag | Notes |
| --- | --- |
| `--hostname` | application hostname |
| `--short-lived-cert` | emit short-lived cert example |

### 5.7 `access ssh-gen`

Flags:

| Flag | Notes |
| --- | --- |
| `--hostname` | application hostname |

## 6. `tail` Command Tree

### 6.1 `tail`

Purpose: remote log streaming from a connector.

Flags:

| Flag | Env | Hidden | Default | Notes |
| --- | --- | --- | --- | --- |
| `--connector-id` | `TUNNEL_MANAGEMENT_CONNECTOR` | no | empty | pick specific connector |
| `--event` | `TUNNEL_MANAGEMENT_FILTER_EVENTS` | no | all | filter event type |
| `--level` | `TUNNEL_MANAGEMENT_FILTER_LEVEL` | no | `debug` | filter minimum log level |
| `--sample` | `TUNNEL_MANAGEMENT_FILTER_SAMPLE` | no | `1.0` | sampling ratio `(0.0, 1.0]` |
| `--token` | `TUNNEL_MANAGEMENT_TOKEN` | no | empty | explicit management token |
| `--management-hostname` | `TUNNEL_MANAGEMENT_HOSTNAME` | yes | `management.argotunnel.com` | override management host |
| `--trace` | none | yes | empty | cf-trace-id for request |
| `--loglevel` | `TUNNEL_LOGLEVEL` | no | `info` | logging level |
| `--origincert` | `TUNNEL_ORIGIN_CERT` | no | discovered cert | certificate used when token acquisition is needed |
| `--output` | `TUNNEL_MANAGEMENT_OUTPUT`, `TUNNEL_LOG_OUTPUT` | no | `default` | log output format |

### 6.2 `tail token`

Purpose: hidden helper to emit management JWT for logs resource.

## 7. `management` Command Tree

### 7.1 `management`

Purpose: hidden management API tooling.

Subcommands:

- `token`

### 7.2 `management token`

Purpose: emit management JWT with requested resource scope.

Flags:

| Flag | Env | Notes |
| --- | --- | --- |
| `--resource` | none | required; `logs`, `admin`, or `host_details` |
| `--origincert` | `TUNNEL_ORIGIN_CERT` | origin certificate path |
| `--loglevel` | `TUNNEL_LOGLEVEL` | logging level |
| `--output` | `TUNNEL_MANAGEMENT_OUTPUT`, `TUNNEL_LOG_OUTPUT` | log output format |

## 8. Hidden, Deprecated, And Compatibility Commands

Known hidden or compatibility-only command paths:

- `management` and `management token`
- `tail token`
- `tunnel ingress` subtree
- `db-connect` removed command placeholder
- `proxy-dns` removed command placeholder at top level and under `tunnel`

Compatibility command behavior:

- returns explicit error text that the command is no longer supported
- remains present to avoid silent breakage in scripts that still parse older command names

## 9. CLI Breakage Review Prompts

When modifying the command surface, ask:

- Does this change parsing, precedence, or defaults of a visible or hidden flag?
- Does this remove a compatibility-only path that scripts may still call?
- Does this alter empty invocation or service install behavior?
- Does this rename any env-bound flag or alias?
- Does this change the semantics of hidden flags relied upon by Cloudflare internal tooling or automation?
