# Cloudflared Behavioral Specification

This document captures repository-visible behavior in operational terms. It is written for rewrite, regression, and bug-hunt work.

## 1. Behavioral Scope

Behavior here means:

- startup and mode selection
- runtime state transitions
- config and flag precedence effects
- request and datagram routing outcomes
- health, readiness, and diagnostics behavior
- failure, retry, and shutdown behavior

The focus is not only “what commands exist,” but “what the process does under each relevant condition.”

## 2. Root Process Behavior

### 2.1 Root Process Initialization

At process start, cloudflared does the following before command dispatch:

1. Sets `QUIC_GO_DISABLE_ECN=1`.
2. Registers build information metrics.
3. Calls `automaxprocs.Set()`.
4. Builds the root CLI app.
5. Installs tunnel, access, updater, tracing, token, tail, and management command modules.
6. Hands control to OS-specific `runApp` logic.

Operational implication:

- the binary always applies some runtime-global behavior before command selection
- even a simple `version` command runs after those initialization steps

### 2.2 Empty Invocation Behavior

An empty root invocation is special.

Condition:

- no args
- no flags

Effect:

- enters service mode rather than printing help
- watches config file using `watcher.NewFile()` and config manager machinery
- can create a config file path via `config.FindOrCreateConfigPath()`

This is behaviorally significant for rewrites because a naive CLI implementation could easily replace this with help output and silently break service users.

### 2.3 Non-Empty Root Invocation

If invocation is not empty and no more specific root command intercepts first, the root action executes tunnel behavior through `tunnel.TunnelCommand`.

## 3. Tunnel Mode Decision Behavior

### 3.1 Decision Order For `cloudflared tunnel`

When `cloudflared tunnel` is invoked without a subcommand, the behavior is selected in this order:

1. If `--name` is set: ad hoc named tunnel path.
2. Else if quick-tunnel conditions are met: quick tunnel path.
3. Else if config contains `tunnel:` UUID: return guidance to use `tunnel run`.
4. Else if `--hostname` is present in classic-tunnel style: return deprecated classic tunnel error.
5. Else return the tunnel usage error text.

### 3.2 Ad Hoc Named Tunnel Behavior

Condition:

- `--name` is provided to `cloudflared tunnel`

Behavior:

1. Validate hostname if supplied.
2. Reject case where hostname and URL are equal non-empty strings.
3. Look up whether a tunnel with the provided stable name already exists.
4. If not found, create it and optionally write credentials.
5. If route flags imply DNS or LB route, attempt provisioning.
6. Run the tunnel.

This mode is a compound create-route-run workflow rather than a thin alias.

### 3.3 Quick Tunnel Behavior

Trigger:

- `--url` or `--hello-world`
- quick-service path available

Behavioral properties:

- explicit, not implicit
- intended for testing and experimentation only
- current repo behavior disables ICMP packet routing for quick tunnels
- current repo behavior exposes the quick tunnel URL to observer sinks and metrics `/quicktunnel`
- quick tunnels use a single connection to edge since 2023.3.2

## 4. Named Tunnel Run Behavior

### 4.1 Tunnel Reference Resolution

`tunnel run` resolves target tunnel in this order:

1. explicit token from `--token`
2. token from `--token-file`
3. positional tunnel arg
4. `tunnel:` value from loaded config

Failure mode:

- if no token and no tunnel reference from arg or config, command errors

### 4.2 Credential Resolution Behavior

When token is not used:

- tunnel ID/name is resolved via API and credentials lookup path
- credentials contents can override file path when both are supplied

When token is used:

- encoded token is decoded to credentials and directly used
- invalid token is treated as usage error

### 4.3 PQ And Protocol Behavior

If PQ strict mode is active:

- selected transport must be QUIC or `auto`
- explicit non-QUIC transport fails startup
- effective transport becomes QUIC

## 5. Startup Behavior For Running Tunnel Daemon

`StartServer` behavior is the process runtime assembly path.

### 5.1 Core Startup Sequence

