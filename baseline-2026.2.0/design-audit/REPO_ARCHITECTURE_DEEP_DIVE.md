# Cloudflared Architecture Deep Dive

This document describes the runtime architecture, subsystem boundaries, dependency directions, and rewrite-preservation rules.

Path scope note: package names and file paths in this document refer to the Go
reference tree under `old-impl/` unless explicitly stated otherwise.

## 1. Architectural Style

cloudflared is a multi-surface connector daemon with a command-driven entry layer and a long-running runtime core.

Architecturally it is best described as:

- command-oriented at the edge
- service-oriented in process structure
- transport-multiplexed in its data plane
- copy-on-write in its runtime config model

## 2. Layered View

### 2.1 Layer 1: Command And Environment Layer

Packages:

- `cmd/cloudflared`
- `cmd/cloudflared/tunnel`
- `cmd/cloudflared/access`
- `cmd/cloudflared/tail`
- `cmd/cloudflared/management`
- `cmd/cloudflared/cliutil`
- `cmd/cloudflared/flags`

Responsibilities:

- process startup
- OS-specific service-manager integration
- CLI parsing
- config path selection
- token/credential acquisition path selection

### 2.2 Layer 2: Runtime Assembly Layer

Packages:

- `cmd/cloudflared/tunnel`
- `orchestration`
- `supervisor`
- `tunnelstate`

Responsibilities:

- construct runtime object graph
- start metrics, management, and updater goroutines
- connect signal/shutdown path
- maintain tunnel lifecycle

### 2.3 Layer 3: Transport And Protocol Layer

Packages:

- `connection`
- `quic`
- `tunnelrpc`
- `datagramsession`

Responsibilities:

- connect to edge using QUIC or HTTP/2
- multiplex streams and datagrams
- carry registration/configuration RPC traffic
- expose origin-facing proxy contract

### 2.4 Layer 4: Routing And Origin Layer

Packages:

- `ingress`
- `proxy`
- `flow`
- `ipaccess`
- `packet`

Responsibilities:

- match incoming traffic to origin targets
- apply middleware and access checks
- proxy HTTP and TCP traffic
- enforce connection-flow limits
- support private routing and packet handling

### 2.5 Layer 5: Observability And Control Layer

Packages:

- `metrics`
- `management`
- `diagnostic`
- `logger`
- `tracing`

Responsibilities:

- readiness and metrics
- management sessions and remote diagnostics
- troubleshooting collection
- logging and tracing propagation

### 2.6 Layer 6: Integration Layer

Packages:

- `cfapi`
- `credentials`
- `config`
- `edgediscovery`
- `features`
- `tlsconfig`

Responsibilities:

- Cloudflare HTTP API interaction
- origin cert and tunnel credential handling
- edge resolution and feature selection
- TLS config construction

## 3. Dependency Direction

The intended dependency direction is generally:

- command layer depends on runtime layers
- runtime assembly depends on transport, routing, and observability
- transport depends on routing contracts and RPC schemas
- routing depends on config/origin abstractions and support packages
- observability depends on runtime state trackers

The main anti-pattern to avoid in rewrites is letting low-level transport code grow direct knowledge of high-level CLI or config parsing.

## 4. Core Internal Boundaries

### 4.1 `TunnelConnection`

Boundary between supervisor and transport implementations.

Meaning:

- supervisor should treat transport as a black box that runs until error or cancellation

### 4.2 `Orchestrator`

Boundary between connection layer and hot-reload config/origin state.

Meaning:

- transport code can ask for origin proxy and push config updates without owning ingress state directly

### 4.3 `OriginProxy`

Boundary between edge-facing request handling and origin-facing forwarding.

Meaning:

- connection layer should not know details of origin routing implementation

### 4.4 `Ingress`

Boundary between request classification and origin selection.

Meaning:

- matching policy is centralized and ordered

### 4.5 `ConnTracker`

Boundary between actual connection lifecycle and health/readiness exposure.

Meaning:

- readiness depends on tracked active connections, not on process aliveness alone

## 5. Runtime Object Graph

At startup the runtime graph is approximately:

