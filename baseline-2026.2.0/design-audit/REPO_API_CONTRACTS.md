# Cloudflared API Contracts

This document consolidates repository-visible API contracts across local HTTP endpoints, management WebSocket events, Cloudflare REST client interfaces, token/certificate encodings, and RPC schemas.

## 1. API Taxonomy

The repo contains five distinct API families:

1. Local process HTTP endpoints.
2. Management service HTTP and WebSocket endpoints.
3. Cloudflare REST client contracts used by CLI flows.
4. Tunnel RPC contracts over Cap'n Proto.
5. Local file-format and token encoding contracts.

## 2. Local Process HTTP API

### 2.1 Metrics Server Endpoints

| Endpoint | Method | Auth | Response |
| --- | --- | --- | --- |
| `/metrics` | GET | none | Prometheus metrics |
| `/healthcheck` | GET | none | text `OK` |
| `/ready` | GET | none | readiness JSON and 200/503 |
| `/quicktunnel` | GET | none | JSON hostname payload |
| `/config` | GET | none | versioned config JSON or 500 error text |
| `/debug/...` | GET | none | pprof and debug handlers |

### 2.2 `/ready` Contract

Response JSON shape:

```json
{
  "status": 200,
  "readyConnections": 1,
  "connectorId": "uuid"
}
```

Semantics:

- `status` mirrors HTTP status code
- `readyConnections` is active edge connection count
- `connectorId` is the connector UUID

### 2.3 `/quicktunnel` Contract

Response shape:

```json
{"hostname":"example.trycloudflare.com"}
```

## 3. Management Service API

### 3.1 HTTP Endpoints

| Endpoint | Method | Auth | Meaning |
| --- | --- | --- | --- |
| `/ping` | GET, HEAD | token query middleware | liveness |
| `/logs` | GET -> WebSocket | token query middleware | log streaming session |
| `/host_details` | GET | token query middleware | connector identity and host data |
| `/metrics` | GET | token query middleware and diagnostics enabled | remote metrics |
| `/debug/pprof/{heap\|goroutine}` | GET | token query middleware and diagnostics enabled | remote pprof |

### 3.2 `/host_details` Response Contract

Response shape:

```json
{
  "connector_id": "uuid",
  "ip": "10.0.0.4",
  "hostname": "custom:label-or-hostname"
}
```

Field semantics:

- `connector_id` is required
- `ip` may be omitted if derivation fails
- `hostname` is connector label if provided, else local hostname, else `unknown`

### 3.3 WebSocket Event Contract

#### Client Event Types

- `start_streaming`
- `stop_streaming`

#### Server Event Types

- `logs`

#### `start_streaming` Payload

```json
{
  "type": "start_streaming",
  "filters": {
    "events": ["http", "tcp"],
    "level": "warn",
    "sampling": 0.5
  }
}
```

#### `logs` Payload

```json
{
  "type": "logs",
  "logs": [
    {
      "time": "...",
      "level": "info",
      "message": "...",
      "event": "cloudflared",
      "fields": {}
    }
  ]
}
```

#### Log Event Enumerations

Allowed `event` values:

- `cloudflared`
- `http`
- `tcp`
- `udp`

Allowed log levels:

- `debug`
- `info`
- `warn`
- `error`

### 3.4 WebSocket Failure Contract

Repository-defined close status codes:

- `4001`: invalid command or wrong first event
- `4002`: session limit exceeded
- `4003`: session idle too long

## 4. Cloudflare REST Client Contract

The repository uses `cfapi.Client` as the main internal abstraction over Cloudflare HTTP APIs.

### 4.1 Client Interface Families

#### Tunnel Client

- `CreateTunnel(name, tunnelSecret)`
- `GetTunnel(tunnelID)`
- `GetTunnelToken(tunnelID)`
- `GetManagementToken(tunnelID, resource)`
- `DeleteTunnel(tunnelID, cascade)`
- `ListTunnels(filter)`
- `ListActiveClients(tunnelID)`
- `CleanupConnections(tunnelID, params)`