1. Initialize Sentry.
2. Create root process context for tunnel lifetime.
3. Start signal watcher for graceful shutdown.
4. Start autoupdater goroutine.
5. Build observer and prepare tunnel config.
6. Resolve service-operation IP fallback if possible via edge discovery.
7. Build management service with diagnostics mode and connector label.
8. Inject management service as internal ingress rule.
9. Build orchestrator.
10. Open metrics listener.
11. Start metrics server goroutine.
12. Start supervisor goroutine.
13. Wait for error or graceful shutdown signal.

### 5.2 Local Config Absence Warning Behavior

If config source is empty and no tunnel token is present:

- startup logs the no-config warning
- this is informational, not immediately fatal, because remote configuration or other inputs may still make startup valid

### 5.3 Trace Output Behavior

If `--trace-output` is set:

- a temp trace file is created
- runtime tracing starts into temp file
- on shutdown temp file is renamed to requested output path
- failures in rename/remove are logged

## 6. Ingress Behavior

### 6.1 Ingress Source Selection

Ingress comes from one of two sources:

1. explicit config-file ingress
2. synthesized single-origin CLI ingress

If config ingress exists and validates, it wins.

If config ingress is absent:

- CLI-derived ingress may be created from `--url`, `--unix-socket`, `--hello-world`, or `--bastion`

If neither exists:

- default ingress is created that returns HTTP 503 for incoming HTTP requests

### 6.2 Match Order Behavior

Ingress request matching order:

1. internal rules
2. user rules in configured order
3. final catch-all assumption

Behavioral detail:

- internal rule matches return negative indices
- user rule matches return non-negative indices

### 6.3 Catch-All Validation Behavior

During ingress validation:

- last rule must match all URLs
- a last rule containing hostname or path restriction is rejected

### 6.4 Punycode And IDN Matching Behavior

Ingress rules with internationalized domain names (IDN) are matched against both the original Unicode hostname and its punycode (ASCII) equivalent:

- During rule parsing, `idna.Lookup.ToASCII()` converts the hostname to punycode.
- If the punycode form differs from the original, both are stored (`hostname` and `punycodeHostname`).
- At match time, `Matches()` checks both: `matchHost(hostname) || matchHost(punycodeHostname)`.

This enables requests using either `müller.example.com` or `xn--mller-kva.example.com` to match the same ingress rule.

### 6.5 CLI-Origin Exclusivity Behavior

If `--unix-socket` is used with `--url` or positional origin arg:

- validation fails

If `--url` is set while multi-origin ingress is in effect:

- ingress validation fails because CLI single-origin mode and multi-origin mode are intentionally incompatible

## 7. HTTP Proxying Behavior

### 7.1 Request Tagging Behavior

Before proxying HTTP requests:

- cloudflared appends configured tag headers to request

### 7.2 Rule And Middleware Behavior

For each HTTP request:

1. find ingress rule
2. log request and selected rule
3. apply ingress middleware handlers in order
4. if middleware filters request, write status and stop
5. dispatch to service-specific proxy path

### 7.3 Origin Type Behavior

Possible behaviors by rule service type:

- `HTTPOriginProxy`: roundtrip-like HTTP behavior
- `StreamBasedOriginProxy`: stream upgrade / websocket / TCP-like behavior
- `HTTPLocalProxy`: local handler path

### 7.4 Chunked Encoding Behavior

If `disableChunkedEncoding` is enabled for applicable HTTP origin path:

- transfer encoding behavior is altered
- content-length may be preserved if available

### 7.5 User-Agent Behavior

If request lacks `User-Agent`:

- cloudflared explicitly sets it to empty string to avoid Go default insertion

## 8. TCP Proxy Behavior

For TCP flows:

1. increment TCP metrics
2. attempt to acquire a flow-limit slot
3. reject if too many concurrent flows
4. parse destination as `netip.AddrPort`
5. proxy stream to origin dialer
6. release flow-limit slot on exit

Behavioral implication:

- flow limiting is not advisory; it actively rejects new TCP flows when exhausted

## 9. Metrics And Readiness Behavior

