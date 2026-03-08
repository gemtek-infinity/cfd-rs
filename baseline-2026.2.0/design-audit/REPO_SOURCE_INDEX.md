# Cloudflared Source Index

This file is a topic-to-source map for humans and AI. It is optimized for fast navigation from a behavior or contract to the owning code and likely supporting tests.

Path scope note:

- All source paths listed in this file are rooted at `old-impl/` in the current rewrite branch layout.
- The spec documents live alongside this file in `setpoint-docs-2026.2.0/`.

Companion appendices:

- `REPO_CLI_INVENTORY.md`
- `REPO_CONFIG_CONTRACT.md`
- `REPO_COMPONENTS_AND_DEPENDENCIES.md`
- `REPO_BEHAVIORAL_SPEC.md`
- `REPO_API_CONTRACTS.md`
- `REPO_ARCHITECTURE_DEEP_DIVE.md`
- `REPO_QUIRKS_AND_COMPATIBILITY.md`
- `REPO_ORGANIZATION_CATALOG.md`
- `REPO_DIAGRAMS.md`

## 1. Top-Level Package Map

| Path | Role | Category | Key Neighbors |
| --- | --- | --- | --- |
| `cmd/cloudflared` | root CLI, OS-specific service entrypoints | runtime entry | `config`, `logger`, `metrics`, `watcher` |
| `cmd/cloudflared/tunnel` | main tunnel commands and daemon startup | runtime-critical | `supervisor`, `orchestration`, `ingress`, `connection` |
| `cmd/cloudflared/access` | Access client commands | user tooling | `token`, `carrier`, `validation` |
| `cmd/cloudflared/tail` | remote log-streaming client | user tooling | `management`, `cfapi`, `credentials` |
| `cmd/cloudflared/management` | management JWT helper | support tooling | `cfapi`, `credentials` |
| `cmd/cloudflared/cliutil` | common CLI helpers and deprecation wrappers | support | all command packages |
| `cmd/cloudflared/flags` | flag-name constants | support | command packages |
| `config` | YAML config search, load, validation helpers | contract-critical | `credentials`, `ingress`, `validation` |
| `credentials` | origin cert and tunnel credential discovery/parsing | contract-critical | `config`, `cfapi` |
| `cfapi` | Cloudflare API client and filter types | contract-critical | tunnel subcommands |
| `supervisor` | tunnel connection lifecycle and reconnection | runtime-critical | `connection`, `edgediscovery`, `retry`, `tunnelstate` |
| `orchestration` | hot-reload config/origin orchestration | runtime-critical | `ingress`, `proxy`, `flow` |
| `connection` | transport abstraction and implementations | runtime-critical | `quic`, `tunnelrpc`, `metrics`, `tracing` |
| `quic` | QUIC utilities and v3 datagram support | runtime-critical | `connection`, `datagramsession` |
| `tunnelrpc` | Cap'n Proto RPC schema and adapters | protocol-critical | `connection`, `orchestration`, `datagramsession` |
| `ingress` | routing rules and origin model | runtime-critical | `proxy`, `config`, `ipaccess` |
| `proxy` | origin proxying and request forwarding | runtime-critical | `ingress`, `flow`, `stream`, `carrier` |
| `datagramsession` | UDP/ICMP session multiplexing | runtime-critical | `packet`, `quic`, `management` |
| `packet` | raw packet encode/decode | runtime-critical | `datagramsession`, `ingress` |
| `flow` | active-flow limiting | runtime-critical | `orchestration`, `proxy` |
| `edgediscovery` | edge address and protocol-discovery logic | runtime-critical | `connection`, `supervisor` |
| `features` | feature-flag selection and deprecation filtering | runtime-critical | `client`, `supervisor` |
| `metrics` | local metrics HTTP server and readiness | contract-critical | `diagnostic`, `tunnelstate` |
| `management` | management service handlers and token/session logic | contract-critical | `logger`, `metrics`, `tail` |
| `diagnostic` | local/remote diagnostics support | support but operationally important | `metrics`, `management` |
| `tunnelstate` | connection tracking for readiness and diagnostics | runtime-critical | `metrics`, `connection` |
| `logger` | zerolog setup | support | most runtime packages |
| `tracing` | OpenTelemetry and trace propagation | support/runtime | `connection`, `proxy` |
| `tlsconfig` | TLS config assembly and reload | runtime-critical | `connection`, `ingress` |
| `token` | Access token handling | support | `access`, `management` |
| `validation` | hostname/url validation and auth validation | support | `config`, `access`, `ingress` |
| `watcher` | file-watch support for service mode and config reloads | runtime-critical when used | `config` |
| `retry` | retry and backoff primitives | runtime-critical | `supervisor` |
| `carrier` | Access TCP-over-WebSocket forwarding | support/runtime | `access`, `proxy` |
| `hello` | hello-world origin service | support/runtime | tunnel quick-test paths |
| `socks` | SOCKS proxy support | support/runtime | `ingress` |
| `stream` | stream helpers | support/runtime | `proxy`, `connection` |
| `fips` | FIPS-mode compile-time behavior | build/runtime | `tlsconfig`, build system |
| `component-tests` | Python end-to-end tests | test-only but behavior-critical | whole system |