#### Hostname Client

- `RouteTunnel(tunnelID, route)`

#### IP Route Client

- `ListRoutes(filter)`
- `AddRoute(newRoute)`
- `DeleteRoute(id)`
- `GetByIP(params)`

#### Vnet Client

- `CreateVirtualNetwork(newVnet)`
- `ListVirtualNetworks(filter)`
- `DeleteVirtualNetwork(id, force)`
- `UpdateVirtualNetwork(id, updates)`

### 4.2 Base Endpoint Construction Contract

From `NewRESTClient`:

- account-level tunnel endpoint: `/accounts/{accountTag}/cfd_tunnel`
- account routes endpoint: `/accounts/{accountTag}/teamnet/routes`
- account virtual networks endpoint: `/accounts/{accountTag}/teamnet/virtual_networks`
- zone-level tunnel endpoint: `/zones/{zoneTag}/tunnels`

### 4.3 HTTP Request Contract

Every request includes:

- `User-Agent`
- `Authorization: Bearer <token>`
- `Accept: application/json;version=1`
- `Content-Type: application/json` when body is present

### 4.4 Response Envelope Contract

Expected envelope shape:

```json
{
  "success": true,
  "errors": [],
  "messages": [],
  "result": ...,
  "result_info": {
    "count": 1,
    "page": 1,
    "per_page": 20,
    "total_count": 1
  }
}
```

Behavior:

- envelope is parsed first
- `errors` are converted into client-side error values
- `success=false` without parseable errors is treated as `ErrAPINoSuccess`

### 4.5 Status Code To Error Contract

Mapped errors:

- 200 -> success
- 400 -> `ErrBadRequest`
- 401 or 403 -> `ErrUnauthorized`
- 404 -> `ErrNotFound`
- all others -> formatted API failure error

## 5. CLI Filter Contracts For API Queries

### 5.1 Tunnel List Filters

Query parameters represented by `TunnelFilter`:

- `name`
- `name_prefix`
- `exclude_prefix`
- `is_deleted=false` for default non-deleted path
- `existed_at`
- `uuid`
- `per_page`
- `page`

### 5.2 IP Route Filters

Query parameters represented by `IpRouteFilter`:

- `tun_types=cfd_tunnel`
- `is_deleted`
- `network_subset`
- `network_superset`
- `comment`
- `tunnel_id`
- `virtual_network_id`
- `per_page`
- `page`

### 5.3 Virtual Network Filters

Query parameters represented by `VnetFilter`:

- `id`
- `name`
- `is_default`
- `is_deleted`
- `per_page`

## 6. Cap'n Proto RPC Contract

### 6.1 Registration Schema

Core interfaces from `tunnelrpc.capnp`:

- `RegistrationServer`
- `SessionManager`
- `ConfigurationManager`
- `CloudflaredServer`

### 6.2 RegistrationServer Methods

- `registerConnection(auth, tunnelId, connIndex, options) -> ConnectionResponse`
- `unregisterConnection()`
- `updateLocalConfiguration(config)`

### 6.3 SessionManager Methods

- `registerUdpSession(sessionId, dstIp, dstPort, closeAfterIdleHint, traceContext="") -> RegisterUdpSessionResponse`
- `unregisterUdpSession(sessionId, message)`

### 6.4 ConfigurationManager Methods

- `updateConfiguration(version, config) -> UpdateConfigurationResponse`

### 6.5 Important Cap'n Proto Structs

`TunnelAuth`:

- `accountTag`
- `tunnelSecret`

`ClientInfo`:

- `clientId`
- `features`
- `version`
- `arch`

`ConnectionOptions`:

- `client`
- `originLocalIp`
- `replaceExisting`
- `compressionQuality`
- `numPreviousAttempts`

`ConnectionResponse` union:

- `error`
- `connectionDetails`

`ConnectionDetails`:

- `uuid`
- `locationName`
- `tunnelIsRemotelyManaged`

`UpdateConfigurationResponse`:

- `latestAppliedVersion`
- `err`