1. CLI context and config inputs
2. observer and connection tracker
3. management service
4. orchestrator with internal rules and origin dialer service
5. metrics server with readiness and diagnostics handlers
6. supervisor with tunnel config and reconnect channel

Key graph property:

- management service is injected as internal ingress, not bolted on as a completely separate server in the tunnel path

## 6. State And Concurrency Model

### 6.1 Goroutine Domains

Major goroutine domains:

- updater
- metrics server
- tunnel supervisor
- optional stdin control
- signal watcher
- management session readers/writers

### 6.2 Shared-State Patterns

Used patterns include:

- mutex-protected mutable config version
- `atomic.Value` for proxy replacement
- channels for reconnect and shutdown signaling
- context cancellation for service-wide stop

### 6.3 Copy-On-Write Config Pattern

The orchestrator updates ingress/proxy by:

1. validating/constructing new ingress and origin set
2. starting new origins before closing old ones
3. replacing current origin proxy atomically
4. closing the prior proxy shutdown channel

Architectural reason:

- avoid downtime during config updates

Rewrite implication:

- preserving hot reload requires preserving this no-gap swap behavior, not just “ability to reload config eventually”

## 7. Architecture Of Request Handling

### 7.1 HTTP/WebSocket Path

1. edge transport receives request
2. connection layer materializes request context and metadata
3. proxy asks ingress to match host/path
4. middleware may filter request
5. selected origin service handles roundtrip or stream
6. response is written back to edge-facing writer

### 7.2 TCP Path

1. edge transport passes `TCPRequest`
2. proxy acquires flow slot
3. destination is parsed
4. origin dialer opens stream/connection
5. data is copied bidirectionally
6. flow slot is released

### 7.3 Datagram/Private Routing Path

1. QUIC/datagram path receives registration or payload
2. datagram/session manager maps session state
3. origin packet or UDP flow is serviced
4. idle timeout and cleanup rules reclaim resources

## 8. Architecture Of Service Installation And Runtime Modes

### 8.1 Linux

- service install emits systemd or SysV templates
- auto-update service/timer may be installed separately

### 8.2 macOS

- root install emits launch daemon
- user install emits launch agent

### 8.3 Windows

- service manager runtime can control graceful shutdown through a shared channel

Architectural implication:

- root CLI and runtime are not completely decoupled from OS service manager semantics

## 9. Architectural Hotspots

These are the subsystems where architecture mistakes are most likely to cause subtle regressions.

### 9.1 `cmd/cloudflared/tunnel`

- too much behavior converges here: mode selection, startup, diagnostics, PQ, config precedence, service behavior

### 9.2 `supervisor`

- owns retry/fallback/reconnect behavior, where semantic regressions are easy even if tests are green

### 9.3 `connection`

- transport and request adaptation layer is where wire compatibility meets origin proxy expectations

### 9.4 `orchestration`

- hot-reload correctness depends on ordering and atomic swap semantics

### 9.5 `management` and `metrics`

- security-sensitive operational surfaces; mistakes can expose more than intended or break support tooling

## 10. Rewrite Preservation Rules

If rewritten from scratch, preserve:

- CLI mode selection behavior
- config search and auto-create side effects
- ingress match order and catch-all requirement
- metrics bind probing behavior
- readiness definition
- management token-gated surface and WebSocket event model
- QUIC/HTTP2/PQ selection rules
- hot-reload no-gap proxy replacement
- single-connection quick-tunnel behavior
- service-manager integration semantics per OS

## 11. Supervisor And Reconnect Architecture

### 11.1 Supervisor Structure

The supervisor owns:

- `config *TunnelConfig`: immutable tunnel configuration
- `orchestrator`: runtime config and proxy manager
- `edgeIPs *edgediscovery.Edge`: edge address pool
- `tunnelErrors chan tunnelError`: error channel per tunnel index
- `tunnelsProtocolFallback map[int]*protocolFallback`: per-tunnel backoff state
- `reconnectCh chan ReconnectSignal`: external reconnect trigger
- `gracefulShutdownC <-chan struct{}`: shutdown signal

