# Cloudflared Repository Reference

Repository: cloudflare/cloudflared  
Branch baseline: master  
Reference date: 2026-03-08

Current rewrite layout note:

- The branch root contains `AGENTS.md`, `SKILLS.md`, `LICENSE`, `old-impl/`, and `setpoint-docs-2026.2.0/`.
- The Go source of truth lives under `old-impl/`.
- Unless explicitly stated otherwise, package and file paths in this document are paths inside `old-impl/`.

This document is intended to be the canonical repository-local reference for cloudflared. It is written to support five concrete uses:

1. Full comprehension for both humans and AI.
2. Bug and quirk discovery.
3. Contract breakage analysis.
4. Human and AI onboarding.
5. Future rewrite planning.

It covers repository-observable behavior and contracts. Where behavior depends on Cloudflare-managed services, only the client-visible side is treated as authoritative here.

## 1. What Cloudflared Is

cloudflared is Cloudflare's command-line tunnel connector and related client utility. At its core it maintains one or more long-lived outbound connections from a machine or network to Cloudflare edge, then proxies traffic received from the edge to local origins without requiring inbound firewall openings.

At repository level, cloudflared is not a single feature. It is a bundle of related surfaces:

- Tunnel connector daemon for HTTP, WebSocket, TCP, UDP, ICMP, and private-routing traffic.
- Tunnel lifecycle CLI for creating, routing, running, inspecting, deleting, and cleaning up tunnels.
- Access client utilities for authenticating to Access-protected applications.
- Management and log-streaming client utilities.
- Metrics, diagnostics, tracing, packaging, and service-installation support.

The dominant runtime path is:

1. CLI parses flags and config.
2. Tunnel startup builds runtime config, routing, diagnostics, and edge connection state.
3. Supervisor establishes and maintains multiple edge connections.
4. Connection layer receives streams or datagrams from the edge.
5. Ingress and proxy layers route traffic to the correct local origin.
6. Metrics, management, and diagnostics stay available throughout the process lifetime.

## 2. Documentation Contract

This reference distinguishes five classes of statements:

- Stable contract: user-visible, operator-visible, or wire-visible behavior the repository clearly treats as an interface.
- Observed behavior: behavior implemented in code but not clearly committed as an external promise.
- Internal boundary: subsystem contracts that matter for maintenance and rewrites.
- Version-sensitive quirk: behavior that changed over time or is intentionally special-cased.
- Deprecated path: behavior retained for compatibility, migration, or explicit rejection.

For rewrite or breakage analysis, stable contracts take precedence over internal structure. Internal structure is documented because many bugs and regressions arise when a rewrite preserves surface syntax but loses timing, precedence, fallback, or shutdown semantics.

## 3. Repository Mental Model

The easiest way to reason about cloudflared is as seven stacked planes:

1. Command plane: `cmd/cloudflared` and subcommands register the user and operator surface.
2. Config plane: `config`, `credentials`, and CLI flag resolution build runtime inputs.
3. Orchestration plane: `cmd/cloudflared/tunnel`, `orchestration`, and `supervisor` assemble and operate the daemon.
4. Transport plane: `connection`, `quic`, `tunnelrpc`, and `datagramsession` define edge connectivity and multiplexing.
5. Routing plane: `ingress`, `proxy`, `flow`, and origin services decide where traffic goes.
6. Observability and management plane: `metrics`, `management`, `diagnostic`, `tracing`, and `logger` expose health and control.
7. Integration plane: `cfapi`, packaging scripts, service installers, and component tests connect the repo to Cloudflare APIs, OS service managers, and release systems.

## 4. Runtime Modes

cloudflared has multiple materially different operating modes.

### 4.1 Tunnel Daemon Modes

- Named tunnel: persistent tunnel identified by UUID plus credentials JSON.
- Quick tunnel: ephemeral trycloudflare-style tunnel created only when `--url` or `--hello-world` is explicitly supplied and quick-service is in use.
- Ad hoc named tunnel: `cloudflared tunnel --name ...` can create, optionally route, and run a named tunnel in one invocation.
- Service mode: empty invocation on supported platforms enters config-file-watching service behavior rather than regular subcommand handling.

### 4.2 Access Client Modes

- `access login`: interactive browser-based login and token caching.
- `access token`: token emission.
- `access curl`: wrapper that injects Access JWT into outbound requests.
- `access tcp` and aliases: Layer 4 forwarding through Cloudflare edge.
- `access ssh-config` and `access ssh-gen`: SSH helper workflows.

### 4.3 Management Modes

- `tail`: remote log streaming from a connector through the management plane.
- `management token`: hidden command for scoped management JWT creation.
- `tunnel diag`: local troubleshooting bundle generation.