## 2. Topic Index

### 2.1 CLI Root And Service Behavior

- Root app and command registration: `cmd/cloudflared/main.go`
- Linux service install/uninstall: `cmd/cloudflared/linux_service.go`
- macOS service install/uninstall: `cmd/cloudflared/macos_service.go`
- Windows service install/uninstall and service runtime: `cmd/cloudflared/windows_service.go`
- unsupported generic service mode: `cmd/cloudflared/generic_service.go`

### 2.2 Tunnel CLI

- tunnel root behavior and flags: `cmd/cloudflared/tunnel/cmd.go`
- tunnel subcommands: `cmd/cloudflared/tunnel/subcommands.go`
- ingress helper commands: `cmd/cloudflared/tunnel/ingress_subcommands.go`
- private route commands: `cmd/cloudflared/tunnel/teamnet_subcommands.go`
- virtual network commands: `cmd/cloudflared/tunnel/vnets_subcommands.go`
- tunnel login: `cmd/cloudflared/tunnel/login.go`
- credential lookup helpers: `cmd/cloudflared/tunnel/credential_finder.go`

Likely tests:

- `cmd/cloudflared/tunnel/subcommand_context_test.go`
- `cmd/cloudflared/tunnel/subcommands_test.go`
- `cmd/cloudflared/tunnel/tag_test.go`

### 2.3 Access CLI

- `cmd/cloudflared/access/cmd.go`
- `cmd/cloudflared/access/carrier.go`

### 2.4 Config And Credentials

- config search/load/schema: `config/configuration.go`
- credential schema and origin cert search: `credentials/credentials.go`, `credentials/origin_cert.go`

Likely tests:

- `config/configuration_test.go`
- `credentials/credentials_test.go`
- `credentials/origin_cert_test.go`

### 2.5 Startup And Shutdown

- daemon startup: `cmd/cloudflared/tunnel/cmd.go`
- supervisor entry: `supervisor/supervisor.go`
- per-tunnel behavior and fallback: `supervisor/tunnel.go`
- signal handling: `signal/`

### 2.6 Transport

- protocol selector and TLS names: `connection/protocol.go`
- transport abstraction: `connection/connection.go`
- HTTP/2 transport: `connection/http2.go`
- QUIC transport: `connection/quic.go`, `connection/quic_connection.go`
- datagram v2: `connection/quic_datagram_v2.go`
- datagram v3: `connection/quic_datagram_v3.go`, `quic/v3/`

Likely tests:

- `connection/protocol_test.go`
- `connection/http2_test.go`
- `connection/quic_connection_test.go`
- `connection/quic_datagram_v2_test.go`

### 2.7 RPC And Wire Contracts

- Cap'n Proto schema: `tunnelrpc/proto/tunnelrpc.capnp`
- QUIC metadata protocol schema: `tunnelrpc/proto/quic_metadata_protocol.capnp`
- QUIC protocol helpers: `tunnelrpc/quic/`
- schema adapters: `tunnelrpc/pogs/`

### 2.8 Ingress And Origin Routing

- ingress parsing and matching: `ingress/ingress.go`
- origin proxy logic: `ingress/origin_proxy.go`
- ingress middleware: `ingress/middleware/`
- proxy bridge: `proxy/`

Likely tests:

- `ingress/ingress_test.go`
- `connection/header_test.go`
- `proxy`-adjacent tests in connection and integration suites

### 2.9 Management, Metrics, And Diagnostics

