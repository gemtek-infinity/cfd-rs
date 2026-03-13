# Root And Global Flags — CLI Parity Audit

This document inventories every global and root-level flag from the frozen Go
baseline ([baseline-2026.2.0/old-impl/cmd/cloudflared/tunnel/cmd.go](../../../baseline-2026.2.0/old-impl/cmd/cloudflared/tunnel/cmd.go)) and
records current Rust coverage.

Authoritative checklist rows: CLI-001, CLI-002, CLI-003, CLI-004, CLI-005,
CLI-006, CLI-007.

## Root invocation divergence

The frozen Go binary enters service mode (`handleServiceMode()`) on empty
invocation. The current Rust binary shows help text. This is a critical
behavioral divergence (CLI-001).

## Version format divergence

Go format: `{Version} (built {BuildTime}{BuildTypeMsg})`
with `--short`/`-s` for number-only output.

Rust format: `cloudflared 2026.2.0-alpha.202603`
with no `--short`/`-s` support.

CLI-005 tracks this gap.

## Top-level commands (frozen Go)

| Command | Category | Hidden | Baseline source |
| --- | --- | --- | --- |
| `update` | | no | `updater/update.go` |
| `version` | | no | app-level config |
| `tunnel` | Tunnel | no | `tunnel/cmd.go` |
| `login` | | hidden when subcommand | `tunnel/login.go` (compat alias) |
| `proxy-dns` | DNS over HTTPS | no | `proxydns/cmd.go` (removed feature) |
| `access` (alias `forward`) | Access | no | `access/cmd.go` |
| `tail` | Tunnel | no | `tail/cmd.go` |
| `management` | Management | yes | `management/cmd.go` |
| `service` | Service | no | `linux_service.go` (Linux only) |

Rust coverage: `help`, `version`, `validate` (transitional), `run` (alpha).
Missing: `update`, `tunnel`, `login`, `proxy-dns`, `access`/`forward`, `tail`,
`management`, `service`.

## Global flag inventory

All flags below are defined in `tunnel/cmd.go` `Flags()` and apply to the
`tunnel` command family. Many are conditionally hidden when running as a
subcommand.

### Cloudflare configuration flags

| Flag | Aliases | Type | Default | Env var | Hidden | Rust |
| --- | --- | --- | --- | --- | --- | --- |
| `--config` | | string | `FindDefaultConfigPath()` | | hidden when subcommand | present |
| `--origincert` | | string | `FindDefaultOriginCertPath()` | `TUNNEL_ORIGIN_CERT` | no | absent |
| `--autoupdate-freq` | | duration | `DefaultCheckUpdateFreq` | | no | absent |
| `--no-autoupdate` | | bool | false | `NO_AUTOUPDATE` | no | absent |
| `--metrics` | | string | `GetMetricsDefaultAddress(Runtime)` | `TUNNEL_METRICS` | no | absent |
| `--pidfile` | | string | | `TUNNEL_PIDFILE` | no | absent |

### Edge connection flags

| Flag | Aliases | Type | Default | Env var | Hidden | Rust |
| --- | --- | --- | --- | --- | --- | --- |
| `--edge` | | string slice | | `TUNNEL_EDGE` | yes | absent |
| `--region` | | string | | `TUNNEL_REGION` | no | absent |
| `--edge-ip-version` | | string | `4` | `TUNNEL_EDGE_IP_VERSION` | no | absent |
| `--edge-bind-address` | | string | | `TUNNEL_EDGE_BIND_ADDRESS` | no | absent |
| `--cacert` | | string | | `TUNNEL_CACERT` | yes | absent |

### Credentials and auth flags

| Flag | Aliases | Type | Default | Env var | Hidden | Rust |
| --- | --- | --- | --- | --- | --- | --- |
| `--credentials-file` | `--cred-file` | string | | `TUNNEL_CRED_FILE` | no | absent |
| `--credentials-contents` | | string | | `TUNNEL_CRED_CONTENTS` | no | absent |
| `--token` | | string | | `TUNNEL_TOKEN` | no | absent |
| `--token-file` | | string | | `TUNNEL_TOKEN_FILE` | no | absent |
| `--is-autoupdated` | | bool | false | | yes | absent |

### Tunnel identity and routing flags

| Flag | Aliases | Type | Default | Env var | Hidden | Rust |
| --- | --- | --- | --- | --- | --- | --- |
| `--hostname` | | string | | `TUNNEL_HOSTNAME` | hidden when subcommand | absent |
| `--id` | | string | | `TUNNEL_ID` | yes | absent |
| `--lb-pool` | | string | | `TUNNEL_LB_POOL` | hidden when subcommand | absent |
| `--name` | `-n` | string | | `TUNNEL_NAME` | hidden when subcommand | absent |

### Deprecated API flags