### 4.4 Service Installation Modes

- Linux: systemd or SysV install/uninstall.
- macOS: launch agent for user, launch daemon for root.
- Windows: Windows service install/uninstall with service control manager integration.
- Other OSes: explicit not-supported service commands.

## 5. Entry Points And Startup

Primary entry point: `cmd/cloudflared/main.go`.

Startup does the following before any tunnel work:

1. Disables QUIC ECN via `QUIC_GO_DISABLE_ECN=1` because of an explicit upstream/runtime workaround.
2. Registers build info metrics.
3. Applies automatic CPU limit tuning through `automaxprocs`.
4. Builds the root CLI app.
5. Initializes subcommand modules: tunnel, access, updater, tracing, token, tail, management.
6. Dispatches to OS-specific `runApp`.

The default root action has two materially different branches:

- Empty invocation: enters service mode and watches config from `config.FindOrCreateConfigPath()`.
- Non-empty invocation: executes `tunnel.TunnelCommand`.

This means an empty `cloudflared` invocation is not a no-op and not a help command. It can create a config file path, monitor a config file, and run service-like behavior.

## 6. CLI Contract

The CLI is built on `urfave/cli/v2`, but the repository uses a forked replacement via `replace github.com/urfave/cli/v2 => github.com/ipostelnik/cli/v2` in `go.mod`. Any rewrite that assumes upstream stock CLI behavior should verify the fork’s semantics where it matters.

### 6.1 Root Commands

Top-level commands registered from `cmd/cloudflared/main.go` and OS-specific files:

- `tunnel`
- `access`
- `tail`
- `management` (hidden)
- `update`
- `version`
- `service` (OS-specific)
- Removed-compat commands such as `proxy-dns` and `db-connect` are still represented only to fail explicitly.

### 6.2 Root Global Flags

The root flag set is composed from tunnel flags plus access flags. The operationally important global flags include:

- `--config`: YAML config path.
- `--origincert`: origin certificate path. Env: `TUNNEL_ORIGIN_CERT`.
- `--metrics`: metrics bind address. Env: `TUNNEL_METRICS`.
- `--loglevel`: application log level. Env: `TUNNEL_LOGLEVEL`.
- `--logfile`: log file path. Env: `TUNNEL_LOGFILE`.
- `--log-directory`: log directory. Env: `TUNNEL_LOGDIRECTORY`.
- `--output`: log output format. Env includes `TUNNEL_LOG_OUTPUT` and management-specific output envs.
- `--trace-output`: runtime trace output path. Env: `TUNNEL_TRACE_OUTPUT`.
- `--autoupdate-freq`: autoupdate polling frequency.
- `--no-autoupdate`: disable autoupdate. Env: `NO_AUTOUPDATE`.
- `--pidfile`: write PID after first successful connection. Env: `TUNNEL_PIDFILE`.

Tunnel-global runtime flags that materially affect behavior include:

- `--protocol`: `auto`, `quic`, `http2`. Hidden in some contexts. Env: `TUNNEL_TRANSPORT_PROTOCOL`.
- `--post-quantum` or `--pq`: force post-quantum-capable path, which in practice forces QUIC. Env: `TUNNEL_POST_QUANTUM`.
- `--edge-ip-version`: `4`, `6`, `auto`. Env: `TUNNEL_EDGE_IP_VERSION`.
- `--edge-bind-address`: source bind IP for edge connections. Env: `TUNNEL_EDGE_BIND_ADDRESS`.
- `--region`: edge region override. Env: `TUNNEL_REGION`.
- `--ha-connections`: number of parallel edge connections. Default 4.
- `--grace-period`: shutdown wait for in-flight work. Default 30s. Env: `TUNNEL_GRACE_PERIOD`.
- `--compression-quality`: cross-stream compression level, default 0. Env: `TUNNEL_COMPRESSION_LEVEL`.
- `--metrics-update-freq`: metrics refresh cadence.
- `--management-diagnostics`: enables extra management endpoints. Default true in current tree. Env: `TUNNEL_MANAGEMENT_DIAGNOSTICS`.
- `--max-active-flows`: override private-network flow limits. Env: `TUNNEL_MAX_ACTIVE_FLOWS`.
- `--dns-resolver-addrs`: override virtual DNS service resolvers. Env: `TUNNEL_DNS_RESOLVER_ADDRS`.

Legacy or hidden flags still present for compatibility or internal use include:

- `--api-key`, `--api-email`, `--api-ca-key`: explicitly deprecated since 2017.10.1.
- `--proxy-connection-timeout`, `--proxy-expect-continue-timeout`: deprecated, no effect.
- `--ui`: deprecated, hidden.
- `--quick-service`: hidden quick tunnel service URL.
- `--stdin-control`: hidden stdin control path.
- `--use-reconnect-token`: hidden reconnect token experiment.
- QUIC flow control and PMTU flags: hidden tuning knobs.
- Removed `proxy-dns` flags are still wired only so existing scripts fail less abruptly when parsing.

### 6.3 Tunnel Command Contract

`cloudflared tunnel` is both a namespace and a runnable action.

If run directly without a subcommand, behavior is decision-based:

1. If `--name` is set, run ad hoc named tunnel workflow.
2. Else if quick-tunnel conditions are met, run a quick tunnel.
3. Else if config contains a tunnel ID, return guidance to use `cloudflared tunnel run`.
4. Else if `--hostname` is used as legacy classic tunnel syntax, reject as deprecated classic tunnels.
5. Else return the tunnel command usage error message.

This is important for breakage analysis because `cloudflared tunnel` is not a pure namespace command.

Major tunnel subcommands:

- `login`: obtain local origin certificate for named-tunnel management.
- `create`: create tunnel and local credentials file.
- `route`: manage hostname, load-balancer, and private IP routing.
- `vnet`: manage virtual networks for overlapping private IP spaces.
- `run`: run a named tunnel.
- `list`: list tunnels.
- `ready`: readiness helper.
- `info`: inspect a tunnel and its connectors.
- `ingress`: hidden config validation and rule-testing tools.
- `delete`: delete tunnel.
- `cleanup`: stop connectors and clean associated state.
- `token`: retrieve tunnel run token/credentials material.
- `diag`: collect troubleshooting information.

Important route subtree behavior:

- `route dns`: map hostname to tunnel via Cloudflare DNS.
- `route lb`: attach tunnel origin to a Cloudflare load balancer pool.
- `route ip`: manage private CIDR routes for WARP/private network access.

Important virtual-network behavior:

- Virtual networks exist to disambiguate overlapping private CIDRs.
- If omitted, routing uses the account default virtual network.
- Deleting with force can delete dependent resources or move them to the current default virtual network.

### 6.4 Access Command Contract

`cloudflared access` is a client toolset for Access-protected apps.

Subcommands:

- `login`: fetches scoped JWT using interactive browser flow.
- `curl`: wraps curl and injects Access token headers.
- `token`: prints JWT for an Access application.
- `tcp` with aliases `rdp`, `ssh`, `smb`: forwards Layer 4 traffic over the Access path.
- `ssh-config`: prints an SSH config snippet.
- `ssh-gen`: generates short-lived certificates for SSH workflows.

Notable flags and envs:

- `--hostname` / `TUNNEL_SERVICE_HOSTNAME`
- `--destination` / `TUNNEL_SERVICE_DESTINATION`
- `--url` / `TUNNEL_SERVICE_URL`
- `--service-token-id` / `TUNNEL_SERVICE_TOKEN_ID`
- `--service-token-secret` / `TUNNEL_SERVICE_TOKEN_SECRET`
- `--fedramp` toggles fedramp-specific access handling.

### 6.5 Tail Command Contract

`cloudflared tail [TUNNEL-ID]` streams logs from a remote connector through the management plane.

Operational contract:

- Acquires or accepts a management token.
- Opens a WebSocket to the management hostname.
- Can filter by connector, event type, log level, and sampling ratio.

Flags:

- `--connector-id`
- `--event` with values such as `cloudflared`, `http`, `tcp`, `udp`
- `--level` with `debug`, `info`, `warn`, `error`
- `--sample` in `(0.0, 1.0]`
- `--token`
- hidden `--trace`
- hidden `--management-hostname`, default `management.argotunnel.com`

### 6.6 Management Command Contract

`cloudflared management token --resource <logs|admin|host_details> TUNNEL_ID`

This command is hidden and returns JSON containing a scoped management JWT. It exists to support direct management-plane interactions and tooling.

### 6.7 Service Command Contract

Linux:

- `cloudflared service install`
- `cloudflared service uninstall`
- optional `--no-update-service`

Linux install behavior:

- Detects systemd vs SysV.
- Writes service files under `/etc/systemd/system` or `/etc/init.d`.
- Default systemd ExecStart includes `--no-autoupdate` and optional extra args.
- If autoupdate service is enabled, daily timer updates the binary and restarts cloudflared only when updater returns code 11.

macOS install behavior:

- Installs per-user launch agent or root launch daemon depending on privilege.
- User mode runs only when the user is logged in.

Windows install behavior:

- Adds Windows service named `Cloudflared`.
- Runtime distinguishes true service execution from false negatives in interactive-session detection.

Other OSes:

- Service commands exist but return not-supported errors.

## 7. Configuration Contract

Primary config source: YAML file parsed by `config.ReadConfigFile`.

### 7.1 Config Search And Creation Rules

Default config filenames:

- `config.yml`
- `config.yaml`

Default search directories:

- `~/.cloudflared`
- `~/.cloudflare-warp`
- `~/cloudflare-warp`
- `/etc/cloudflared` on non-Windows
- `/usr/local/etc/cloudflared` on non-Windows

Key behaviors:

- `FindDefaultConfigPath()` returns the first existing config path in search order.
- `FindOrCreateConfigPath()` will create the default config path and parent directory if needed, then write a minimal YAML containing `logDirectory`.
- Empty service-mode invocation therefore has file-system side effects.
- If config file is not explicitly set and not found, code uses `ErrNoConfigFile` rather than treating the absence as a parse failure.

### 7.2 Top-Level Config Keys

Stable, user-facing YAML keys observable in the current tree:

- `tunnel`: tunnel UUID for named tunnels.
- `ingress`: ordered list of ingress rules.
- `warp-routing`: private routing tuning.
- `originRequest`: default origin behavior for ingress rules.
- `logDirectory`: current root-level log directory written by auto-created config.

The config loader also retains unknown fields in a generic map so older settings can still be read via CLI context fallback, but strict re-decode is used to emit warnings for unknown keys.

### 7.3 Ingress Rule Schema

Per rule keys:

- `hostname`
- `path`
- `service`
- `originRequest`

Important ingress invariants:

- Last rule must be a catch-all without hostname or path restriction.
- Hostname wildcards may contain at most one wildcard and only as a subdomain pattern like `*.example.com`.
- Hostname must not contain a port.
- When matching requests, internal rules are checked before user rules.
- Internal rules report negative indices so logs and diagnostics can distinguish them from user rules.

Important service forms:

- `https://...`, `http://...`
- `unix:/path`
- `unix+tls:/path`
- `http_status:<code>`
- bastion and private-routing-specific services

Important compatibility rule:

- Multiple-origin ingress is incompatible with `--url`.

### 7.4 originRequest Schema

Documented repository keys:

- `connectTimeout`
- `tlsTimeout`
- `tcpKeepAlive`
- `noHappyEyeballs`
- `keepAliveConnections`
- `keepAliveTimeout`
- `httpHostHeader`
- `originServerName`
- `matchSNIToHost`
- `caPool`
- `noTLSVerify`
- `disableChunkedEncoding`
- `bastionMode`
- `proxyAddress`
- `proxyPort`
- `proxyType`
- `ipRules`
- `http2Origin`
- `access`

`access` sub-keys:

- `required`
- `teamName`
- `audTag`
- `environment`

`ipRules` sub-keys:

- `prefix`
- `ports`
- `allow`

### 7.5 warp-routing Schema

Current repository-visible keys:

- `connectTimeout`
- `maxActiveFlows`
- `tcpKeepAlive`

Historical quirk:

- `warp-routing.enabled` is no longer supported for local config as of 2023.9.0. The effective behavior now depends on whether private routes are configured rather than a local enable/disable flag.

### 7.6 Config Precedence

Precedence rules are distributed across CLI, config loading, and subcommand handling.

Operationally important cases:

- CLI flags override config-file values.
- `credentials-contents` overrides `credentials-file`.
- tunnel token overrides credentials material and also takes precedence over `token-file`.
- `--post-quantum` plus non-QUIC transport is invalid.
- `--unix-socket` must be exclusive relative to `--url` or positional origin URL.
- `--url` or hello-world plus no ingress rules defines a single-rule ingress model.

### 7.7 Credentials And Auth Files

Named tunnel credentials JSON contains fields such as:

- `AccountTag`
- `TunnelSecret`
- `TunnelID`
- optional `Endpoint`

Origin certificate file:

- Default file name `cert.pem`.
- Repository treats it as PEM-wrapped JSON containing zone/account/API token information.

## 8. Tunnel Behavior Model

### 8.1 StartServer Lifecycle

Tunnel daemon startup from `StartServer` performs these major steps:

1. Initialize Sentry.
2. Warn if running a locally configured tunnel without config file or token.
3. Optionally enable runtime trace output.
4. Create root context and graceful-shutdown path.
5. Start autoupdater goroutine.
6. Build observer and tunnel/orchestrator config.
7. Build management service and inject it as an internal ingress rule.
8. Open metrics listener.
9. Start metrics server with readiness and diagnostics.
10. Start tunnel supervisor.
11. Wait for error or graceful shutdown signal.