### 9.1 Metrics Bind Selection Behavior

If `--metrics` is explicit:

- bind exactly that address or fail

If `--metrics` is default address:

1. try known-address list in order
2. if none available, bind random port

### 9.2 Readiness Behavior

Readiness uses active edge connection count from `tunnelstate.ConnTracker`.

Rule:

- if active connections > 0: HTTP 200
- else: HTTP 503

Response body includes:

- status
- readyConnections
- connectorId

### 9.3 Diagnostic Selection Behavior

`tunnel diag` behavior when no explicit metrics address is supplied:

- searches the known metrics addresses list

If zero instances found:

- returns a friendly “No instances found” path

If multiple instances found:

- reports each instance and instructs user to select one with `--metrics`

## 10. Management And Tail Behavior

### 10.1 Management Endpoint Exposure

Default management service behavior always exposes:

- `/ping`
- `/logs`
- `/host_details`

If diagnostics enabled:

- `/metrics`
- `/debug/pprof/heap`
- `/debug/pprof/goroutine`

### 10.2 Host Details Behavior

`/host_details` response behavior:

- always returns connector ID
- attempts to derive preferred private IP by dialing service-op target
- uses `custom:<label>` if connector label is set
- otherwise falls back to OS hostname or `unknown`

### 10.3 Log Streaming Session Behavior

Session lifecycle:

1. accept WebSocket
2. first event must be `start_streaming`
3. validate filters
4. ensure only one active actor session exists, except same actor can preempt itself
5. stream logs while session active
6. stop on client close, server error, idle timeout, or explicit stop

Close-code behavior:

- invalid first command: 4001
- session limit exceeded: 4002
- idle limit exceeded: 4003

### 10.4 Tail Client Behavior

Tail client behavior:

- if `--token` is absent, attempts to acquire management token automatically
- validates filter values locally before opening request
- interprets HTTP 530 as “no suitable connector available or reachable” condition

## 11. Protocol Selection And Fallback Behavior

### 11.1 Auto Protocol Behavior

Auto mode behavior:

- consults remote percentage fetcher
- selects first protocol above switch threshold
- if none exceed threshold, defaults to first protocol in preference list
- current preference list is QUIC then HTTP2

### 11.2 Fallback Behavior

- QUIC may fall back to HTTP/2
- HTTP/2 has no lower fallback
- explicit protocol choice should prevent undesired fallback in cases where feature support depends on chosen protocol

### 11.3 Remembered Success Behavior

Historical/current behavior:

- successful protocol connections influence future fallback behavior to avoid unnecessary demotion

## 12. Remote Config Behavior

### 12.1 Update Acceptance Behavior

Remote config updates are versioned.

Rule:

- if received version is older than or equal to current version, ignore update and keep current version

### 12.2 Config Update Failure Behavior

If remote config JSON fails to deserialize or apply:

- current version remains active
- update response returns latest applied version plus error text

### 12.3 Empty Ingress During Update

If a config update results in empty ingress:

- orchestrator inserts default 503 ingress rules rather than leaving origin proxy absent

## 13. Shutdown Behavior

### 13.1 Graceful Shutdown Sequence

On graceful shutdown signal:

1. root shutdown channel closes
2. wait for grace period if configured and not already terminated
3. cancel root server context
4. background components observe context cancellation and stop
5. wait group joins metrics, tunnel, and updater goroutines

### 13.2 Shutdown Edge Cases

- repeated shutdown attempts may accelerate termination on some service-manager paths
- metrics server startup uses a startup delay to avoid shutdown-before-serve race

## 14. Supervisor Retry And Reconnect Behavior

### 14.1 HA Connection Startup

The supervisor starts `HAConnections` tunnels (default 4 for named tunnels, 1 for quick tunnels).

Startup sequence:

1. Start first tunnel and wait for connection success signal.
2. Start remaining tunnels with 1-second spacing (`registrationInterval`).
3. Each tunnel gets its own `protocolFallback` state.

### 14.2 Reconnect Signal Behavior

`ReconnectSignal` carries a `Delay` duration.