### 11.2 Protocol Fallback Architecture

`protocolFallback` embeds `retry.BackoffHandler` and adds:

- `protocol`: current transport enum
- `inFallback`: whether currently in a fallback attempt

State transitions:

- `reset()`: clear backoff retries and inFallback flag on successful connection.
- `fallback(fallbackProtocol)`: reset backoff, switch protocol, set inFallback=true.

Decision path in `selectNextProtocol()`:

1. Is QUIC broken (specific error types)?
2. Has backoff reached max retries?
3. Is a fallback protocol available and different from current?
4. Has any connection succeeded with current protocol (ConnTracker)?

If ConnTracker shows success with current protocol: skip fallback. This prevents transient failures from demoting a working protocol.

### 11.3 HA Connection Architecture

- HA connections are launched sequentially with 1-second spacing.
- Each connection runs in its own goroutine.
- Success is signaled per-connection via `nextConnectedSignal` channels.
- First connection success unblocks the main `connectedSignal` to indicate process readiness.

## 12. Edge Discovery Architecture

### 12.1 DNS-Based Resolution

`edgediscovery.ResolveEdge()` performs:

1. SRV lookup for `_v2-origintunneld._tcp.argotunnel.com`.
2. If SRV fails: DNS-over-TLS fallback to `cloudflare-dns.com:853`.
3. A/AAAA resolution of each SRV target.
4. Addresses organized into region-based pools.

### 12.2 Address Pool Architecture

The `Edge` struct wraps `allregions.Regions` which manages:

- Per-region address pools.
- Used/unused tracking.
- Connectivity error marking for backoff.

Connections are assigned addresses via `GetAddr(connIndex)` which prefers previously-used addresses for connection stability.

### 12.3 Static Edge Testing

`StaticEdge()` constructor allows hardcoded edge addresses for testing, bypassing DNS.

## 13. Bastion And SOCKS Proxy Architecture

### 13.1 Bastion Service Architecture

`tcpOverWSService` with `isBastion=true`:

- No fixed destination; destination resolved per-request from `Cf-Access-Jump-Destination` header.
- Stream handler chosen at startup based on `proxyType`: SOCKS handler or default bidirectional copy.
- No listener is started; connection handling happens per-client inline.

### 13.2 SOCKS Proxy Architecture

`socksProxyOverWSService` wraps a connection object with an `accessPolicy`.

`StandardRequestHandler`:

- `dialer Dialer`: interface for dialing destinations.
- `accessPolicy *ipaccess.Policy`: IP access control.

Request flow: SOCKS5 command parsing -> IP access check (resolve FQDN if needed) -> dial destination -> bidirectional proxy.

### 13.3 IP Access Policy Architecture

`ipaccess.Policy`:

- `defaultAllow bool`: fallback decision when no rule matches.
- `rules []Rule`: ordered list, first-match-wins.

`Rule`:

- `ipNet *net.IPNet`: CIDR block.
- `ports []int`: sorted port list; empty = all ports.
- `allow bool`: allow or deny.

Evaluation: linear scan, first CIDR+port match wins, then default.

## 14. Access Token Caching Architecture

### 14.1 Token Storage

- App tokens: file at path derived from `(appDomain, appAUD, "token")`.
- Org tokens: file at path derived from `authDomain`.
- JWT plaintext with `0600` permissions.

### 14.2 Lock Architecture

File-based locking with:

- Lock path: `tokenPath + ".lock"`.
- Exponential backoff (7 retries) for waiting on existing lock.
- Signal handlers (SIGINT, SIGTERM) registered during lock hold for cleanup.
- Stale locks force-deleted after max retries.

### 14.3 Token Refresh Architecture

Three-level fallback:

1. Return existing app token if not expired.
2. Exchange org token for app token via SSO endpoint.
3. Full browser-based auth via transfer service.

Expired tokens are deleted from disk on detection.

## 15. Datagram Session Architecture

### 15.1 V2 Architecture

