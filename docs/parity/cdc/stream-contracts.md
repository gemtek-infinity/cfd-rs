# CDC Feature Group: Stream Contracts

## Scope

This document covers the per-stream request/response contracts used over QUIC
data streams (and HTTP/2 muxed streams), including the `ConnectRequest` and
`ConnectResponse` schemas, wire framing, metadata conventions, and internal
transport headers.

## Frozen Baseline Schema

Source: `baseline-2026.2.0/old-impl/tunnelrpc/proto/quic_metadata_protocol.capnp`

### ConnectRequest (`@0xc47116a1045e4061`)

| Field | Type | Ordinal |
| --- | --- | --- |
| `dest` | Text | @0 |
| `type` | ConnectionType | @1 |
| `metadata` | List(Metadata) | @2 |

### ConnectionType (`@0xc52e1bac26d379c8`)

| Value | Name | Numeric |
| --- | --- | --- |
| `http` | HTTP | 0 |
| `websocket` | WebSocket | 1 |
| `tcp` | TCP | 2 |

### Metadata (`@0xe1446b97bfd1cd37`)

| Field | Type | Ordinal |
| --- | --- | --- |
| `key` | Text | @0 |
| `val` | Text | @1 |

### ConnectResponse (`@0xb1032ec91cef8727`)

| Field | Type | Ordinal |
| --- | --- | --- |
| `error` | Text | @0 |
| `metadata` | List(Metadata) | @1 |

## Wire Framing

Source: `baseline-2026.2.0/old-impl/tunnelrpc/pogs/quic_metadata_protocol.go`

### Go Serialization

The Go baseline serializes `ConnectRequest` and `ConnectResponse` as Cap'n
Proto binary messages. The `ToPogs()` and `FromPogs()` methods marshal and
unmarshal via the `capnp-go` library.

### Metadata Key Conventions

Known metadata keys used by the frozen baseline:

| Key | Purpose | Source |
| --- | --- | --- |
| `HttpMethod` | HTTP request method | stream metadata |
| `HttpHost` | HTTP Host header value | stream metadata |
| `HttpHeader:<name>` | arbitrary HTTP header (prefixed) | stream metadata |
| `HttpStatus` | HTTP response status code | response metadata |
| `FlowID` | flow identifier for multiplexing | stream metadata |
| `cf-trace-id` | distributed tracing | stream metadata |
| `HttpHeader:Content-Length` | body length | stream metadata |

### Response Metadata

The `ConnectResponse` `metadata` field carries response state back from the
origin proxy. Key patterns:

- `HttpStatus` for HTTP response status code
- `HttpHeader:<name>` for response headers
- flow rate limiting flags via metadata

## Transport Header Contracts

Source: `baseline-2026.2.0/old-impl/connection/header.go` and design audit

### Internal Headers (Wire-Visible)

| Header | Direction | Meaning |
| --- | --- | --- |
| `cf-cloudflared-request-headers` | edge → cloudflared | base64-encoded HTTP/1 request headers |
| `cf-cloudflared-response-headers` | cloudflared → edge | base64-encoded HTTP/1 response headers |
| `cf-cloudflared-response-meta` | cloudflared → edge | JSON response metadata |
| `Cf-Cloudflared-Proxy-Connection-Upgrade` | internal (HTTP/2) | stream type indicator |
| `Cf-Cloudflared-Proxy-Src` | internal (HTTP/2) | TCP proxy source marker |

### Header Serialization Format

- encoding: `base64.RawStdEncoding` (no padding)
- format: `base64(name):base64(value)` pairs joined by `;`
- example: `dGVzdA:dmFsdWU;aG9zdA:ZXhhbXBsZQ` (headers: test:value,
  host:example)

### ResponseMeta Wire Format

```json
{"src": "origin"}
{"src": "cloudflared"}
{"src": "cloudflared", "flow_rate_limited": true}
```

Pre-generated at init time; no dynamic serialization per-request.

### Control Header Detection

`IsControlResponseHeader` returns true for headers with prefixes: `:`,
`cf-int-`, `cf-cloudflared-`, `cf-proxy-`. These are stripped from
user-visible response headers.

### HTTP/2 Stream Type Dispatch

Checked in priority order:

1. `Cf-Cloudflared-Proxy-Connection-Upgrade: update-configuration` →
   `TypeConfiguration`
2. `Cf-Cloudflared-Proxy-Connection-Upgrade: websocket` → `TypeWebsocket`
3. `Cf-Cloudflared-Proxy-Src` present → `TypeTCP`
4. `Cf-Cloudflared-Proxy-Connection-Upgrade: control-stream` →
   `TypeControlStream`
5. default → `TypeHTTP`

## Current Rust Stream Surface

Source: `crates/cloudflared-proto/src/stream.rs`

### ConnectRequest

```rust
pub struct ConnectRequest {
    pub dest: String,
    pub connection_type: ConnectionType,
    pub metadata: Vec<Metadata>,
}
```