Path:

1. Signal arrives on `reconnectCh` channel.
2. `listenReconnect()` goroutine in the connection errgroup receives it.
3. Calls `reconnect.DelayBeforeReconnect()` which sleeps for the specified delay.
4. Returns the signal as an error, aborting the current connection.
5. Supervisor detects `ReconnectSignal` type and immediately restarts that tunnel index without normal backoff.

Stdin-control feeds the `reconnectCh` from outside the supervisor.

### 14.3 Protocol Fallback Decision Logic

`protocolFallback` struct:

- embeds `retry.BackoffHandler`
- `protocol`: current transport (QUIC or HTTP2)
- `inFallback`: whether currently in fallback attempt

`selectNextProtocol()` decision:

1. Check `isQuicBroken(cause)` — true for QUIC `IdleTimeoutError` or `TransportError` with `operation not permitted`.
2. If backoff max retries reached OR (fallback available AND QUIC is broken): attempt fallback.
3. Fallback only occurs if `protocol != fallback` and a fallback protocol exists.
4. Returns `false` if no more fallback options available (stop retrying).

Critical rule:

- `HasConnectedWith(protocol)` in `tunnelstate.ConnTracker` checks if ANY connection succeeded with the current protocol. If true, the supervisor skips fallback because the protocol likely works and the failure is transient.

### 14.4 Retry And Backoff Behavior

Backoff parameters:

- `tunnelRetryDuration = 10 seconds` (base backoff delay)
- `TunnelConfig.Retries` = max retry attempts
- Formula: `maxWait = baseTime * (1 << retries)`, then `wait = rand(0, maxWait)`

Error classification:

- `DupConnRegisterTunnelError`: not retried; edge picks new address.
- `ServerRegisterTunnelError`: retried only if not permanent.
- `EdgeQuicDialError`: not retried immediately.
- `ReconnectSignal`: retried immediately with signal delay.
- `context.Canceled`: not retried.
- Default: retried unless `unrecoverableError`.

Grace period reset:

- When all tunnels connect successfully and no tunnels are waiting, `backoff.SetGracePeriod()` is called.
- This extends the deadline after which retry counters reset, preventing unnecessary protocol demotions after transient issues.

### 14.5 Supervisor Main Loop

The supervisor `Run()` loop monitors:

- `ctx.Done()`: wait for all tunnels to finish, return nil.
- `tunnelErrors`: decrement active count, classify error, queue for retry or reconnect.
- `backoffTimer`: restart all waiting tunnels after backoff expires.
- `nextConnectedSignal`: handle tunnel success, advance HA sequencing.
- `gracefulShutdownC`: set shutdown flag, stop restarting tunnels.

## 15. SOCKS Proxy Behavior

### 15.1 SOCKS5 Command Support

- `CONNECT` (command 1): fully implemented with IP access checks.
- `BIND` (command 2): not implemented; returns `commandNotSupported`.
- `ASSOCIATE` (command 3): not implemented; returns `commandNotSupported`.

### 15.2 SOCKS5 CONNECT Flow

1. If IP access policy is configured:
   - If destination IP is nil (FQDN-based), resolve via `net.ResolveIPAddr("ip", fqdn)`. Resolution failure returns `ruleFailure` (2).
   - Check `policy.Allowed(ip, port)`. Denied returns `ruleFailure` (2).
2. Dial destination via configured dialer.
3. Map dial errors to SOCKS reply codes: `connectionRefused` (5) for "refused", `networkUnreachable` (3) for "network is unreachable", `hostUnreachable` (4) otherwise.
4. On success, send `successReply` (0) with bound address.
5. Bidirectional copy between client and destination.

### 15.3 SOCKS5 Reply Codes

| Code | Name | Meaning |
| --- | --- | --- |
| 0 | successReply | Success |
| 1 | serverFailure | General server failure |
| 2 | ruleFailure | Connection denied by ruleset |
| 3 | networkUnreachable | Network unreachable |
| 4 | hostUnreachable | Host unreachable |
| 5 | connectionRefused | Connection refused |
| 6 | ttlExpired | TTL expired |
| 7 | commandNotSupported | Command not supported |
| 8 | addrTypeNotSupported | Address type not supported |