| Flag | Type | Env var | Hidden | Rust |
| --- | --- | --- | --- | --- |
| `--api-key` | string | `TUNNEL_API_KEY` | yes | absent |
| `--api-email` | string | `TUNNEL_API_EMAIL` | yes | absent |
| `--api-ca-key` | string | `TUNNEL_API_CA_KEY` | yes | absent |

### API and metrics configuration flags

| Flag | Type | Default | Env var | Hidden | Rust |
| --- | --- | --- | --- | --- | --- |
| `--api-url` | string | `https://api.cloudflare.com/client/v4` | `TUNNEL_API_URL` | yes | absent |
| `--metrics-update-freq` | duration | 5s | `TUNNEL_METRICS_UPDATE_FREQ` | no | absent |
| `--tag` | string slice | | `TUNNEL_TAG` | yes | absent |

### Tunnel connection flags

| Flag | Type | Default | Env var | Hidden | Rust |
| --- | --- | --- | --- | --- | --- |
| `--heartbeat-interval` | duration | 5s | | yes | absent |
| `--heartbeat-count` | int | 5 | | yes | absent |
| `--max-edge-addr-retries` | int | 8 | | yes | absent |
| `--retries` | int | 5 | `TUNNEL_RETRIES` | hidden when subcommand | absent |
| `--ha-connections` | int | 4 | | yes | absent |
| `--rpc-timeout` | duration | 5s | | yes | absent |
| `--write-stream-timeout` | duration | 0s | `TUNNEL_STREAM_WRITE_TIMEOUT` | yes | absent |
| `--quic-disable-pmtu-discovery` | bool | false | `TUNNEL_DISABLE_QUIC_PMTU` | yes | absent |
| `--quic-connection-level-flow-control-limit` | int | 30MB | `TUNNEL_QUIC_CONN_LEVEL_FLOW_CONTROL_LIMIT` | yes | absent |
| `--quic-stream-level-flow-control-limit` | int | 6MB | `TUNNEL_QUIC_STREAM_LEVEL_FLOW_CONTROL_LIMIT` | yes | absent |

### Tunnel features and advanced flags

| Flag | Aliases | Type | Default | Env var | Hidden | Rust |
| --- | --- | --- | --- | --- | --- | --- |
| `--label` | | string | | | no | absent |
| `--grace-period` | | duration | 30s | `TUNNEL_GRACE_PERIOD` | hidden when subcommand | absent |
| `--compression-quality` | | int | 0 | `TUNNEL_COMPRESSION_LEVEL` | hidden when subcommand | absent |
| `--use-reconnect-token` | | bool | true | `TUNNEL_USE_RECONNECT_TOKEN` | yes | absent |
| `--dial-edge-timeout` | | duration | 15s | `DIAL_EDGE_TIMEOUT` | yes | absent |
| `--stdin-control` | | bool | false | `STDIN_CONTROL` | yes | absent |
| `--ui` | | bool | false | | yes | absent |
| `--quick-service` | | string | `https://api.trycloudflare.com` | | yes | absent |
| `--max-fetch-size` | | int | | `TUNNEL_MAX_FETCH_SIZE` | yes | absent |
| `--post-quantum` | `-pq` | bool | | `TUNNEL_POST_QUANTUM` | hidden when FIPS | absent |
| `--management-diagnostics` | | bool | true | `TUNNEL_MANAGEMENT_DIAGNOSTICS` | no | absent |
| `--protocol` | `-p` | string | `auto` | `TUNNEL_TRANSPORT_PROTOCOL` | yes | absent |
| `--overwrite-dns` | | bool | | | no | absent |
| `--management-hostname` | | string | `management.argotunnel.com` | `TUNNEL_MANAGEMENT_HOSTNAME` | yes | absent |
| `--service-op-ip` | | string | `198.41.200.113:80` | `TUNNEL_SERVICE_OP_IP` | yes | absent |

### Logging flags (from `cliutil.ConfigureLoggingFlags`)

| Flag | Type | Default | Env var | Hidden | Rust |
| --- | --- | --- | --- | --- | --- |
| `--loglevel` | string | | `TUNNEL_LOGLEVEL` | no | absent |
| `--transport-loglevel` | string | | `TUNNEL_TRANSPORT_LOGLEVEL` | no | absent |
| `--logfile` | string | | | no | absent |
| `--log-directory` | string | | | no | absent |
| `--output` | string | `default` | | no | absent |
| `--trace-output` | string | | | no | absent |

### Proxy/origin flags

