# CDC Feature Group: Stream Contracts

## Scope

This document covers the per-stream request/response contracts used over QUIC
data streams (and HTTP/2 muxed streams), including the `ConnectRequest` and
`ConnectResponse` schemas, wire framing, metadata conventions, and internal
transport headers.

## Frozen Baseline Schema

Source: [baseline-2026.2.0/tunnelrpc/proto/quic_metadata_protocol.capnp](../../../baseline-2026.2.0/tunnelrpc/proto/quic_metadata_protocol.capnp)

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

Source: [baseline-2026.2.0/tunnelrpc/pogs/quic_metadata_protocol.go](../../../baseline-2026.2.0/tunnelrpc/pogs/quic_metadata_protocol.go)

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

Source: [baseline-2026.2.0/connection/header.go](../../../baseline-2026.2.0/connection/header.go) and design audit

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

Source: [crates/cfdrs-cdc/src/stream.rs](../../../crates/cfdrs-cdc/src/stream.rs)

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

### Rust Wire Framing (ConnectRequest/ConnectResponse)

Source: [crates/cfdrs-cdc/src/stream_codec.rs](../../../crates/cfdrs-cdc/src/stream_codec.rs)

The Rust wire codec now uses Cap'n Proto binary encoding matching the Go
baseline (`ToPogs()`/`FromPogs()` in `quic_metadata_protocol.go`).

- `ConnectRequest::marshal_capnp()` / `ConnectRequest::unmarshal_capnp()` —
  builder/reader-level codec matching `quic_metadata_protocol.capnp` schema
- `encode_connect_request()` / `decode_connect_request()` — wire-level
  serialization producing Cap'n Proto binary byte buffer
- `ConnectResponse::marshal_capnp()` / `ConnectResponse::unmarshal_capnp()` —
  builder/reader-level codec for response path
- `encode_connect_response()` / `decode_connect_response()` — wire-level
  serialization for response path
- `Metadata::marshal_capnp()` / `Metadata::unmarshal_capnp()` — metadata
  entry codec shared by both request and response

10 tests: 7 wire roundtrip tests + 3 builder→reader marshal/unmarshal tests.
Runtime integration is live for both directions: `lifecycle.rs` now parses
incoming `ConnectRequest` values with `decode_connect_request()`, and the
proxy/runtime path encodes outbound `ConnectResponse` values with
`encode_connect_response()`.

## Incoming Stream Round-Trip Path

Source: [baseline-2026.2.0/connection/quic_connection.go](../../../baseline-2026.2.0/connection/quic_connection.go) and
[baseline-2026.2.0/proxy/proxy.go](../../../baseline-2026.2.0/proxy/proxy.go)

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

Source: [crates/cfdrs-bin/src/proxy/origin.rs](../../../crates/cfdrs-bin/src/proxy/origin.rs) and
[crates/cfdrs-bin/src/transport/quic/lifecycle.rs](../../../crates/cfdrs-bin/src/transport/quic/lifecycle.rs)