## 16. IP Access Rules Behavior

### 16.1 Policy Structure

`Policy` contains:

- `defaultAllow bool`: action when no rule matches.
- `rules []Rule`: ordered list of CIDR/port rules.

`Rule` contains:

- `ipNet *net.IPNet`: IPv4 or IPv6 CIDR block.
- `ports []int`: sorted port list; empty means all ports match.
- `allow bool`: allow or deny.

### 16.2 Evaluation Order

1. If no rules exist, return `defaultAllow`.
2. Linear scan through rules in definition order.
3. Per rule: check CIDR match via `ipNet.Contains(ip)`.
4. If IP matches: check ports. Empty port list = all ports match. Non-empty = binary search (`sort.SearchInts`).
5. First match wins: return the rule's `allow` decision.
6. If no rule matches after full scan: return `defaultAllow`.

Port validation: 1-65535 range enforced during rule creation.

## 17. Bastion Mode Behavior

### 17.1 Bastion Service Activation

Bastion mode activates when:

- ingress rule service is `"bastion"`, OR
- `originRequest.bastionMode` is true in config

### 17.2 Destination Resolution

`carrier.ResolveBastionDest(request)` extracts destination from `Cf-Access-Jump-Destination` header.

Behavior:

- Parses header value as URL to extract host.
- Strips scheme and path.
- Returns `hostname:port` string.
- Missing header returns error: "Did not receive final destination from client".

### 17.3 Bastion Proxy Behavior

- Uses `tcpOverWSService` with `isBastion=true`.
- No fixed destination; each request specifies its own via the jump-destination header.
- Stream handler is set based on `proxyType`: SOCKS handler or default bidirectional copy.
- Dialer timeout and keepalive come from `originRequest` config.

## 18. Access Command Behavior

### 18.1 Access Login Flow

1. Initialize Sentry.
2. Get app URL from args/flags.
3. Call `token.GetAppInfo()` to fetch app metadata via HEAD request (7s timeout).
4. Call `verifyTokenAtEdge()` to check/refresh token.
5. Call `token.GetAppTokenIfExists()` to retrieve stored JWT.
6. Output token based on `--quiet`/`--no-verbose` flags.

### 18.2 Access TCP/SSH/RDP/SMB Forwarding

- Creates WebSocket connection via `carrier.NewWSConnection(log)`.
- Sets `Cf-Access-Jump-Destination` header from `--destination` flag.
- If `--url` listener provided: `carrier.StartForwarder()` binds TCP listener and tunnels each accepted connection over WebSocket.
- If no listener: `carrier.StartClient()` reads from stdin/stdout.
- `--debug-stream` wraps the stream with payload logging.

### 18.3 Access SSH-Gen Behavior

1. Validate hostname.
2. Fetch app info via `token.GetAppInfo()`.
3. Obtain JWT via `token.FetchTokenWithRedirect()`.
4. Generate short-lived SSH certificate via `sshgen.GenerateShortLivedCertificate()`.

### 18.4 Access Token Lifecycle

Storage:

- App tokens stored at path derived from `GenerateAppTokenFilePathFromURL(appDomain, appAUD, "token")`.
- Org tokens stored at path derived from `generateOrgTokenFilePathFromURL(authDomain)`.
- JWT plaintext files with `0600` permissions.
- Lock files: `tokenPath + ".lock"` for multi-process safety.

Refresh flow:

1. Check existing app token on disk; return if not expired.
2. Try org token; if valid, exchange for app token via SSO endpoint, save and return.
3. Fall back to full auth flow via transfer service (browser login).

Expiry: `jwtPayload.Exp` compared against `time.Now().Unix()`. Expired tokens are deleted from disk.

Lock mechanism:

- File-based lock with exponential backoff (7 retries, `DefaultBaseTime`).
- SIGINT/SIGTERM signal handlers ensure lock cleanup on abort.
- Stale lock files are force-deleted after max retries.