### 6.6 QUIC Metadata Protocol Schema

From `quic_metadata_protocol.capnp`:

`ConnectRequest`:

- `dest`
- `type`
- `metadata`

`ConnectionType` values:

- `http`
- `websocket`
- `tcp`

`Metadata`:

- `key`
- `val`

`ConnectResponse`:

- `error`
- `metadata`

## 7. Internal Interface Contracts Relevant To Rewrite

### 7.1 `connection.TunnelConnection`

- single method `Serve(ctx context.Context) error`

Contract implication:

- transport implementations are expected to run until error or context cancellation

### 7.2 `connection.Orchestrator`

- `UpdateConfig(version, config)`
- `GetConfigJSON()`
- `GetOriginProxy()`

Contract implication:

- connection layer depends on orchestrator as runtime-config and origin-proxy provider

### 7.3 `connection.OriginProxy`

- `ProxyHTTP(w, req, isWebsocket)`
- `ProxyTCP(ctx, rwa, req)`

Contract implication:

- this is the core data-plane bridge contract between edge-facing code and origin-facing code

### 7.4 `management` Event Types

- event names and payloads are part of the effective management protocol contract and must be preserved if compatibility is required

## 8. File Format Contracts

### 8.1 Tunnel Token Encoding

`connection.TunnelToken` JSON field mapping:

- `a` -> account tag
- `s` -> tunnel secret
- `t` -> tunnel UUID
- `e` -> endpoint optional

Encoded token behavior:

- JSON marshaled then base64 encoded

### 8.2 Origin Certificate Encoding

PEM block type used by current encoding:

- `ARGO TUNNEL TOKEN`

JSON fields:

- `zoneID`
- `accountID`
- `apiToken`
- `endpoint` optional

## 9. Feature Flags Sent During Registration

Features are included in `ConnectionOptions.Client.Features` during registration RPC.

Default features:

- `allow_remote_config`
- `serialized_headers`
- `support_datagram_v2`
- `support_quic_eof`
- `management_logs`

Optional features added by CLI or remote rollout:

- `support_datagram_v3_2`
- `postquantum`
- `quick_reconnects`

Deprecated features automatically filtered:

- `support_datagram_v3` (TUN-9291)
- `support_datagram_v3_1` (TUN-9883)

## 10. Account Versus Zone Level API Endpoints

The REST client constructs two sets of endpoint roots:

- Account level: `/accounts/{accountTag}/cfd_tunnel` — used for tunnel CRUD/token/management flows.
- Zone level: `/zones/{zoneTag}/tunnels` — used for zone-scoped hostname routing.
- Account routes: `/accounts/{accountTag}/teamnet/routes` — IP route management.
- Account vnets: `/accounts/{accountTag}/teamnet/virtual_networks` — virtual network management.

Both `accountTag` and `zoneTag` are derived from the origin certificate (`cert.pem`).

## 11. Management Token Resource Types

`GetManagementToken(tunnelID, resource)` supports three resource scopes:

| Resource | Meaning |
| --- | --- |
| `logs` | Log streaming access |
| `admin` | Administrative access |
| `host_details` | Host details read access |

## 12. Cleanup Parameters Contract

`CleanupConnections(tunnelID, params)` accepts `CleanupParams`:

- `ForClient(clientID)` method encodes `connector_id` query parameter to clean only connections from a specific connector.
- Without `ForClient()`, all stale connections for the tunnel are cleaned.

## 13. Datagram V2 Versus V3 Wire Differences

### 13.1 V2 Wire Contract

- Session registration via Cap'n Proto RPC: `RegisterUdpSession()` / `UnregisterUdpSession()`.
- Payload multiplexed through `DatagramMuxerV2`.
- Session manager maps session IDs to UDP sockets.

### 13.2 V3 Wire Contract

- Session registration via inline datagrams (binary, not RPC).
- RPC registration calls rejected with `ErrUnsupportedRPCUDPRegistration`.
- RPC unregistration calls rejected with `ErrUnsupportedRPCUDPUnregistration`.

Datagram types:

| Type | Value | Meaning |
| --- | --- | --- |
| `UDPSessionRegistrationType` | 0x0 | Registration request |
| `UDPSessionPayloadType` | 0x1 | Session payload |
| `ICMPType` | 0x2 | ICMP packet |
| `UDPSessionRegistrationResponseType` | 0x3 | Registration response |

### 13.3 V3 Response Codes

- `OK`
- `DestinationUnreachable`
- `UnableToBindSocket`
- `TooManyActiveFlows`
- `ErrorWithMsg`

## 14. Edge Discovery DNS Contract

Edge resolution uses DNS:

- SRV record: `_v2-origintunneld._tcp.argotunnel.com`
- Fallback: DNS-over-TLS to `cloudflare-dns.com:853`
- Each SRV target resolved via `net.LookupIP()` for A/AAAA records
- Results sorted by priority, randomized by weight

## 15. Cap'n Proto Deprecated RPC Methods

The `tunnelrpc.capnp` schema retains deprecated methods for compatibility:

- Legacy `TunnelServer` auth/registration methods marked `DEPRECATED`
- Legacy `ClientService` methods marked `DEPRECATED`
- These remain in schema to avoid ID reuse collisions but should not be called in new code

## 16. Internal Transport Header Contracts

### 16.1 Header Constants

These headers are set by the edge or by cloudflared internally and are wire-visible:

| Header Name | Value | Direction | Meaning |
| --- | --- | --- | --- |
| `cf-cloudflared-request-headers` | serialized base64 headers | edge → cloudflared | User request headers |
| `cf-cloudflared-response-headers` | serialized base64 headers | cloudflared → edge | User response headers |
| `cf-cloudflared-response-meta` | JSON | cloudflared → edge | Response metadata (source, flow limiting) |
| `Cf-Cloudflared-Proxy-Connection-Upgrade` | upgrade type string | internal (HTTP/2) | Stream type indicator |
| `Cf-Cloudflared-Proxy-Src` | source string | internal (HTTP/2) | TCP proxy source marker |

### 16.2 Header Serialization Wire Format

User headers are serialized for transport over HTTP/2 and QUIC:

- Encoding: `base64.RawStdEncoding` (no padding)
- Format: `base64(name):base64(value)` pairs joined by `;`
- Example: `dGVzdA:dmFsdWU;aG9zdA:ZXhhbXBsZQ` (two headers: test:value, host:example)

Deserialization splits on `;`, then each pair on `:`, then base64-decodes each half.

### 16.3 `ResponseMeta` Wire Format

```go
type responseMetaHeader struct {
    Source          string `json:"src"`
    FlowRateLimited bool   `json:"flow_rate_limited,omitempty"`
}
```

Pre-generated values (no dynamic serialization at request time):

| Source | JSON |
| --- | --- |
| origin success | `{"src":"origin"}` |
| cloudflared error | `{"src":"cloudflared"}` |
| flow rate limited | `{"src":"cloudflared","flow_rate_limited":true}` |

### 16.4 Control Header Detection

`IsControlResponseHeader` returns true for headers starting with: `:`, `cf-int-`, `cf-cloudflared-`, or `cf-proxy-`. These are stripped from user-visible response headers.

### 16.5 HTTP/2 Stream Type Dispatch

HTTP/2 streams are classified by internal header values, checked in this priority:

1. `Cf-Cloudflared-Proxy-Connection-Upgrade: update-configuration` → `TypeConfiguration`
2. `Cf-Cloudflared-Proxy-Connection-Upgrade: websocket` → `TypeWebsocket`
3. `Cf-Cloudflared-Proxy-Src` present → `TypeTCP`
4. `Cf-Cloudflared-Proxy-Connection-Upgrade: control-stream` → `TypeControlStream`
5. default → `TypeHTTP`

Connection type enum:

| Value | Name | Meaning |
| --- | --- | --- |
| 0 | `TypeWebsocket` | WebSocket upgrade |
| 1 | `TypeTCP` | TCP stream |
| 2 | `TypeControlStream` | RPC control stream |
| 3 | `TypeHTTP` | Standard HTTP request |
| 4 | `TypeConfiguration` | Remote config update |