- `sessionManager datagramsession.Manager`: tracks active UDP sessions.
- `datagramMuxer *cfdquic.DatagramMuxerV2`: multiplexes datagrams on QUIC connection.
- `flowLimiter cfdflow.Limiter`: limits concurrent UDP sessions.
- `packetRouter *ingress.PacketRouter`: routes ICMP packets.

Session lifecycle: RPC registration -> socket creation -> bidirectional muxing -> idle timeout or explicit unregister.

### 15.2 V3 Architecture

- `datagramMuxer cfdquic.DatagramConn`: handles all datagram processing inline.
- No RPC session management; registration embedded in datagram frames.
- Type-length-value datagram format with session registration, payload, ICMP, and response types.

## 16. Replaceable Versus Non-Replaceable Internals

### 16.1 Replaceable Internals

- logger implementation details
- internal data structures behind trackers and proxy wrappers
- exact goroutine layout, if semantics remain intact
- HTTP client plumbing behind `cfapi`, if request/response contract remains intact

### 16.2 Non-Replaceable Semantics

- public CLI syntax and env bindings
- config schema and precedence
- API and RPC wire contracts
- endpoint shapes and management session semantics
- readiness, shutdown, and fallback behavior
- supervisor retry/backoff/reconnect-signal semantics
- edge discovery DNS and address pool management
- bastion destination header contract
- IP access rule evaluation order
- access token lock file and expiry semantics
- datagram v2 vs v3 wire format distinction
- header serialization wire format (base64, delimiters)
- QUIC stream protocol signatures (exact bytes)
- HTTP/2 stream type dispatch priority order
- ResponseMeta JSON constants
- Prometheus metric names and label schemas
- management WebSocket close codes and event types

## 17. Origin Dialer Architecture

### 17.1 OriginDialerService

The `OriginDialerService` in `ingress/origin_dialer.go` provides centralized TCP/UDP origin dialing with reserved-service routing:

- `reservedTCPServices map[netip.AddrPort]OriginTCPDialer`: static TCP targets
- `reservedUDPServices map[netip.AddrPort]OriginUDPDialer`: static UDP targets
- `defaultDialer OriginDialer`: fallback dialer (protected by `sync.RWMutex`)
- `writeTimeout time.Duration`: UDP write deadline

Dial dispatch: checks reserved services map first; falls back to default dialer.

The default `Dialer` wraps `net.Dialer` with `Timeout = ConnectTimeout` and `KeepAlive = TCPKeepAlive` from `WarpRoutingConfig`.

UDP write deadline: `writeDeadlineUDP = 200ms`.

### 17.2 Hot-Swap During Config Update

The orchestrator's `updateIngress` method swaps the origin dialer atomically during config hot-reload. The dialer is re-created from the new warp-routing config.

## 18. Orchestrator Start-Before-Stop Pattern

The config update path in `orchestration/orchestrator.go` follows a start-before-stop pattern:

1. Parse new ingress rules and warp routing config.
2. Override remote warp routing values with local CLI flags (e.g., `MaxActiveFlows`).
3. Assign internal management/diagnostic rules.
4. Start new origin services (`ingressRules.StartOrigins`).
5. Update flow limiter from warp routing config.
6. Update origin dialer from warp routing config.
7. Create new `proxy.Proxy` and store via `atomic.Value`.
8. Close previous proxy by closing `o.proxyShutdownC` channel.

Version logic: starts at `-1`. Any remote version `≥ 0` overrides. `UpdateConfig` rejects if `currentVersion >= version`.

This ordering ensures the new proxy is ready to serve before the old one is shut down, preventing request drops during config transitions.

## 19. Proxy Request Dispatch Architecture

The proxy dispatches based on origin service interface type:

- `ingress.HTTPOriginProxy` → `proxyHTTPRequest` (HTTP round-trip to origin)
- `ingress.StreamBasedOriginProxy` → `proxyStream` (TCP/WS dial and bidirectional copy)
- `ingress.HTTPLocalProxy` → `proxyLocalRequest` (local handler serving request)

Bastion dispatch: proxy checks `rule.Service.String() == "bastion"` and calls `carrier.ResolveBastionDest(req)` to determine target.