## 19. Edge Discovery Behavior

### 19.1 Discovery Mechanism

- Primary: DNS SRV lookup for `_v2-origintunneld._tcp.argotunnel.com`.
- Fallback: DNS-over-TLS to `cloudflare-dns.com:853` if stdlib DNS fails.
- Each SRV target is resolved via `net.LookupIP()` to get IPv4/IPv6 addresses.
- Results sorted by priority and randomized by weight.

### 19.2 Edge Address Management

`Edge` struct methods:

| Method | Behavior |
| --- | --- |
| `GetAddrForRPC()` | Returns any available edge address for RPC |
| `GetAddr(connIndex)` | Prefers previously-used address; assigns new if needed |
| `GetDifferentAddr(connIndex, hasConnectivityError)` | Releases old, assigns new; optionally marks error |
| `AvailableAddrs()` | Count of unused addresses |
| `GiveBack(addr, hasConnectivityError)` | Returns address to pool; optionally marks error for backoff |

### 19.3 IPv4/IPv6 Selection

`ConfigIPVersion` values:

- `Auto` (2): select based on availability.
- `IPv4Only` (4): only IPv4.
- `IPv6Only` (6): only IPv6.

Each `EdgeAddr` carries `IPVersion` field: `V4` or `V6`.

## 20. Feature Flag Negotiation Behavior

### 20.1 Feature Constants

| Feature | String | Status |
| --- | --- | --- |
| Serialized headers | `serialized_headers` | active |
| Quick reconnects | `quick_reconnects` | active |
| Allow remote config | `allow_remote_config` | active |
| Datagram v2 | `support_datagram_v2` | active |
| Post-quantum | `postquantum` | active |
| QUIC EOF support | `support_quic_eof` | active |
| Management logs | `management_logs` | active |
| Datagram v3.2 | `support_datagram_v3_2` | active |
| Datagram v3 | `support_datagram_v3` | deprecated (TUN-9291) |
| Datagram v3.1 | `support_datagram_v3_1` | deprecated (TUN-9883) |

### 20.2 Default Features Sent During Registration

- `allow_remote_config`
- `serialized_headers`
- `support_datagram_v2`
- `support_quic_eof`
- `management_logs`

### 20.3 Feature Selection Priority

1. CLI `--features` flag takes precedence.
2. Remote evaluation via percentage-based rollout.
3. Default fallback: DatagramV2.

Deprecated features are automatically filtered out via `dedupAndRemoveFeatures()`.

Features are included in `ConnectionOptions.Client.Features` during Cap'n Proto `RegisterConnection()` RPC.

## 21. Datagram V2 Versus V3 Behavior

### 21.1 Datagram V2 Behavior

- UDP sessions registered via RPC: `RegisterUdpSession(sessionID, dstIP, dstPort, closeAfterIdleHint, traceContext)`.
- Registration checks flow limiter; rejects if too many active sessions.
- Creates UDP socket via `originDialer.DialUDP(addrPort)`.
- Session multiplexed by `datagramsession.Manager` via `DatagramMuxerV2`.
- Idle timeout and explicit unregister close sessions.

### 21.2 Datagram V3 Behavior

- No RPC-based session management.
- Sessions registered inline via datagrams (`UDPSessionRegistrationDatagram`).
- `RegisterUdpSession()` RPC call returns `ErrUnsupportedRPCUDPRegistration`.
- `UnregisterUdpSession()` RPC call returns `ErrUnsupportedRPCUDPUnregistration`.

Datagram types (v3):

| Type | Value | Meaning |
| --- | --- | --- |
| `UDPSessionRegistrationType` | 0x0 | Session registration request |
| `UDPSessionPayloadType` | 0x1 | Session payload |
| `ICMPType` | 0x2 | ICMP v4 or v6 |
| `UDPSessionRegistrationResponseType` | 0x3 | Registration response |

Response codes: `OK`, `DestinationUnreachable`, `UnableToBindSocket`, `TooManyActiveFlows`, `ErrorWithMsg`.