### 16.6 HTTP/2 Response Rewriting

- `101 Switching Protocols` is rewritten to `200 OK` (HTTP/2 disallows 101)
- Tracing header `cf-int-cloudflared-tracing` is promoted to `Cf-Cloudflared-Tracing`
- Error responses use `502 Bad Gateway`; flow-limited errors include `flow_rate_limited` meta

## 17. QUIC StreamProtocol Signatures And Constants

### 17.1 Stream Protocol Signatures

QUIC streams are identified by a 6-byte signature followed by a 2-byte version:

| Stream Type | Signature Bytes | Version |
| --- | --- | --- |
| Data stream | `0x0A 0x36 0xCD 0x12 0xA1 0x3E` | `"01"` (2 bytes) |
| RPC stream | `0x52 0xBB 0x82 0x5C 0xDB 0x65` | `"01"` (2 bytes) |

Total preamble: 8 bytes (signature + version). First stream opened is always the control (RPC) stream.

### 17.2 QUIC Metadata Keys

| Key | Meaning |
| --- | --- |
| `HttpHeader` | Prefix for HTTP headers (format: `HttpHeader:<Name>`) |
| `HttpMethod` | HTTP method |
| `HttpHost` | HTTP host |
| `HttpStatus` | Response status code |
| `FlowID` | Flow identifier |

### 17.3 Protocol TLS Settings

| Protocol | ServerName | ALPN/NextProtos |
| --- | --- | --- |
| HTTP/2 | `h2.cftunnel.com` | (none) |
| QUIC | `quic.cftunnel.com` | `["argotunnel"]` |

### 17.4 QUIC MaxDatagramFrameSize

| Platform | MaxDatagramFrameSize | maxDatagramPayloadSize |
| --- | --- | --- |
| Unix/macOS | 1350 | 1280 |
| Windows | 1220 | 1200 |

### 17.5 MaxConcurrentStreams

HTTP/2 server sets `MaxConcurrentStreams = math.MaxUint32` (4294967295).

## 18. Datagram V2 Binary Wire Format

V2 uses **suffix-based** encoding (type and session ID appended after payload):

```text
[payload bytes][session ID: 16 bytes (UUID)][type ID: 1 byte]
```

Type enum (suffix byte):

| Value | Name | Meaning |
| --- | --- | --- |
| 0 | `DatagramTypeUDP` | UDP payload |
| 1 | `DatagramTypeIP` | Full IP packet |
| 2 | `DatagramTypeIPWithTrace` | IP packet with tracing ID |
| 3 | `DatagramTypeTracingSpan` | Tracing spans (protobuf) |

Session ID is a standard UUID serialized as 16 raw bytes at positions `[len-17..len-2]`.

## 19. Datagram V3 Binary Wire Format

V3 uses **prefix-based** encoding (type byte at offset 0).

### 19.1 Type Enum

| Value | Name | Meaning |
| --- | --- | --- |
| `0x0` | `UDPSessionRegistrationType` | Session registration |
| `0x1` | `UDPSessionPayloadType` | Session UDP payload |
| `0x2` | `ICMPType` | ICMP packet |
| `0x3` | `UDPSessionRegistrationResponseType` | Registration response |

### 19.2 Registration Datagram Layout

```text
Offset  Field                     Size
0       Type (0x0)                1 byte
1       Flags                     1 byte
2-3     Destination Port          2 bytes (big-endian)
4-5     Idle Duration Seconds     2 bytes (big-endian)
6-21    Session ID (RequestID)    16 bytes
22-25   IPv4 Address              4 bytes (if IPv4)
  OR
22-37   IPv6 Address              16 bytes (if IPv6)
26/38+  Bundle Payload            variable (if bundled)
```

Flags byte:

| Bit | Mask | Meaning |
| --- | --- | --- |
| 0 | `0b0000_0001` | IPv6 (0=IPv4, 1=IPv6) |
| 1 | `0b0000_0010` | Traced |
| 2 | `0b0000_0100` | Bundled (has payload in same datagram) |

Header sizes: IPv4 = 26 bytes, IPv6 = 38 bytes.

### 19.3 Payload Datagram Layout

```text
Offset  Field                     Size
0       Type (0x1)                1 byte
1-16    Session ID (RequestID)    16 bytes
17+     Payload                   variable
```

Payload header: 17 bytes fixed.

### 19.4 Registration Response Layout

```text
Offset  Field                     Size
0       Type (0x3)                1 byte
1       Response Code             1 byte
2-17    Session ID (RequestID)    16 bytes
18-19   Error Message Length      2 bytes (big-endian)
20+     Error Message             variable
```

Response codes:

| Value | Name |
| --- | --- |
| `0x00` | `ResponseOk` |
| `0x01` | `ResponseDestinationUnreachable` |
| `0x02` | `ResponseUnableToBindSocket` |
| `0x03` | `ResponseTooManyActiveFlows` |
| `0xff` | `ResponseErrorWithMsg` |

### 19.5 ICMP Datagram Layout

```text
Offset  Field        Size
0       Type (0x2)   1 byte
1+      Payload      variable (max 1280 bytes)
```

### 19.6 RequestID Format

RequestID is a custom `uint128` type (NOT a UUID):

```go
type uint128 struct {
    hi uint64
    lo uint64
}
```

Serialized as 16 bytes big-endian (hi first). String format: `%016x%016x` (32 hex chars).

## 20. Prometheus Metrics Contract

### 20.1 Tunnel Metrics (namespace: `cloudflared`)

| Name | Subsystem | Type | Labels | Description |
| --- | --- | --- | --- | --- |
| `local_config_pushes` | `config` | Counter | — | Local config pushes to edge |
| `local_config_pushes_errors` | `config` | Counter | — | Errors pushing config |
| `max_concurrent_requests_per_tunnel` | `tunnel` | GaugeVec | `connection_id` | Max concurrent requests |
| `server_locations` | `tunnel` | GaugeVec | `connection_id`, `edge_location` | Edge server locations |
| `tunnel_rpc_fail` | `tunnel` | CounterVec | `error`, `rpcName` | RPC connection errors |
| `tunnel_register_fail` | `tunnel` | CounterVec | `error`, `rpcName` | Registration errors |
| `user_hostnames_counts` | `tunnel` | CounterVec | `userHostname` | User hostnames |
| `tunnel_register_success` | `tunnel` | CounterVec | `rpcName` | Successful registrations |

### 20.2 Proxy Metrics (namespace: `cloudflared`)

| Name | Subsystem | Type | Labels | Description |
| --- | --- | --- | --- | --- |
| `total_requests` | `tunnel` | Counter | — | Total proxied requests |
| `concurrent_requests_per_tunnel` | `tunnel` | Gauge | — | Concurrent requests |
| `response_by_code` | `tunnel` | CounterVec | `status_code` | Responses by HTTP status |
| `request_errors` | `tunnel` | Counter | — | Origin proxy errors |
| `active_sessions` | `tcp` | Gauge | — | Concurrent TCP sessions |
| `total_sessions` | `tcp` | Counter | — | Total TCP sessions |
| `connect_latency` | `proxy` | Histogram | — | Connection latency (ms) |
| `connect_streams_errors` | `proxy` | Counter | — | Failed stream connections |

Histogram buckets for `connect_latency`: `[1, 10, 25, 50, 100, 500, 1000, 5000]` ms.

### 20.3 UDP/ICMP Metrics (namespace: `cloudflared`)

| Name | Subsystem | Type | Labels |
| --- | --- | --- | --- |
| `active_flows` | `udp` | GaugeVec | `conn_index` |
| `total_flows` | `udp` | CounterVec | `conn_index` |
| `failed_flows` | `udp` | CounterVec | `conn_index` |
| `retry_flow_responses` | `udp` | CounterVec | `conn_index` |
| `migrated_flows` | `udp` | CounterVec | `conn_index` |
| `unsupported_remote_command_total` | `udp` | CounterVec | `conn_index`, `command` |
| `dropped_datagrams` | `udp` | CounterVec | `conn_index`, `reason` |
| `dropped_packets` | `icmp` | CounterVec | `conn_index`, `reason` |