### 8.2 Quick Tunnels

Current quick-tunnel behavior:

- Must be explicitly invoked via `--url` or `--hello-world`-style path. It is no longer implicitly spun up by bare `cloudflared tunnel`.
- Intended for testing and experimentation, not production.
- Since 2023.3.2 quick tunnels make a single edge connection rather than the normal HA connector set.
- Quick tunnel URL is exposed to observers and the metrics `/quicktunnel` endpoint.
- Quick tunnels disable ICMP packet routing.

### 8.3 Named Tunnels

Named tunnels are the normal production model.

Observable properties:

- Identified by UUID plus credentials file.
- Can be created and managed via CLI or remote dashboard-driven flows.
- Support multiple simultaneous connector processes.
- Can be locally configured or remotely managed.

### 8.4 Remotely Managed Versus Locally Managed

The registration/control path can indicate whether a tunnel is remotely managed. That matters because local ingress and config assumptions are not always the source of truth when remote configuration is active.

Key implication:

- Absence of local ingress does not necessarily mean absence of effective ingress if remote management is in use.

### 8.5 Shutdown

Shutdown contract:

- First SIGINT or SIGTERM triggers graceful shutdown.
- cloudflared stops accepting new work, waits for in-flight work, and enforces grace-period timeout.
- Second interrupt can force faster termination depending on the environment path.
- Shutdown cancellation propagates through the root context to metrics and tunnel supervisor.

Historical quirk:

- QUIC shutdown used to ignore grace period, fixed by 2024.10.0.

## 9. Transport And Protocol Contracts

### 9.1 Supported Edge Transports

Current primary transports:

- QUIC over UDP.
- HTTP/2 over TCP.

Deprecated transport history:

- h2mux is removed and no longer supported.
- If legacy config still mentions h2mux, code paths intentionally route users away from it.

### 9.2 Protocol Selection

User-facing selector values:

- `auto`
- `quic`
- `http2`

Contract details:

- `auto` chooses among protocols over time and can refresh based on remote percentage fetches.
- QUIC can fall back to HTTP/2.
- HTTP/2 has no lower fallback.
- If user explicitly chooses a protocol, fallback behavior is intentionally restricted to preserve requested semantics.
- `--post-quantum` forces QUIC because HTTP/2 path does not support PQ mode.

Server name and ALPN details:

- HTTP/2 server name: `h2.cftunnel.com`
- QUIC server name: `quic.cftunnel.com`
- QUIC ALPN: `argotunnel`

### 9.3 Protocol Discovery

`auto` mode relies on edge discovery and percentage-based selection rather than a static preference only. The selector caches current protocol for a TTL and refreshes after expiry.

Historical quirk:

- Since 2022.8.1 cloudflared remembers successful protocol selection to avoid unnecessary fallback churn.

### 9.4 QUIC Contract

Repository-visible QUIC properties:

- Primary transport for modern tunnel connectivity.
- Supports streams and datagrams.
- Required for post-quantum mode.
- Has OS-specific listener behavior, including special handling on macOS for DF bit and UDP network choice.
- Supports path MTU discovery unless explicitly disabled.

Hidden tuning parameters exist for:

- path MTU discovery disablement
- connection-level flow control
- stream-level flow control

### 9.5 HTTP/2 Contract

Repository-visible HTTP/2 properties:

- Fallback transport when QUIC is unavailable or not selected.
- Carries stream traffic and control stream behavior.
- Does not support post-quantum mode.

### 9.6 QUIC Datagram Versions

The tree contains two datagram implementations:

- v2: RPC-assisted UDP session registration.
- v3: newer stateless-style datagram flow with different assumptions and unsupported session RPC calls.

Important v3 quirk:

- Register/unregister UDP session RPCs are intentionally unsupported in v3 code paths.

### 9.7 Cap'n Proto RPC Contract

Canonical schemas live under `tunnelrpc/proto`.

Current stable interfaces in use include:

- `RegistrationServer`
- `SessionManager`
- `ConfigurationManager`
- `CloudflaredServer`

Key contract objects:

- `TunnelAuth`
- `ClientInfo`
- `ConnectionOptions`
- `ConnectionResponse`
- `ConnectionError`
- `ConnectionDetails`
- `RegisterUdpSessionResponse`
- `UpdateConfigurationResponse`

Important semantics:

- Registration returns either an error object or connection details.
- Connection details include `tunnelIsRemotelyManaged`.
- Configuration update replies carry both latest applied version and optional error, allowing partial failure reporting with older config still active.