## 22. Origin Service Types Behavior

Complete list of origin service implementations:

| Type | String | Interface | Notes |
| --- | --- | --- | --- |
| `httpService` | full URL | HTTPOriginProxy | HTTP/HTTPS roundtrip |
| `unixSocketPath` | `unix:PATH` or `unix+tls:PATH` | HTTPOriginProxy | Unix socket |
| `rawTCPService` | `warp-routing` | StreamBasedOriginProxy | WARP routing TCP |
| `tcpOverWSService` | `ssh://`, `rdp://`, `smb://`, `tcp://` | StreamBasedOriginProxy | TCP-over-WebSocket |
| `tcpOverWSService` (bastion) | `bastion` | StreamBasedOriginProxy | Dynamic destination |
| `socksProxyOverWSService` | `socks-proxy` | StreamBasedOriginProxy | SOCKS5 proxy |
| `helloWorld` | `hello_world` | HTTPOriginProxy | Built-in test server on 127.0.0.1 dynamic port |
| `statusCode` | `http_status:CODE` | HTTPLocalProxy | Fixed status code responder; 503 for no-ingress default |
| `ManagementService` | `management` | HTTPLocalProxy | Management mux |

## 23. HTTP/2 Connection Type Dispatch Behavior

HTTP/2 streams are classified at request time using internal headers, checked in strict priority:

1. `Cf-Cloudflared-Proxy-Connection-Upgrade: update-configuration` → config update
2. `Cf-Cloudflared-Proxy-Connection-Upgrade: websocket` → WebSocket proxy
3. `Cf-Cloudflared-Proxy-Src` header present → TCP stream
4. `Cf-Cloudflared-Proxy-Connection-Upgrade: control-stream` → RPC control
5. None of above → standard HTTP

Behavioral details:

- TypeHTTP missing URL parts: fills `r.URL.Scheme = "http"` and `r.URL.Host = "localhost:8080"` if empty
- 101 Switching Protocols responses are rewritten to 200 OK (HTTP/2 spec disallows 101)
- Error responses: 502 Bad Gateway with ResponseMeta indicating source

## 24. Response Header And Meta Behavior

### 24.1 Header Serialization

User headers are serialized via base64-encoded key-value pairs joined by `;` delimiters and `:` separators:

- Encoding: `base64.RawStdEncoding` (no padding characters)
- Single header: `base64(name):base64(value)`
- Multiple headers: pairs joined by `;`

Deserialization reverses: split on `;`, split each on `:`, base64-decode both halves.

### 24.2 ResponseMeta Behavior

The `cf-cloudflared-response-meta` header communicates request outcome to the edge:

- Origin success: `{"src":"origin"}`
- cloudflared error: `{"src":"cloudflared"}`
- Flow rate limited: `{"src":"cloudflared","flow_rate_limited":true}`

These are pre-generated constants (not dynamically serialized per-request).

### 24.3 Control Header Filtering

Headers starting with `:`, `cf-int-`, `cf-cloudflared-`, or `cf-proxy-` are classified as control headers and filtered from user-visible response headers.

## 25. Proxy Tag Header And LB Probe Behavior

### 25.1 Tag Headers

All configured tunnel tags are added to proxied requests with the prefix `Cf-Warp-Tag-`:

- Tag `{Name: "env", Value: "prod"}` becomes header `Cf-Warp-Tag-env: prod`

### 25.2 Load Balancer Probe Detection

Requests with User-Agent starting with `"Mozilla/5.0 (compatible; Cloudflare-Traffic-Manager/1.0;"` are identified as LB probes. The `LBProbe` field is set on `TCPRequest` structs for these requests.

### 25.3 Response Flushing Heuristics

Streaming response detection for flush-on-write behavior:

- No `Content-Length` header → flush
- `Transfer-Encoding` contains `chunked` → flush
- `Content-Type` starts with `text/event-stream`, `application/grpc`, or `application/x-ndjson` → flush
- Connection types `TypeWebsocket`, `TypeTCP`, `TypeControlStream` → always flush

## 26. QUIC Stream Identification Behavior