Drop reason labels:

| Reason | Meaning |
| --- | --- |
| `write_failed` | Write to QUIC failed |
| `write_deadline_exceeded` | Write deadline timeout |
| `write_full` | Write channel at capacity |
| `write_flow_unknown` | Flow not found in session map |
| `read_failed` | Read from origin failed |
| `read_too_large` | Origin packet exceeds max size |

### 20.4 Orchestration Metric

| Name | Type | Description |
| --- | --- | --- |
| `cloudflared_orchestration_config_version` | Gauge | Current applied config version |

## 21. Management WebSocket Internals

### 21.1 JWT Token Claims Structure

```go
type managementTokenClaims struct {
    Tunnel tunnel `json:"tun"`
    Actor  actor  `json:"actor"`
    jwt.Claims
}
type tunnel struct {
    ID         string `json:"id"`
    AccountTag string `json:"account_tag"`
}
type actor struct {
    ID      string `json:"id"`
    Support bool   `json:"support"`
}
```

JWT is parsed with `go-jose/v4`, algorithm `ES256`. **Not verified locally** — uses `UnsafeClaimsWithoutVerification` (edge verifies).

FedRAMP detection: `IsFed()` returns true if `Issuer == "fed-tunnelstore"`.

### 21.2 WebSocket Close Codes

| Code | Constant | Meaning |
| --- | --- | --- |
| 4001 | `StatusInvalidCommand` | Expected start_streaming as first event |
| 4002 | `StatusSessionLimitExceeded` | Too many streaming sessions |
| 4003 | `StatusIdleLimitExceeded` | Session idle timeout |

### 21.3 Event Types

Client events: `start_streaming`, `stop_streaming`.

Server events: `logs`.

### 21.4 Log Event Types And Levels

Log event types:

| Value | Name |
| --- | --- |
| 0 | `Cloudflared` |
| 1 | `HTTP` |
| 2 | `TCP` |
| 3 | `UDP` |

Log levels:

| Value | Name |
| --- | --- |
| 0 | `Debug` |
| 1 | `Info` |
| 2 | `Warn` |
| 3 | `Error` |

### 21.5 Log Wire Format

```json
{
  "time": "...",
  "level": 1,
  "message": "...",
  "event": 0,
  "fields": {}
}
```

### 21.6 Streaming Filters

```json
{
  "events": [0, 1],
  "level": 1,
  "sampling": 0.5
}
```

Sampling is clamped to `[0, 1]`.

### 21.7 Session Internals

- Log buffer: 30-element channel (`logWindow = 30`)
- Single active session per actor (new session preempts same-actor old session)
- Idle timeout: 5 minutes
- Heartbeat ping: every 15 seconds
- Logger implements `zerolog.LevelWriter` — parses zerolog JSON and distributes to sessions

## 22. API Breakage Review Questions

- Does this change any endpoint path, response shape, or status-code rule?
- Does this change any management WebSocket event name, payload field, or close code?
- Does this change the RPC schema or method ordering/signature?
- Does this change token or certificate encoding?
- Does this change the REST request envelope or response parsing assumptions?
- Does this change account vs zone endpoint routing?
- Does this change feature flag names or default feature list?
- Does this change datagram type values or registration/response wire format?
- Does this change management token resource scope values?
- Does this change header serialization format (base64, delimiters)?
- Does this change ResponseMeta JSON shape or pre-generated values?
- Does this change HTTP/2 stream type dispatch priority?
- Does this change QUIC stream protocol signatures or version bytes?
- Does this change any Prometheus metric name, label, or type?
- Does this change WebSocket close codes or session limits?
- Does this change datagram V2 suffix encoding order or V3 prefix byte offsets?