1. stream accepted from QUIC connection
2. `parse_connect_request(data)` reads from CDC-owned `stream_codec.rs`
   (Cap'n Proto binary codec via `decode_connect_request`)
3. `dispatch_to_origin(request, config)` matches ingress rules and dispatches:
   - `HttpStatus(code)` → returns status code response (**wired**)
   - `HelloWorld` → returns the Go-shaped 200/connect-response metadata path;
     the standalone quick-tunnel hello server remains HIS-owned and deferred
   - `Http(url)` → performs a real `reqwest` round-trip to the configured
     origin and forwards origin status plus response headers into the
     `ConnectResponse`
   - `TcpOverWebsocket`, `UnixSocket`, `UnixSocketTls`, `Bastion`,
     `SocksProxy`, `NamedToken` → explicit `Unimplemented` stubs
4. `proxy::origin::to_connect_response()` converts the dispatch result into
   CDC-owned response metadata
5. `proxy/mod.rs` queues the encoded response and `lifecycle.rs` writes it
   back to the QUIC stream with `fin=true`

### Key Differences

- Go: after `ConnectResponse`, the stream stays open as a bidirectional pipe
  and HTTP/TCP payload bytes continue flowing
- Rust: request parsing and `ConnectResponse` encoding are wired, including
  real HTTP origin status/header round-trips, but the stream currently closes
  immediately after the `ConnectResponse` (`fin=true`), so post-response body
  piping and long-lived TCP/WebSocket forwarding remain deferred
- Go: full origin round-trip exists across the supported connection types
- Rust: `HttpStatus`, `HelloWorld`, and HTTP origin status/header dispatch are
  wired; TCP/WebSocket/Unix/bastion/named-token flows still report honest
  unimplemented boundaries

## Protocol Event Model

Source: [baseline-2026.2.0/connection/event.go](../../../baseline-2026.2.0/connection/event.go) and
[baseline-2026.2.0/connection/protocol.go](../../../baseline-2026.2.0/connection/protocol.go)

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

**CDC-owned types** ([crates/cfdrs-cdc/src/protocol.rs](../../../crates/cfdrs-cdc/src/protocol.rs)):

- `Protocol` enum: `Http2`, `Quic` — matches Go iota order
- `Protocol::tls_settings()` → `TlsSettings { server_name, next_protos }` — matches Go `TLSSettings`
- `Protocol::fallback()` → `Option<Protocol>` — QUIC→Http2, Http2→None
- `ProtocolSelector` trait and `StaticProtocolSelector` — matches Go `ProtocolSelector` interface
- `PROTOCOL_LIST` constant: `[Quic, Http2]` — matches Go `ProtocolList`
- `ConnectionStatus` enum: 6 variants matching Go `Status` iota (Disconnected through Unregistering)
- `ConnectionEvent` struct: `index`, `event_type`, `location`, `protocol`, `url`, `edge_address`

**Runtime-internal types** ([crates/cfdrs-bin/src/protocol.rs](../../../crates/cfdrs-bin/src/protocol.rs)):

`ProtocolBridgeState` is a separate runtime-internal concern tracking bridge lifecycle (BridgeUnavailable, BridgeCreated, RegistrationSent, RegistrationObserved, BridgeClosed). It does not replace CDC-owned `ConnectionStatus`.

**ProtocolEvent** ([crates/cfdrs-bin/src/protocol.rs](../../../crates/cfdrs-bin/src/protocol.rs)):

| Variant | Fields | Go equivalent |
| --- | --- | --- |
| `Registered` | `peer: SocketAddr` | `Connected` event |
| `IncomingStream` | `stream_id: u64, request: ConnectRequest` | (no Go event equivalent — Go dispatches inline) |
| `RegistrationComplete` | `conn_uuid: Uuid, location: String` | `Connected` with details |

### Key Differences

- Go events carry `Protocol`, `URL`, `EdgeAddress` fields; runtime `ProtocolEvent` does not yet consume CDC-owned `ConnectionEvent`
- Runtime does not yet emit CDC-owned `ConnectionEvent` for observer consumption
- `Reconnecting`, `SetURL` status transitions exist in CDC types but are not yet wired in runtime
- Rust `IncomingStream` event has no Go equivalent (Go dispatches inline without raising an event)

## Gap Summary

| Gap | Severity | Detail |
| --- | --- | --- |
| ~~wire encoding mismatch (ConnectRequest)~~ | ~~critical~~ resolved | Cap'n Proto binary codec implemented in CDC-owned `stream_codec.rs` and live request parsing uses it |
| ~~ConnectResponse not wired~~ | ~~high~~ resolved | proxy/runtime now encode outbound `ConnectResponse` values with the CDC-owned Cap'n Proto codec |
| post-ConnectResponse stream piping absent | critical | `lifecycle.rs` writes the encoded response with `fin=true`, so HTTP bodies and long-lived TCP/WebSocket streams do not stay open after the response |
| ~~transport header serialization absent~~ | ~~high~~ resolved | `serialize_headers()` / `deserialize_headers()` implemented in `stream_contract.rs` with base64 STANDARD_NO_PAD; 7 tests |
| post-response HTTP body streaming absent | critical | reachable HTTP origins contribute status and headers to `ConnectResponse`, but response body bytes are not piped after that response |
| TCP/WebSocket/Unix dispatch absent | high | all non-HTTP connection types return `Unimplemented` |
| ~~ResponseMeta not implemented~~ | ~~medium~~ resolved | `RESPONSE_META_ORIGIN`, `RESPONSE_META_CLOUDFLARED`, `RESPONSE_META_CLOUDFLARED_FLOW_LIMITED` constants match baseline; JSON-validated by tests |
| ~~control header stripping absent~~ | ~~medium~~ resolved | `is_control_response_header()` + `is_websocket_client_header()` implemented in `stream_contract.rs`; 3 tests |
| HTTP/2 stream type dispatch absent | medium | HTTP/2 transport not implemented |
| header round-trip tests absent | high | no frozen-baseline wire fixture tests |
| ~~protocol event model incomplete~~ | ~~medium~~ resolved | CDC-owned `Protocol` enum, `ConnectionStatus` (6 variants), `ConnectionEvent` struct, `ProtocolSelector` trait, and `StaticProtocolSelector` implemented; runtime wiring pending |
| QUIC ALPN `argotunnel` | parity-backed | Rust sets ALPN correctly in [crates/cfdrs-bin/src/transport/quic/session.rs](../../../crates/cfdrs-bin/src/transport/quic/session.rs) |

## Wire Encoding Assessment

The Rust `ConnectRequest` and `ConnectResponse` wire codecs now use Cap'n Proto
binary encoding via CDC-owned `stream_codec.rs`, matching the Go baseline's
`ToPogs()`/`FromPogs()` codec in `quic_metadata_protocol.go`.

The custom big-endian binary format that was previously used has been
replaced, and the live QUIC stream path now uses the CDC codec for both
request parsing and response emission. 10 round-trip tests verify
schema-level marshaling and wire-level serialization. The codec uses the
generated Cap'n Proto bindings from the frozen baseline
`quic_metadata_protocol.capnp` schema.

**Remaining gap:** the runtime still closes the QUIC stream immediately after
the encoded `ConnectResponse`, so the post-response bidirectional pipe
contract is not yet implemented.