Deprecated but preserved schema objects remain in the capnp file specifically to maintain protocol compatibility shape.

### 9.8 Stream Framing And Metadata

QUIC protocol helper code defines magic bytes for stream identification and explicit version strings for data vs RPC streams. These are wire-level compatibility details that a rewrite must preserve if it retains existing peer compatibility.

## 10. Management, Metrics, And Diagnostics Contracts

### 10.1 Metrics Listener Contract

If `--metrics` is not explicitly set, current behavior is not “bind random port immediately.” It is:

1. Try a known deterministic-ish range.
2. Fall back to a random port only if all known ports are unavailable.

Known addresses:

- Host runtime: `localhost:20241` through `localhost:20245`
- Virtual runtime: `0.0.0.0:20241` through `0.0.0.0:20245`

Default bind addresses:

- Host runtime: `localhost:0`
- Virtual runtime: `0.0.0.0:0`

This behavior matters for diagnostics, because the diagnostic tooling searches known addresses and can fail if multiple or no listeners appear there.

### 10.2 Local Metrics Endpoints

Current endpoints exposed by metrics server:

- `/metrics`
- `/healthcheck`
- `/ready` when ready server is present
- `/quicktunnel`
- `/config` when orchestrator is present
- `/debug/` via default pprof mux
- diagnostics endpoints installed by `diagnostic.Handler`

Important response semantics:

- `/healthcheck` returns `OK` text.
- `/ready` returns JSON with `status`, `readyConnections`, and `connectorId`.
- `/ready` returns 200 only when active connection count is greater than zero; otherwise 503.
- `/quicktunnel` returns JSON containing hostname.
- `/config` returns the versioned config JSON or 500 with error text if config fetch fails.

### 10.3 Remote Management Service Contract

Management HTTP service exposes:

- `GET` and `HEAD` `/ping`
- `GET` `/logs`
- `GET` `/host_details`
- when diagnostics enabled: `GET` `/metrics`
- when diagnostics enabled: `GET` `/debug/pprof/{heap|goroutine}`

Management plane access is protected by token validation middleware.

`/host_details` returns JSON with:

- `connector_id`
- `ip` if derived from service-op path
- `hostname` or `custom:<label>` if connector label was provided

### 10.4 Log Streaming Contract

Remote log streaming behavior includes:

- WebSocket-based session.
- First client event must be a start-streaming event.
- Only one active streaming actor is allowed at a time, except same actor can preempt its own prior session.
- Idle timeout and session-limit violations terminate the connection with custom WebSocket status codes.

Repository-defined status codes:

- `4001`: invalid command / expected start streaming first
- `4002`: session limit exceeded
- `4003`: idle limit exceeded

### 10.5 Diagnostics Contract

Relevant user-visible diagnostics surfaces:

- `cloudflared tunnel diag`
- local metrics and pprof endpoints
- remote management diagnostics when enabled

Historical changes:

- 2023.7.0 introduced opt-in management diagnostics.
- 2024.2.1 enabled tunnel diagnostics by default, with opt-out available via `--management-diagnostics=false`.
- 2024.12.2 introduced local troubleshooting bundle command `cloudflared tunnel diag`.

## 11. Ingress, Routing, And Origin Contracts

### 11.1 Matching Model

Ingress matching order:

1. Internal rules
2. User rules in order
3. If somehow none match, use the last user rule as catch-all

The catch-all assumption is a design invariant validated during ingress parsing.

### 11.2 Default No-Rule Behavior

Current behavior when no ingress rules are found locally and no CLI single-origin substitute is supplied:

- cloudflared uses a default rule that returns HTTP 503 for incoming HTTP requests.

Historical quirk:

- Before 2023.3.1 the default behavior was effectively tied to localhost:8080 assumptions. That behavior was intentionally removed.

### 11.3 CLI-Origin Compatibility Model

When no multi-origin ingress is defined, CLI can synthesize a single origin from:

- `--url`
- `--unix-socket`
- `--hello-world`
- `--bastion`

Flags such as `http-host-header`, `origin-server-name`, `origin-ca-pool`, `no-tls-verify`, and several proxy timeout options are explicitly documented in code as legacy CLI-origin behavior. They only take effect when origin is defined by `--url` and ingress rules are not used.

### 11.4 Private Routing

Private routing is driven by:

- `tunnel route ip` commands
- virtual network association
- warp-routing and flow-limit settings

Critical invariants:

- IP routes may overlap only when disambiguated by virtual networks.
- Cloudflare WARP/private users reach those routes through account-linked tunnel/private-routing configuration.

## 12. Cloudflare API Contract Usage

`cfapi` is the client-side contract layer for Cloudflare REST APIs. Repository usage includes:

- tunnel CRUD
- hostname routing
- load balancer routing
- private IP route management
- virtual network management
- management token acquisition

Important note for breakage analysis:

- The repository depends on API path and response stability, but the server-side APIs are not fully defined in this repo. Only request types, filter usage, and command expectations in client code are locally auditable.

Default API base URL is hidden flag `--api-url`, defaulting to `https://api.cloudflare.com/client/v4`.

## 13. Architecture And Subsystem Boundaries

### 13.1 High-Level Runtime Flow

Eyeball request or private-route packet  
-> Cloudflare edge  
-> tunnel connection (QUIC or HTTP/2)  
-> connection handler  
-> orchestrator and ingress resolution  
-> proxy/origin service  
-> origin response or packet flow back to edge

### 13.2 Key Internal Boundaries

- `connection.TunnelConnection`: transport abstraction boundary.
- `orchestration.Orchestrator`: dynamic config and proxy ownership boundary.
- `ingress.Ingress`: matching and defaulting boundary.
- `metrics.ReadyServer`: readiness semantics boundary.
- `tunnelstate.ConnTracker`: active connection truth source for readiness/diagnostics.
- `management.ManagementService`: remote debug/log surface boundary.

### 13.3 Concurrency Model

Important concurrency properties visible in the tree:

- Supervisor maintains multiple long-lived connections concurrently.
- Metrics server, updater, and tunnel daemon all run in separate goroutines.
- Management service serializes streaming-session start decisions with a mutex.
- Protocol selector uses mutex-protected current state and TTL refresh.
- Orchestrator uses copy-on-write and atomic replacement style to avoid stopping traffic for config refreshes.

### 13.4 Rewrite Preservation Rules

Any rewrite must preserve at least these behaviors:

- empty invocation service-mode behavior
- ingress last-rule catch-all semantics
- default no-ingress 503 behavior
- graceful shutdown semantics and grace period
- quick-tunnel single-connection behavior
- metrics bind fallback order
- readiness requiring at least one active edge connection
- single-active log-stream session semantics
- QUIC/PQ/HTTP2 selection and incompatibility rules
- negative-index distinction for internal ingress rules in diagnostics/log reasoning

## 14. Code Organization

The codebase is organized around top-level packages rather than a monolithic `internal` tree. The primary buckets are:

- CLI and command registration: `cmd/cloudflared`
- runtime tunnel core: `supervisor`, `orchestration`, `connection`, `ingress`, `proxy`
- private routing and packet plumbing: `datagramsession`, `packet`, `flow`, `ipaccess`
- contracts and protocols: `tunnelrpc`, `cfapi`, `management`, `metrics`, `diagnostic`
- support subsystems: `logger`, `tracing`, `watcher`, `retry`, `tlsconfig`
- auth and client helpers: `credentials`, `token`, `access` command tree
- tooling and release: release scripts, Dockerfiles, packaging scripts, component tests

See `REPO_SOURCE_INDEX.md` for a package-by-package map.

## 15. External Components And Technologies

Critical external dependencies and roles:

- `quic-go` via Cloudflare fork replacement: QUIC transport.
- `urfave/cli/v2` via fork replacement: CLI framework.
- `zombiezen.com/go/capnproto2`: RPC schema and transport objects.
- `prometheus/client_golang`: metrics exposure.
- `go.opentelemetry.io/otel`: tracing.
- `rs/zerolog`: structured logging.
- `nhooyr.io/websocket` and `gorilla/websocket`: WebSocket handling.
- `go-systemd`: systemd notification and service support.
- `gopkg.in/yaml.v3`: config parsing.

Critical external services and endpoints:

- Cloudflare edge tunnel endpoints
- Cloudflare API v4
- `management.argotunnel.com`
- quick-tunnel service default `https://api.trycloudflare.com`
- update endpoints under `update.argotunnel.com` and staging equivalents

## 16. Build, Test, Packaging, And Release Contracts

Build and test entrypoints from `Makefile`:

- `make cloudflared`
- `make test`
- `make lint`
- `make vet`
- `make cover`
- `make fuzz`
- `make capnp`

Important build facts:

- Go version baseline: 1.24.
- Uses vendored modules.
- FIPS builds change binary naming and linking strategy.
- Container builds set metrics runtime to `virtual`, changing metrics default bind behavior.

Testing model:

- Go unit tests across packages.
- fuzz targets for packet, QUIC v3, tracing, and validation paths.
- Python component tests in `component-tests` require real tunnel credentials/config and exercise observable end-to-end behavior.

Packaging/release model:

- DEB, RPM, MSI packaging.
- system install scripts `postinst.sh` and `postrm.sh`.
- Dockerfiles for multiple architectures.

## 17. Historical Changes And Known Quirks

High-value changes from `CHANGES.md` that affect current reasoning:

- 2026.2.0 removed `proxy-dns` feature and its config/flags/commands.
- 2025.1.1 introduced new post-quantum curves for QUIC tunnel use.
- 2024.12.1 introduced semi-deterministic metrics port binding.
- 2024.10.0 fixed grace-period behavior for QUIC shutdown.
- 2024.2.1 enabled management diagnostics by default.
- 2023.9.0 removed support for local `warp-routing.enabled`.
- 2023.4.1 introduced `cloudflared tail`.
- 2023.3.2 made quick tunnels single-connection.
- 2023.3.1 changed no-ingress default behavior to HTTP 503 rather than localhost:8080 assumptions.
- 2023.2.2 deprecated legacy tunnels and removed h2mux support.
- 2022.8.1 added remembered successful protocol behavior.
- 2022.3.0 added `unix+tls:` ingress origin support.

Operational quirks worth preserving in mental model:

- Empty invocation may create config and log directories.
- Quick tunnels are intentionally not production-grade behavior.
- Metrics address default differs between host and virtual/container runtime.
- Management diagnostics exposure depends on runtime flag and token-gated access.
- Several CLI flags exist only for backward compatibility or script stability and should not be treated as current feature endorsements.

## 18. Bug-Hunting Hotspots

Highest-value files and subsystems to inspect when looking for regressions or latent defects:

- `cmd/cloudflared/tunnel/cmd.go`: startup, shutdown, flag interactions, service behavior.
- `supervisor/`: reconnection, fallback, HA, connection lifecycle.
- `connection/`: transport-specific stream/datagram behavior.
- `ingress/`: rule validation, catch-all semantics, compatibility between CLI and config.
- `orchestration/orchestrator.go`: hot reload and config application semantics.
- `management/service.go`: WebSocket session limits and auth-protected diagnostics.
- `metrics/metrics.go` and `metrics/readiness.go`: readiness and bind behavior.
- `tunnelrpc/proto/*.capnp`: protocol compatibility.
- `component-tests/`: externally visible behavior and assumptions.

Typical bug classes by subsystem:

- CLI/config: precedence bugs, hidden compatibility paths, missing validation.
- shutdown: grace-period leaks, double-signal behavior, goroutine leaks.
- transport: QUIC fallback loops, HTTP/2 incompatibility, flow-control tuning regressions.
- ingress: misordered rules, invalid wildcard handling, defaulting regressions.
- management: auth bypass exposure, streaming-session races, idle timeout errors.
- metrics/diagnostics: bind conflicts, accidental exposure, stale readiness.
- private routing: virtual-network ambiguity, flow-limit exhaustion, route lookup mismatches.

## 19. Rewrite Guidance

For a future rewrite, preserve these contracts first and re-derive internals second:

- CLI syntax and mode selection.
- config schema and precedence.
- edge connection selection and fallback behavior.
- ingress validation and match order.
- management and metrics endpoints and semantics.
- readiness definition.
- graceful shutdown behavior.
- run token, credentials, and remote-management compatibility.
- Cap'n Proto schema compatibility if interop with existing edge/control plane is required.

Suggested rewrite strategy:

1. Treat CLI, config, metrics/management endpoints, and RPC schema as contract fixtures.
2. Treat supervisor and transport layers as behavioral fixtures with state-machine tests.
3. Treat orchestrator, proxy, and ingress as semantics fixtures with route and failure-mode tests.
4. Replace internal implementation only after preserving timing, fallback, and shutdown guarantees.

## 20. Cross-References

- Package and topic map: `REPO_SOURCE_INDEX.md`
- Breakage and bug-hunt checklist: `REPO_AUDIT_CHECKLIST.md`
- Exhaustive command and flag inventory: `REPO_CLI_INVENTORY.md`
- Configuration and endpoint appendix: `REPO_CONFIG_CONTRACT.md`
- Components and dependency appendix: `REPO_COMPONENTS_AND_DEPENDENCIES.md`
- Behavioral specification: `REPO_BEHAVIORAL_SPEC.md`
- API contracts: `REPO_API_CONTRACTS.md`
- Architecture deep dive: `REPO_ARCHITECTURE_DEEP_DIVE.md`
- Quirks and compatibility: `REPO_QUIRKS_AND_COMPATIBILITY.md`
- Organization catalog: `REPO_ORGANIZATION_CATALOG.md`
- Mermaid diagrams: `REPO_DIAGRAMS.md`