| Flag | Type | Default | Env var | Hidden | Rust |
| --- | --- | --- | --- | --- | --- |
| `--url` | string | `http://localhost:8080` | `TUNNEL_URL` | hidden when subcommand | absent |
| `--hello-world` | bool | false | `TUNNEL_HELLO_WORLD` | hidden when subcommand | absent |
| `--socks5` | bool | false | `TUNNEL_SOCKS` | hidden when subcommand | absent |
| `--proxy-connect-timeout` | duration | 30s | | hidden when subcommand | absent |
| `--proxy-tls-timeout` | duration | 10s | | hidden when subcommand | absent |
| `--proxy-tcp-keepalive` | duration | 30s | | hidden when subcommand | absent |
| `--proxy-no-happy-eyeballs` | bool | false | | hidden when subcommand | absent |
| `--proxy-keepalive-connections` | int | 100 | | hidden when subcommand | absent |
| `--proxy-keepalive-timeout` | duration | 90s | | hidden when subcommand | absent |
| `--proxy-connection-timeout` | duration | 90s | | hidden when subcommand | absent |
| `--proxy-expect-continue-timeout` | duration | 90s | | hidden when subcommand | absent |
| `--http-host-header` | string | | `TUNNEL_HTTP_HOST_HEADER` | hidden when subcommand | absent |
| `--origin-server-name` | string | | `TUNNEL_ORIGIN_SERVER_NAME` | hidden when subcommand | absent |
| `--unix-socket` | string | | `TUNNEL_UNIX_SOCKET` | hidden when subcommand | absent |
| `--origin-ca-pool` | string | | `TUNNEL_ORIGIN_CA_POOL` | hidden when subcommand | absent |
| `--no-tls-verify` | bool | false | `NO_TLS_VERIFY` | hidden when subcommand | absent |
| `--no-chunked-encoding` | bool | false | `TUNNEL_NO_CHUNKED_ENCODING` | hidden when subcommand | absent |
| `--http2-origin` | bool | false | `TUNNEL_ORIGIN_ENABLE_HTTP2` | hidden when subcommand | absent |

### SSH server flags

| Flag | Aliases | Type | Default | Env var | Hidden | Rust |
| --- | --- | --- | --- | --- | --- | --- |
| `--local-ssh-port` | | string | `2222` | `LOCAL_SSH_PORT` | yes | absent |
| `--ssh-idle-timeout` | | duration | | `SSH_IDLE_TIMEOUT` | yes | absent |
| `--ssh-max-timeout` | | duration | | `SSH_MAX_TIMEOUT` | yes | absent |
| `--bucket-name` | | string | | `BUCKET_ID` | yes | absent |
| `--region-name` | | string | | `REGION_ID` | yes | absent |
| `--secret-id` | | string | | `SECRET_ID` | yes | absent |
| `--access-key-id` | | string | | `ACCESS_CLIENT_ID` | yes | absent |
| `--session-token` | | string | | `SESSION_TOKEN_ID` | yes | absent |
| `--s3-url-host` | | string | | `S3_URL` | yes | absent |
| `--host-key-path` | | path | | `HOST_KEY_PATH` | yes | absent |
| `--ssh-server` | | bool | false | `TUNNEL_SSH_SERVER` | yes | absent |
| `--bastion` | | bool | false | `TUNNEL_BASTION` | hidden when subcommand | absent |
| `--proxy-address` | | string | `127.0.0.1` | `TUNNEL_PROXY_ADDRESS` | hidden when subcommand | absent |
| `--proxy-port` | | int | 0 | `TUNNEL_PROXY_PORT` | hidden when subcommand | absent |

### ICMP flags

| Flag | Type | Env var | Hidden | Rust |
| --- | --- | --- | --- | --- |
| `--icmpv4-src` | string | `TUNNEL_ICMPV4_SRC` | no | absent |
| `--icmpv6-src` | string | `TUNNEL_ICMPV6_SRC` | no | absent |
| `--max-active-flows` | uint64 | `TUNNEL_MAX_ACTIVE_FLOWS` | no | absent |
| `--dns-resolver-addrs` | string slice | `TUNNEL_DNS_RESOLVER_ADDRS` | no | absent |

### Root-only flags (non-tunnel)

| Flag | Aliases | Type | Default | Env var | Command | Rust |
| --- | --- | --- | --- | --- | --- | --- |
| `--version` | `-v`, `-V` | bool | | | root app | partial (flag exists, behavior differs) |
| `--help` | `-h` | bool | | | root app | present |
| `--quiet` | `-q` | bool | | | root app | absent |

## Update command flags

| Flag | Type | Default | Hidden | Rust |
| --- | --- | --- | --- | --- |
| `--beta` | bool | false | no | absent |
| `--force` | bool | false | yes | absent |
| `--staging` | bool | false | yes | absent |
| `--version` | string | | no | absent |

## Service command flags (Linux only)

### `service install`

| Flag | Type | Default | Rust |
| --- | --- | --- | --- |
| `--no-update-service` | bool | false | absent |

### `service uninstall`

No flags.

## Coverage summary

- Total global/tunnel-level flags: approximately 80
- Total with Rust coverage: 1 (`--config`)
- Coverage percentage: approximately 1%