- management service: `management/service.go`
- management events and session logic: `management/`
- tail command client: `cmd/cloudflared/tail/cmd.go`
- management token command: `cmd/cloudflared/management/cmd.go`
- metrics server: `metrics/metrics.go`
- readiness: `metrics/readiness.go`
- diagnostics: `diagnostic/`

Likely tests:

- `management/*_test.go`
- `diagnostic/diagnostic_utils_test.go`
- `connection/observer_test.go`

### 2.10 Private Routing And Datagrams

- datagram session manager: `datagramsession/manager.go`, `datagramsession/session.go`
- flow limiter: `flow/limiter.go`
- IP access filters: `ipaccess/`
- route and virtual-network API filters: `cfapi/ip_route*.go`, `cfapi/virtual_network*.go`

Likely tests:

- `datagramsession/*_test.go`
- `cfapi/ip_route_test.go`
- `cfapi/virtual_network_test.go`

### 2.11 Observability And Tracing

- logging: `logger/`
- tracing: `tracing/`
- build info metrics: `metrics/metrics.go`

### 2.12 Build, Release, And Packaging

- build/test/lint/release targets: `Makefile`
- FIPS validation script: `check-fips.sh`
- packaging hooks: `postinst.sh`, `postrm.sh`
- release scripts: `github_release.py`, `github_message.py`, `release_pkgs.py`
- Docker packaging: `Dockerfile`, `Dockerfile.amd64`, `Dockerfile.arm64`

## 3. Fast Paths For Common Questions

### 3.1 “Where is startup controlled?”

- `cmd/cloudflared/main.go`
- `cmd/cloudflared/tunnel/cmd.go`
- `supervisor/supervisor.go`

### 3.2 “Where are CLI flags defined?”

- command-local flags: command files under `cmd/cloudflared/`
- flag-name constants: `cmd/cloudflared/flags/flags.go`
- shared logging flags: `cmd/cloudflared/cliutil/logger.go`

### 3.3 “Where is config precedence enforced?”

- `config/configuration.go`
- `cmd/cloudflared/tunnel/cmd.go`
- `cmd/cloudflared/tunnel/subcommands.go`
- `ingress/ingress.go`

### 3.4 “Where is readiness defined?”

- `metrics/readiness.go`
- `tunnelstate/`

### 3.5 “Where is QUIC vs HTTP/2 selection defined?”

- `connection/protocol.go`
- `supervisor/tunnel.go`
- `cmd/cloudflared/tunnel/configuration.go`

### 3.6 “Where are deprecations and historical behavior changes tracked?”

- `CHANGES.md`
- `cmd/cloudflared/cliutil/deprecated.go`
- removed/compat command registrations in command packages

### 3.7 “Which tests best reflect external behavior?”

- `component-tests/`
- readiness/metrics/diagnostic unit tests
- transport and ingress tests under `connection/`, `ingress/`, and `cfapi/`

## 4. High-Value Test Suites By Topic

| Topic | Tests |
| --- | --- |
| config parsing | `config/configuration_test.go` |
| ingress behavior | `ingress/ingress_test.go` |
| protocol selection | `connection/protocol_test.go` |
| HTTP/2 transport | `connection/http2_test.go` |
| QUIC transport | `connection/quic_connection_test.go`, `connection/quic_datagram_v2_test.go` |
| connection observation | `connection/observer_test.go` |
| cfapi filters and routes | `cfapi/hostname_test.go`, `cfapi/ip_route_test.go`, `cfapi/tunnel_test.go`, `cfapi/virtual_network_test.go` |
| datagram sessions | `datagramsession/*_test.go` |
| diagnostics address logic | `diagnostic/diagnostic_utils_test.go` |
| component behavior | `component-tests/test_*.py` |

## 5. Source Cues For AI Or Human Onboarding

If the question is about:

- CLI shape: start in `cmd/cloudflared/main.go`, then the subcommand file.
- config key meaning: start in `config/configuration.go`, then `ingress/` or command flag definitions.
- runtime edge behavior: start in `cmd/cloudflared/tunnel/cmd.go`, then `supervisor/` and `connection/`.
- private routing: start in `cmd/cloudflared/tunnel/teamnet_subcommands.go`, `cmd/cloudflared/tunnel/vnets_subcommands.go`, `cfapi/ip_route*.go`, `cfapi/virtual_network*.go`.
- a management/debug endpoint: start in `management/service.go` or `metrics/metrics.go`.
- a wire-level contract: start in `tunnelrpc/proto/*.capnp` and `connection/protocol.go`.