Accessors: `metadata_value()`, `http_method()`, `http_host()`,
`http_headers()`, `flow_id()`, `trace_id()`

**Status:** field-level match with Go schema.

### ConnectResponse

```rust
pub struct ConnectResponse {
    pub error: String,
    pub metadata: Vec<Metadata>,
}
```

Constructors: `success()`, `error()`, `http()`, `tcp_ack()`

**Status:** field-level match with Go schema.

### ConnectionType

```rust
pub enum ConnectionType {
    Http = 0,
    WebSocket = 1,
    Tcp = 2,
}
```

**Status:** exact enum-value match with Go.

### Metadata Key Constants

```rust
pub const HTTP_METHOD_KEY: &str = "HttpMethod";
pub const HTTP_HOST_KEY: &str = "HttpHost";
pub const HTTP_HEADER_KEY: &str = "HttpHeader";
pub const HTTP_STATUS_KEY: &str = "HttpStatus";
pub const FLOW_ID_KEY: &str = "FlowID";
pub const CF_TRACE_ID_KEY: &str = "cf-trace-id";
pub const CONTENT_LENGTH_KEY: &str = "HttpHeader:Content-Length";
```

**Status:** matches Go metadata key conventions.

### Rust Wire Framing (ConnectRequest Parse)

Source: `crates/cloudflared-cli/src/transport/quic/lifecycle.rs`

The Rust binary format for `ConnectRequest` on data streams:

```text
[2 bytes: connection_type as u16 big-endian]
[2 bytes: dest length as u16 big-endian]
[N bytes: dest UTF-8]
[2 bytes: metadata count as u16 big-endian]
for each metadata:
  [2 bytes: key length as u16 big-endian]
  [N bytes: key UTF-8]
  [2 bytes: val length as u16 big-endian]
  [N bytes: val UTF-8]
```

### ConnectResponse Wire Encoding

Not yet wired into the response path. A test helper
(`serialize_connect_request`) exists for roundtrip testing.

## Incoming Stream Round-Trip Path

Source: `baseline-2026.2.0/old-impl/connection/quic_connection.go` and
`baseline-2026.2.0/old-impl/proxy/proxy.go`

### Go Frozen Baseline Dispatch Flow

1. **accept stream**: `q.conn.AcceptStream(ctx)` returns a new QUIC data
   stream
2. **spawn goroutine**: `go q.runStream(quicStream)` — one goroutine per
   accepted stream
3. **wrap stream**: wrapped in `SafeStreamCloser` and `nopCloserReadWriter`,
   then served via `rpcquic.NewCloudflaredServer(q.handleDataStream, ...)`
4. **parse ConnectRequest**: `stream.ReadConnectRequestData()` reads Cap'n
   Proto binary from the stream and returns a `ConnectRequest` with dest,
   type, and metadata
5. **dispatch by type**: `q.dispatchRequest(ctx, stream, request)` calls
   `q.orchestrator.GetOriginProxy()` to get the current origin proxy (which
   holds ingress rules), then switches on `request.Type`:
   - `ConnectionTypeHTTP` / `ConnectionTypeWebsocket`:
     - `buildHTTPRequest()` constructs an `http.Request` from metadata
       (method, host, headers)
     - calls `originProxy.ProxyHTTP(&w, tracedReq, isWebsocket)`
   - `ConnectionTypeTCP`:
     - calls `originProxy.ProxyTCP(ctx, rwa, &TCPRequest{...})`
6. **ingress matching**: inside `proxy.ProxyHTTP()`:
   - `p.ingressRules.FindMatchingRule(req.Host, req.URL.Path)` returns the
     matching ingress rule
7. **origin dispatch**: based on rule service type:
   - `HTTPOriginProxy` → `p.proxyHTTPRequest()` — standard HTTP proxy
   - `StreamBasedOriginProxy` → `p.proxyStream()` — bidirectional stream
   - `HTTPLocalProxy` → `p.proxyLocalRequest()` — local service
8. **response return**: response flows back through `httpResponseAdapter`
   which wraps the QUIC `RequestServerStream`, calling
   `WriteConnectResponseData()` to send metadata (status code, headers) as
   Cap'n Proto binary, then streaming body bytes via `Write()`

### Current Rust Dispatch Flow

Source: `crates/cloudflared-cli/src/proxy/origin.rs` and
`crates/cloudflared-cli/src/transport/quic/lifecycle.rs`

1. stream accepted from QUIC connection
2. `parse_connect_request(data)` reads from custom big-endian binary format
3. `dispatch_to_origin(request, config)` matches ingress rules and dispatches:
   - `HttpStatus(code)` → returns status code response (**wired**)
   - `HelloWorld` → returns 200 with HTML body (**wired**)
   - `Http(url)` → dispatch path wired but actual HTTP origin connection
     returns 502 with `X-Cloudflared-Origin-Status: dispatch-wired`
     (**dispatch only, no origin round-trip**)
   - `TcpOverWebsocket`, `UnixSocket`, `UnixSocketTls`, `Bastion`,
     `SocksProxy`, `NamedToken` → explicit `Unimplemented` stubs