QUIC streams are identified by a 6-byte protocol signature followed by a 2-byte version:

- Data streams: `[0x0A, 0x36, 0xCD, 0x12, 0xA1, 0x3E]` + `"01"`
- RPC streams: `[0x52, 0xBB, 0x82, 0x5C, 0xDB, 0x65]` + `"01"`

The first stream opened on a QUIC connection is always the control (RPC) stream. Subsequent streams are accepted via the `acceptStream` loop and dispatched by signature.

QUIC response headers are encoded as `pogs.Metadata` key-value pairs with format `HttpHeader:<HeaderName>` for the key.

## 27. QUIC Port Reuse And Platform-Specific Behavior

QUIC connections reuse UDP ports per connection index via a `portForConnIndex` map.

Platform quirk: on macOS, `"udp4"` or `"udp6"` is used explicitly (not `"udp"`) due to a quic-go DF (Don't Fragment) bit issue on darwin.

## 28. Datagram V2 Suffix Encoding Behavior

V2 datagrams use suffix-based encoding to avoid payload copy:

- Wire layout: `[payload][sessionID:16 bytes][typeID:1 byte]`
- Session ID is a UUID serialized as 16 raw bytes
- Type ID is the last byte of the datagram

Type values: 0 = UDP, 1 = IP, 2 = IP+trace, 3 = tracing spans.

V2 session management happens over RPC streams, not inline in datagrams.

## 29. Datagram V3 Prefix Encoding Behavior

V3 datagrams use prefix-based encoding with type byte at offset 0.

Session registration is inline — no RPC calls. V3 explicitly rejects RPC-based `RegisterUdpSession` and `UnregisterUdpSession` with `ErrUnsupportedRPCUDPRegistration`.

V3 session constants:

- Default idle timeout: 210 seconds (3.5 minutes)
- Max origin UDP packet size: 1500 bytes
- Write channel capacity: 512
- Demux channel capacity: 16
- ICMP datagram channel capacity: 128

Session registration duplicate handling:

- Already registered on same connection → resend OK response (retry flow)
- Bound to different connection → migrate session
- Rate limited → respond with `ResponseTooManyActiveFlows`

Session manager error messages:

- `"flow not found"` — session not in map
- `"flow is in use by another connection"` — migration needed
- `"flow is already registered for this connection"` — duplicate
- `"flow registration rate limited"` — limiter rejected

## 30. Behavioral Invariants For Rewrite

- empty root invocation must remain service behavior
- no-ingress default must remain HTTP 503 behavior
- ingress internal rules must remain higher priority than user rules
- readiness must remain tied to active edge connections
- metrics must preserve known-address probing behavior
- management must preserve token gating and single-active-stream semantics
- PQ mode must remain QUIC-only
- quick tunnels must remain explicit and single-connection unless intentionally changed
- supervisor must preserve HA connection spacing and reconnect-signal bypass of backoff
- protocol fallback must preserve HasConnectedWith() skip logic
- SOCKS BIND and ASSOCIATE must remain unsupported until explicitly implemented
- IP access rules must preserve first-match-wins evaluation order
- bastion destination must come from Cf-Access-Jump-Destination header
- access token lifecycle must preserve file lock and expiry semantics
- edge discovery must preserve SRV-then-DoT fallback
- datagram v3 must reject RPC-based session registration
- header serialization must use base64 raw std encoding with `;`/`:` delimiters
- ResponseMeta must use pre-generated JSON constants, not dynamic serialization
- HTTP/2 stream type dispatch must follow the 5-level priority order
- QUIC stream signature bytes and version must remain exact
- 101 Switching Protocols must be rewritten to 200 OK over HTTP/2
- tag headers must use `Cf-Warp-Tag-` prefix
- LB probe detection must match exact Cloudflare-Traffic-Manager user-agent prefix
- V2 suffix encoding order (payload, sessionID, typeID) must be preserved
- V3 prefix encoding byte offsets and flag bits must be preserved exactly
- V3 RequestID is uint128 (not UUID) — serialization differs