### Key Differences

- Go: full origin round-trip through ingress→proxy→origin→response for all
  connection types
- Rust: dispatch path exists but only `HttpStatus` and `HelloWorld` produce
  real responses; `Http(url)` is wired but returns 502 without actual origin
  connection
- Go: response metadata flows back via Cap'n Proto
  (`WriteConnectResponseData`)
- Rust: `ConnectResponse` type exists but is not wired into the response path

## Protocol Event Model

Source: `baseline-2026.2.0/old-impl/connection/event.go` and
`baseline-2026.2.0/old-impl/connection/protocol.go`

### Go Event Struct

```go
type Event struct {
    Index       uint8
    EventType   Status
    Location    string
    Protocol    Protocol
    URL         string
    EdgeAddress net.IP
}
```

### Go Status Enum

| Value | Name | Meaning |
| --- | --- | --- |
| 0 | `Disconnected` | connection lost |
| 1 | `Connected` | connection established |
| 2 | `Reconnecting` | attempting reconnection |
| 3 | `SetURL` | quick tunnel URL assigned |
| 4 | `RegisteringTunnel` | registration in progress |
| 5 | `Unregistering` | graceful shutdown initiated |

### Go Protocol Enum

| Value | Name | TLS SNI | ALPN |
| --- | --- | --- | --- |
| 0 | `HTTP2` | `h2.cftunnel.com` | (standard) |
| 1 | `QUIC` | `quic.cftunnel.com` | `argotunnel` |

Fallback order: QUIC → HTTP/2 (on QUIC failure)

### Current Rust Equivalents

**ProtocolBridgeState** (`crates/cloudflared-cli/src/protocol.rs`):

| Variant | Approximate Go equivalent |
| --- | --- |
| `BridgeUnavailable` | `Disconnected` |
| `BridgeCreated` | (no direct equivalent — between connect and register) |
| `RegistrationSent` | `RegisteringTunnel` |
| `RegistrationObserved` | `Connected` |
| `BridgeClosed` | `Disconnected` |

**ProtocolEvent** (`crates/cloudflared-cli/src/protocol.rs`):

| Variant | Fields | Go equivalent |
| --- | --- | --- |
| `Registered` | `peer: SocketAddr` | `Connected` event |
| `IncomingStream` | `stream_id: u64, request: ConnectRequest` | (no Go event equivalent — Go dispatches inline) |
| `RegistrationComplete` | `conn_uuid: Uuid, location: String` | `Connected` with details |

### Key Differences

- Go has 6 status values; Rust has 5 bridge states with different granularity
- Go `Reconnecting` has no Rust equivalent (no reconnection logic)
- Go `SetURL` has no Rust equivalent (no quick tunnel URL tracking)
- Go `Unregistering` has no Rust equivalent (no graceful shutdown)
- Rust `IncomingStream` event has no Go equivalent (Go dispatches inline
  without raising an event)
- Go events carry `Protocol`, `URL`, `EdgeAddress` fields; Rust events do not

## Gap Summary

| Gap | Severity | Detail |
| --- | --- | --- |
| wire encoding mismatch (ConnectRequest) | critical | Rust uses custom big-endian binary; Go uses Cap'n Proto binary |
| ConnectResponse not wired | high | response path not integrated |
| transport header serialization absent | high | base64 header encoding/decoding not implemented |
| origin HTTP round-trip absent | critical | `Http(url)` dispatch returns 502; actual origin connection not implemented |
| TCP/WebSocket/Unix dispatch absent | high | all non-HTTP connection types return `Unimplemented` |
| ResponseMeta not implemented | medium | pre-generated JSON response metadata missing |
| control header stripping absent | medium | `IsControlResponseHeader` equivalent missing |
| HTTP/2 stream type dispatch absent | medium | HTTP/2 transport not implemented |
| header round-trip tests absent | high | no frozen-baseline wire fixture tests |
| protocol event model incomplete | medium | missing `Reconnecting`, `SetURL`, `Unregistering` equivalents |
| QUIC ALPN `argotunnel` | parity-backed | Rust sets ALPN correctly in `session.rs` |

## Wire Encoding Assessment

The Rust `ConnectRequest` binary format and the Go `ConnectRequest` Cap'n
Proto binary format are **different wire encodings** of the same logical
schema. This is the most significant CDC gap at the stream level.

The Rust encoding may be deliberately simplified for the initial QUIC-only
transport, but wire compatibility with the edge requires matching the exact
Cap'n Proto binary framing that the edge expects.

**Evidence needed:** confirmation from edge behavior whether the edge accepts
the Rust binary format, or whether Cap'n Proto framing is strictly required.
Until proven, this is treated as a critical open gap.
