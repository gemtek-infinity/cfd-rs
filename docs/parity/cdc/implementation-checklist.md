# CDC Implementation Checklist

## Purpose

This document is the live parity ledger for interactions between cloudflared
and Cloudflare-managed services, APIs, and contracts.

This includes:

- registration RPC and related registration content
- control-stream lifecycle behavior
- per-stream request and response contracts
- management and log-streaming contracts
- metrics and readiness contracts where externally relevant
- Cloudflare API interactions used by command surfaces

This document does not claim parity from Rust code shape alone.

It records:

- the frozen contract that must be matched
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
in [baseline-2026.2.0/](../../../baseline-2026.2.0/) and comparison against the current Rust CDC
surface in [crates/cfdrs-cdc/](../../../crates/cfdrs-cdc/) and [crates/cfdrs-bin/](../../../crates/cfdrs-bin/).

The frozen Go CDC surface uses Cap'n Proto for registration and stream
framing. The Rust CDC crate has Cap'n Proto codec coverage for all six
registration types (`registration_codec.rs`) and both stream types
(`stream_codec.rs`). The runtime in `lifecycle.rs` now uses Cap'n Proto as
the live wire format for registration and stream framing. Origin HTTP
dispatch via `reqwest` performs real round-trips. All six lifecycle events
(`Registered`, `RegistrationComplete`, `IncomingStream`, `Unregistering`,
`Disconnected`, `ConfigPushed`) have send sites wired in `lifecycle.rs`.

### Registration RPC

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-001 | registration RPC schema | `tunnelrpc/proto/tunnelrpc.capnp` | `RegistrationServer.registerConnection(auth: TunnelAuth, tunnelId: Data, connIndex: UInt8, options: ConnectionOptions) -> ConnectionResponse`. Schema IDs: RegistrationServer `@0xf71695ec7fe85497`, TunnelAuth `@0x9496331ab9cd463f`, ConnectionOptions `@0xb4bf9861fe035d04`, ConnectionResponse `@0xdbaa9d03d52b62dc` | cfdrs-cdc `registration.rs`, `registration_codec.rs` | audited, parity-backed | local tests | closed | schema field compare, method signature tests, Cap'n Proto codec tests | critical | Full baseline-matching schema types with Cap'n Proto codec: `TunnelAuth`, `ClientInfo`, `ConnectionOptions`, `ConnectionDetails`, `ConnectionError`, `ConnectionResponse` (union). `marshal_capnp`/`unmarshal_capnp` implemented for all six types with 17 round-trip tests including wire serialization. Codec matches Go `pogs` field mapping. Runtime wired: `lifecycle.rs` calls `encode_registration_request` and `decode_registration_response` for Cap'n Proto binary over the QUIC control stream. See [docs/parity/cdc/registration-rpc.md](registration-rpc.md) |
| CDC-002 | registration wire encoding | `tunnelrpc/registration_client.go`, capnp-go marshal | registration request/response encoded as Cap'n Proto binary over QUIC control stream (stream ID 0) | cfdrs-cdc `registration_codec.rs` | audited, parity-backed | local tests | closed | frozen-fixture wire tests, Cap'n Proto binary roundtrip tests | critical | Cap'n Proto binary codec implemented in `registration_codec.rs` with `marshal_capnp`/`unmarshal_capnp` for all registration types. Wire serialization round-trip test passes. Runtime wired: `lifecycle.rs` uses `encode_registration_request`/`decode_registration_response` as the live wire format — JSON registration path replaced. |
| CDC-003 | registration response contract | `ConnectionResponse` union: `error(ConnectionError)` or `connectionDetails(ConnectionDetails)`. `ConnectionError` has `cause`, `retryAfter` (Int64 ns), `shouldRetry` (Bool). `ConnectionDetails` has `uuid`, `locationName`, `tunnelIsRemotelyManaged` | success returns `ConnectionDetails`; error returns structured `ConnectionError` with retry semantics | cfdrs-cdc `registration.rs`, `registration_codec.rs` | audited, parity-backed | local tests | closed | response golden tests, error retry-semantics tests, ConnectionError field tests | high | `ConnectionResponse` is union enum (Success/Error). `ConnectionError` has `retry_after_ns` (i64) and `should_retry` (bool). Cap'n Proto codec with `marshal_capnp`/`unmarshal_capnp` and round-trip tests. Runtime wired: `await_registration_response` in `lifecycle.rs` decodes `ConnectionResponse` and handles both Success and Error variants with retry logic. |
| CDC-004 | ClientInfo nesting and fields | `ClientInfo` struct: `clientId` (Data, 16-byte UUID), `features` (List(Text)), `version` (Text), `arch` (Text). Nested inside `ConnectionOptions.client` | registration sends client identity with UUID and capability list | cfdrs-cdc `registration.rs`, `registration_codec.rs` | audited, parity-backed | local tests | closed | clientId UUID tests, features list tests, nesting shape tests | high | `ClientInfo` has `client_id` (UUID), `features` (Vec), `version`, `arch`; nested in `ConnectionOptions.client`. `for_current_platform()` constructor. Cap'n Proto codec with `marshal_capnp`/`unmarshal_capnp` and round-trip tests. Runtime wired: `build_registration_request` in `lifecycle.rs` uses `ConnectionOptions::for_current_platform()`. |
| CDC-005 | ConnectionOptions full field set | `ConnectionOptions`: `client` (ClientInfo), `originLocalIp` (Data), `replaceExisting` (Bool), `compressionQuality` (UInt8), `numPreviousAttempts` (UInt8) | all fields sent to edge during registration | cfdrs-cdc `registration.rs`, `registration_codec.rs` | audited, parity-backed | local tests | closed | field-level tests, default-value tests | high | Full field set: `client` (ClientInfo), `origin_local_ip`, `replace_existing`, `compression_quality`, `num_previous_attempts`. `for_current_platform()` constructor. Cap'n Proto codec with `marshal_capnp`/`unmarshal_capnp` and round-trip tests including IPv4/IPv6. Runtime wired: `build_registration_request` in `lifecycle.rs` uses `ConnectionOptions::for_current_platform()` and sets `origin_local_ip`. |
| CDC-006 | feature flags sent during registration | `ConnectionOptions.Client.Features`: default set `allow_remote_config`, `serialized_headers`, `support_datagram_v2`, `support_quic_eof`, `management_logs`; selector-added: `support_datagram_v3_2`, `postquantum`; CLI-passthrough only: `quick_reconnects`; deprecated (filtered before send): `support_datagram_v3`, `support_datagram_v3_1` | capability list negotiates edge behavior at registration time | cfdrs-cdc `features.rs` | audited, parity-backed | local tests | closed | feature list tests, deprecated-feature filtering tests, selector logic tests | high | All feature flag constants match baseline. `default_feature_list()` returns 5 always-on features. `dedup_and_filter()` removes deprecated features and deduplicates. `build_feature_list(datagram_v3, post_quantum)` selector function matches Go `FeatureSnapshot.FeaturesList` construction: starts from defaults, conditionally adds `support_datagram_v3_2` and `postquantum`, then dedup-filters. 8 tests. Runtime wired: `build_registration_request` in `lifecycle.rs` calls `build_feature_list(true, false)`. |
| CDC-007 | unregisterConnection RPC | `RegistrationServer.unregisterConnection()` | graceful shutdown over control stream with configurable grace period | cfdrs-cdc `registration.rs`, `rpc_dispatch.rs` | audited, partial | local tests | open gap | graceful shutdown tests, grace period tests | medium | `UnregisterConnectionRequest` type, `RegistrationClient::unregister_connection()` capnp-rpc client wrapper with local-dispatch test. RPC dispatch layer complete. Runtime call-site requires capnp-rpc transport on the control stream (current control stream uses raw `capnp::serialize` without an `RpcSystem`). Closure path: refactor control stream to use `capnp_rpc::RpcSystem` with quiche-backed two-party transport, then call `RegistrationClient::unregister_connection()` during graceful shutdown. |
| CDC-008 | updateLocalConfiguration RPC | `RegistrationServer.updateLocalConfiguration(config: Data)` | pushes tunnel config to edge on connIndex==0 when not remotely managed | cfdrs-cdc `registration.rs`, `registration_codec.rs`, `rpc_dispatch.rs` | audited, parity-backed | local tests | closed | config push tests, connIndex==0 guard tests | medium | `UpdateLocalConfigurationRequest` type with `from_config_bytes()` and `to_capnp_bytes()` codec helpers, `RegistrationClient::update_local_configuration()` capnp-rpc client wrapper with local-dispatch test, 3 codec tests. Runtime wired: `send_local_configuration` in `lifecycle.rs` creates the request, calls `to_capnp_bytes()`, writes to control stream with `fin=false`, and emits `ConfigPushed`. connIndex==0 and `!is_remotely_managed` guards enforced at call-site in `await_registration_response` (L315). Current wire path uses raw `capnp::serialize`; capnp-rpc transport path available in `RegistrationClient` for future upgrade. |
| CDC-009 | SessionManager interface | `SessionManager.registerUdpSession()` and `unregisterUdpSession()` | UDP session lifecycle over Cap'n Proto RPC | cfdrs-cdc `registration.rs`, `registration_codec.rs`, `rpc_dispatch.rs` | audited, parity-backed | local tests | closed | session registration tests, session cleanup tests | high | `SessionManagerHandler` trait with `register_udp_session`/`unregister_udp_session` methods. `CloudflaredServerDispatch` implements the generated `session_manager::Server` with capnp→domain param translation, handler invocation, and `marshal_capnp` response encoding. 5 codec tests + 2 capnp-rpc local-dispatch tests (register, unregister). Full RPC interface contract closed. |
| CDC-010 | ConfigurationManager interface | `ConfigurationManager.updateConfiguration(version: Int32, config: Data) -> UpdateConfigurationResponse` | remote edge pushes config updates to cloudflared | cfdrs-cdc `registration.rs`, `registration_codec.rs`, `rpc_dispatch.rs` | audited, parity-backed | local tests | closed | config update tests, version tracking tests | medium | `ConfigurationManagerHandler` trait. `CloudflaredServerDispatch` implements the generated `configuration_manager::Server` with capnp→domain param translation, handler invocation, and `marshal_capnp` response encoding. 3 codec tests + 1 capnp-rpc local-dispatch test. Full RPC interface contract closed. |

### Stream Contracts

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-011 | ConnectRequest schema | `quic_metadata_protocol.capnp`: `ConnectRequest` with `dest` (Text), `type` (ConnectionType), `metadata` (List(Metadata)). `ConnectionType` enum: http=0, websocket=1, tcp=2 | per-stream request shape over QUIC data streams | cfdrs-cdc `stream.rs`, `stream_codec.rs` | audited, parity-backed | local tests | closed | schema compare, enum value tests, metadata key convention tests | critical | Rust logical types match Go schema fields. Cap'n Proto codec with `marshal_capnp`/`unmarshal_capnp` in `stream_codec.rs` (10 tests). Runtime wired: `parse_connect_request` in `lifecycle.rs` delegates to `cfdrs_cdc::stream_codec::decode_connect_request`. |
| CDC-012 | ConnectRequest wire framing | `tunnelrpc/pogs/quic_metadata_protocol.go` uses Cap'n Proto binary marshaling via `ToPogs()` | stream requests encoded as Cap'n Proto binary | cfdrs-cdc `stream_codec.rs` | audited, parity-backed | local tests | closed | frozen-fixture wire tests, binary format comparison, malformed-input tests | critical | Cap'n Proto binary codec in `stream_codec.rs` with `marshal_capnp`/`unmarshal_capnp` and wire-level `encode_connect_request`/`decode_connect_request`. 10 tests (7 wire roundtrip + 3 marshal/unmarshal). Custom big-endian format replaced. Runtime wired: `lifecycle.rs:parse_connect_request` delegates to CDC-owned `decode_connect_request`. |
| CDC-013 | ConnectResponse schema and framing | `quic_metadata_protocol.capnp`: `ConnectResponse` with `error` (Text), `metadata` (List(Metadata)). Cap'n Proto binary encoding. | per-stream response shape back to edge | cfdrs-cdc `stream_codec.rs` | audited, parity-backed | local tests | closed | schema compare, response construction tests, wire encoding tests | high | Cap'n Proto binary codec for `ConnectResponse` in `stream_codec.rs` with `marshal_capnp`/`unmarshal_capnp` and wire-level `encode_connect_response`/`decode_connect_response`. Custom format replaced. Runtime wired: `proxy/mod.rs` calls `encode_connect_response` to encode responses before writing to the QUIC stream. |
| CDC-014 | metadata key conventions | `connection/quic_connection.go`, `connection/header.go`, and `tracing/tracing.go` | keys: `HttpMethod`, `HttpHost`, `HttpHeader:<name>`, `HttpStatus`, `FlowID` (defined in `quic_connection.go`), `cf-trace-id` (`TracerContextName` in `tracing/tracing.go`), `HttpHeader:Content-Length` | cfdrs-cdc `stream_contract.rs` | audited, parity-backed | local tests | closed | metadata key inventory tests, accessor tests | medium | All metadata key constants defined in `stream_contract.rs` matching baseline. `header_metadata_key()` and `header_metadata_prefix()` helpers implemented. `CF_TRACE_ID_KEY` matches `TracerContextName`. Constants used by `ConnectRequest` accessors and proxy dispatch path. |
| CDC-015 | transport header serialization | `connection/header.go` | base64.RawStdEncoding pairs joined by `;` for `cf-cloudflared-request-headers` and `cf-cloudflared-response-headers`; JSON for `cf-cloudflared-response-meta` | cfdrs-cdc `stream_contract.rs` | audited, parity-backed | local tests | closed | header serialization roundtrip tests, base64 encoding tests | high | Header key constants defined. `serialize_headers()` and `deserialize_headers()` implemented with base64 STANDARD_NO_PAD (matching Go's `RawStdEncoding`). `HttpHeader` type matches Go's `HTTPHeader`. 7 tests including roundtrip, empty, no-padding, malformed rejection, and special character handling. Partially runtime wired: `proxy/origin.rs` uses `serialize_headers`, `is_control_response_header`, `HttpHeader`, and response meta constants in the response path. |
| CDC-016 | ResponseMeta contract | `connection/header.go` | pre-generated JSON: `{"src":"origin"}`, `{"src":"cloudflared"}`, `{"src":"cloudflared","flow_rate_limited":true}` | cfdrs-cdc `stream_contract.rs` | audited, parity-backed | local tests | closed | response meta shape tests | medium | `RESPONSE_META_ORIGIN`, `RESPONSE_META_CLOUDFLARED`, `RESPONSE_META_CLOUDFLARED_FLOW_LIMITED` constants match frozen baseline JSON. JSON validity and field values verified by tests. Runtime wired: `proxy/origin.rs:to_connect_response` uses `RESPONSE_META_CLOUDFLARED` and `RESPONSE_META_ORIGIN` for origin response meta. |
| CDC-017 | control header stripping | `connection/header.go` `IsControlResponseHeader` | headers with prefixes `:`, `cf-int-`, `cf-cloudflared-`, `cf-proxy-` stripped from user-visible responses | cfdrs-cdc `stream_contract.rs` | audited, parity-backed | local tests | closed | control header detection tests, stripping tests | medium | `CONTROL_HEADER_PREFIXES` and `is_control_response_header()` match baseline prefixes. `is_websocket_client_header()` implemented matching Go's `IsWebsocketClientHeader`. 3 tests (control headers, websocket headers, header key format). Runtime wired: `proxy/origin.rs:to_connect_response` calls `is_control_response_header` to filter headers in the HTTP origin response path. |
| CDC-018 | incoming stream round-trip | stream-serving runtime path (proxy/origin). Go path: AcceptStream → runStream → ReadConnectRequestData → dispatchRequest → GetOriginProxy → type switch (HTTP/WS/TCP) → ingressRules.FindMatchingRule → origin service → response via httpResponseAdapter | request accepted, matched to ingress, proxied to origin, response returned through tunnel | current proxy `origin.rs` | audited, parity-backed | local tests | closed | end-to-end stream tests, origin round-trip tests, error handling tests | critical | ConnectionType-aware dispatch: `dispatch_http_path` (HTTP/WebSocket share path matching Go `ProxyHTTP`), `dispatch_tcp_path` (TCP separate matching Go `ProxyTCP`). `to_connect_response()` bridges `OriginResponse → ConnectResponse`. `service_label()` covers all 9 IngressService variants. 23 origin tests including reachable-origin round-trip with status and header verification, server error propagation (500 as-is, not 502), method and Host forwarding, and full end-to-end pipeline with CDC-015/016 metadata checks. Runtime wired: `dispatch_http_origin` performs real HTTP round-trips via `reqwest::Client`; unreachable origins return 502. Full end-to-end QUIC-to-origin-to-response path integrated in `lifecycle.rs` and `proxy/mod.rs`. |

### Control Stream And Lifecycle

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-019 | control stream lifecycle | `connection/control.go` `ControlStreamHandler` | open control stream → register → optionally send local config → monitor for graceful shutdown → unregister | current QUIC transport `lifecycle.rs` and `protocol.rs` | audited, parity-backed | local tests | closed | lifecycle integration tests, stage transition tests | high | All 6 baseline lifecycle events emitted from correct runtime sites: `Registered` in `cross_protocol_boundary`, `RegistrationComplete` in `await_registration_response`, `IncomingStream` in `try_accept_stream`, `Unregistering` on graceful shutdown in `establish_quic_session`, `Disconnected` in `teardown_session`, `ConfigPushed` in `send_local_configuration`. 12 protocol tests: bridge delivery, all-variant delivery, baseline lifecycle ordering, ConfigPushed conn_index targeting, connection index destructuring, edge detail fields, state-to-Go-stage mapping. Wire-level `unregisterConnection` RPC call is CDC-007 scope, not lifecycle event scope. |
| CDC-020 | connection status events | `connection/event.go` | `Event` with Index, EventType (Disconnected=0, Connected=1, Reconnecting=2, SetURL=3, RegisteringTunnel=4, Unregistering=5), Location, Protocol, URL, EdgeAddress | cfdrs-cdc `protocol.rs` | audited, parity-backed | local tests | closed | event type inventory tests, transition tests | medium | CDC-owned `ConnectionStatus` enum (6 variants matching Go iota order) and `ConnectionEvent` struct (index, event_type, location, protocol, url, edge_address) implemented in cfdrs-cdc `protocol.rs`. Exported from crate root. Runtime `ProtocolBridgeState` in cfdrs-bin is a separate internal concern. 2 tests. |
| CDC-021 | protocol negotiation | `connection/protocol.go` | Protocol enum: HTTP2=0, QUIC=1. TLS server names: `h2.cftunnel.com` (HTTP/2), `quic.cftunnel.com` (QUIC). QUIC ALPN: `argotunnel`. Fallback: QUIC→HTTP/2. | cfdrs-cdc `protocol.rs` and current QUIC transport `edge.rs` | audited, parity-backed | local tests | closed | protocol selection tests, SNI tests, ALPN tests, fallback tests | high | CDC-owned `Protocol` enum (Http2, Quic) with `tls_settings()`, `fallback()`, and `Display`. `TlsSettings` struct. `ProtocolSelector` trait and `StaticProtocolSelector`. `PROTOCOL_LIST` constant. Runtime `edge.rs` now uses `EDGE_QUIC_TLS_SERVER_NAME` from cfdrs-cdc instead of hardcoded literal. 16 tests (7 original + 9 new). |
| CDC-022 | edge discovery | `edgediscovery/` | SRV record `_v2-origintunneld._tcp.argotunnel.com`, DNS-over-TLS fallback (dial `1.1.1.1:853`, TLS serverName `cloudflare-dns.com`), priority+weight sorting via Go stdlib, region1+region2 redundancy | cfdrs-cdc `protocol.rs` and `edge.rs`; cfdrs-bin `edge.rs` | audited, parity-backed | local tests | closed | SRV record round-trip tests (network-dependent), DoT fallback integration tests, region failover tests | high | CDC-owned `protocol.rs` defines decomposed SRV constants (`SRV_SERVICE`, `SRV_PROTO`, `SRV_NAME`), timeout constants (`DOT_TIMEOUT_SECS`, `REGION_FAILOVER_TIMEOUT_SECS`), `PROTOCOL_PERCENTAGE_RECORD`, `EdgeIPVersion` enum (V4=4, V6=6), `ConfigIPVersion` enum (Auto=2, IPv4Only=4, IPv6Only=6), `EdgeAddr` struct, `regional_service_name()` and `regional_srv_domain()`. CDC-owned `edge.rs` defines `AddrSet` (address pool), `Region` (primary/secondary failover with 600s IPv6→IPv4 timeout), `Regions` (two-region balanced manager with `UsedBy` tracking). 30 protocol tests + 16 edge tests. Runtime `cfdrs-bin/transport/quic/edge.rs` performs real SRV discovery via `hickory-resolver`: system resolver lookup with DoT fallback (1.1.1.1:853, serverName=cloudflare-dns.com, 15s timeout), per-CNAME A/AAAA resolution, and `Regions::from_resolved()` → `get_unused_addr()` address selection. 9 edge_region/edge_host_label unit tests. SRV priority+weight sorting delegated to hickory-resolver, matching Go's delegation to `net.LookupSRV` stdlib. |

### Management And Log Streaming

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-023 | management service routes | `management/service.go` | chi router routes: `/ping` (GET/HEAD), `/logs` (GET→WS), `/host_details` (GET), `/metrics` (GET, conditional), `/debug/pprof/{heap or goroutine}` (GET, conditional). All require token query middleware. | cfdrs-cdc `management.rs` | audited, partial | local tests | open gap | endpoint contract tests, route inventory tests, conditional route tests | critical | Route path constants defined in `management.rs` matching Go. Management HTTP service runtime still absent. See [docs/parity/cdc/management-and-diagnostics.md](management-and-diagnostics.md) |
| CDC-024 | management auth middleware | `management/middleware.go` | `?access_token=<JWT>` query param required; parsed via `ParseToken`; error: `{"errors":[{"code":1001,"message":"missing access_token query parameter"}]}` with 400 status (Go `omitempty` on bool `Success` field suppresses `false` from JSON output) | cfdrs-cdc `management.rs` | audited, partial | local tests | open gap | auth middleware tests, error response tests, JWT validation tests | critical | `ManagementError`, `ManagementErrorResponse`, error code 1001, `missing_access_token()` constructor, `ACCESS_TOKEN_QUERY_PARAM` constant. JSON shape matches Go omitempty. 6 parity tests including exact Go JSON byte comparison and omit-on-zero-code/empty-message. JWT parsing and middleware dispatch still absent. |
| CDC-025 | host details contract | `management/service.go` `getHostDetailsResponse` | JSON: `{"connector_id":"uuid","ip":"10.0.0.4","hostname":"custom:label"}` | cfdrs-cdc `management.rs` | audited, partial | local tests | open gap | response shape tests, field derivation tests | high | `HostDetailsResponse` struct with `connector_id`, `ip` (omitempty), `hostname` (omitempty) matching Go JSON field names. 4 parity tests: key names, omitempty behavior, deserialization, UUID string format. Endpoint handler still absent. |
| CDC-026 | log streaming WebSocket | `management/events.go` and `session.go` | WebSocket upgrade on `/logs`; client sends `start_streaming` / `stop_streaming`; server sends `logs` with `[{time, level, message, event, fields}]`; filters: events (cloudflared/http/tcp/udp), level (debug/info/warn/error), sampling (0-1); close codes: 4001/4002/4003 | cfdrs-cdc `log_streaming.rs` | audited, partial | local tests | open gap | WebSocket event tests, filter tests, sampling tests, close code tests, session limit tests | critical | `LogEventType`, `LogLevel`, `LogEntry`, `StreamingFilters`, `EventStartStreaming`, `EventStopStreaming`, `EventLog` types with serde matching Go JSON shape. `LOG_WINDOW=30`. 16 parity tests. WebSocket transport, session management, and close codes still absent. |
| CDC-027 | management CORS | `management/service.go` corsHandler | allowed origins: `https://*.cloudflare.com`; credentials: true; maxAge: 300 | cfdrs-cdc `management.rs` | audited, partial | local tests | open gap | CORS header tests | medium | `CORS_ALLOWED_ORIGIN`, `CORS_MAX_AGE_SECS`, `CORS_ALLOW_CREDENTIALS` constants match Go. 1 parity test. CORS middleware runtime still absent. |
| CDC-028 | diagnostics conditional exposure | `management/service.go` | `/metrics` and `/debug/pprof` only registered when `enableDiagServices=true` | cfdrs-cdc `management.rs` | audited, partial | local tests | open gap | conditional route tests, gating tests | medium | `DIAG_ROUTES` constant identifies the conditionally-gated routes. 1 parity test. Runtime conditional registration still absent. |

### Metrics And Readiness

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-029 | readiness endpoint contract | `metrics/readiness.go` | `GET /ready` returns JSON `{"status":200,"readyConnections":N,"connectorId":"uuid"}` with HTTP 200 if active conns > 0, else 503 | `cfdrs-his` + `cfdrs-bin` runtime | audited, partial | local tests | open gap | HTTP contract tests, ready/not-ready semantics tests | high | runtime serves `/ready` with JSON response matching Go fields (`status`, `readyConnections`, `connectorId`); 21 HIS tests + 6 runtime endpoint tests cover contract; HTTP 503-when-not-ready semantics implemented; CDC column updated to reflect existing HIS+runtime implementation |
| CDC-030 | healthcheck endpoint | `metrics/metrics.go` | `GET /healthcheck` returns text `OK\n` with HTTP 200 | `cfdrs-his` + `cfdrs-bin` runtime | audited, parity-backed | local tests | closed | liveness tests | medium | runtime serves `/healthcheck` returning `OK\n` with `text/plain; charset=utf-8`, status 200; `HEALTHCHECK_RESPONSE` matches Go exactly; endpoint wired in runtime metrics server |
| CDC-031 | Prometheus metrics endpoint | `metrics/metrics.go` | `GET /metrics` served by `promhttp.Handler()` | `cfdrs-his` + `cfdrs-bin` runtime | audited, partial | local tests | open gap | endpoint tests, metric-name tests | medium | runtime serves `/metrics` with `build_info` and `cfdrs_ready_connections` gauge; runtime endpoint test verifies metric output; full Prometheus metric-name parity with Go baseline not yet exhaustively verified |
| CDC-032 | quicktunnel endpoint | `metrics/metrics.go` | `GET /quicktunnel` returns `{"hostname":"<hostname>"}` | cfdrs-his `metrics_server.rs` | audited, partial | local tests | open gap | quicktunnel response tests | low | `QuickTunnelResponse` type in cfdrs-his matches JSON shape. 1 parity test (HIS-028). CDC contract verified; runtime wiring through management service still absent. |

### Cloudflare REST API

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-033 | tunnel CRUD API | `cfapi/tunnel.go` and `client.go` | `CreateTunnel`, `GetTunnel`, `GetTunnelToken`, `DeleteTunnel`, `ListTunnels`, `ListActiveClients`, `CleanupConnections` to `/accounts/{accountTag}/cfd_tunnel/...` | cfdrs-cdc `api_resources.rs` | audited, partial | local tests | open gap | API request shape tests, response envelope tests, error mapping tests | critical | `Tunnel`, `TunnelWithToken`, `TunnelConnection`, `ActiveClient`, `NewTunnel` resource types with serde matching Go JSON shape. 6 parity tests including Go JSON deserialization with flattened `conns` rename. HTTP client and CRUD methods still absent. See [docs/parity/cdc/metrics-readiness-and-api.md](metrics-readiness-and-api.md) |
| CDC-034 | API response envelope | `cfapi/base_client.go` | JSON envelope: `{"success":true,"errors":[],"messages":[],"result":...,"result_info":{...}}`. Error mapping: 400→ErrBadRequest, 401/403→ErrUnauthorized, 404→ErrNotFound | cfdrs-cdc `api.rs` | audited, partial | local tests | open gap | envelope parsing tests, error mapping tests | critical | `ApiResponse`, `ApiError`, `Pagination` structs with serde matching Go JSON shape. `ApiClientError` enum (Unauthorized, BadRequest, NotFound, NoSuccess). 8 parity tests. HTTP client and error mapping runtime still absent. |
| CDC-035 | API auth and headers | `cfapi/base_client.go` | `Authorization: Bearer <token>`, `Accept: application/json;version=1`, `Content-Type: application/json`, timeout 15s, HTTP/2 enabled | cfdrs-cdc `api.rs` | audited, partial | local tests | open gap | auth header tests, content-type tests | high | `AUTHORIZATION_BEARER_PREFIX`, `API_ACCEPT_HEADER`, `JSON_CONTENT_TYPE`, `DEFAULT_API_TIMEOUT` constants match Go. API path templates. 2 parity tests. HTTP client with auth header injection still absent. |
| CDC-036 | IP route API | `cfapi/ip_route.go` | `ListRoutes`, `AddRoute`, `DeleteRoute`, `GetByIP` to `/accounts/{accountTag}/teamnet/routes/...` | cfdrs-cdc `api_resources.rs` | audited, partial | local tests | open gap | route API tests, filter query tests | high | `Route`, `DetailedRoute`, `NewRoute` resource types with serde matching Go JSON shape. 2 parity tests. HTTP client and API methods still absent. |
| CDC-037 | virtual network API | `cfapi/virtual_network.go` | `CreateVirtualNetwork`, `ListVirtualNetworks`, `DeleteVirtualNetwork`, `UpdateVirtualNetwork` to `/accounts/{accountTag}/teamnet/virtual_networks/...` | cfdrs-cdc `api_resources.rs` | audited, partial | local tests | open gap | vnet API tests | medium | `VirtualNetwork`, `NewVirtualNetwork`, `UpdateVirtualNetwork` resource types with serde matching Go JSON shape including `is_default_network` rename. 2 parity tests. HTTP client and CRUD methods still absent. |
| CDC-038 | management token API | `cfapi/client.go` `GetManagementToken` | `GetManagementToken(tunnelID, resource)` with resource: logs, admin, host_details | cfdrs-cdc `api_resources.rs` | audited, partial | local tests | open gap | token request tests, resource scope tests | high | `ManagementResource` enum (Logs=0, Admin=1, HostDetails=2) with `repr(u8)` matching Go iota. 2 parity tests including repr value verification. HTTP client and token request method still absent. |
| CDC-039 | hostname routing API | `cfapi/hostname_route.go` `RouteTunnel` | `RouteTunnel(tunnelID, route)` to `/zones/{zoneTag}/tunnels/{tunnelID}/routes` | cfdrs-cdc `api_resources.rs` | audited, partial | local tests | open gap | route API tests | medium | `DnsRouteRequest`, `LbRouteRequest`, `DnsRouteResult`, `LbRouteResult` types with serde matching Go JSON shape including `MarshalJSON` compat. 3 parity tests including Go JSON deserialization. HTTP client and `RouteTunnel` method still absent. |

### Datagram And UDP

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-040 | datagram V2 wire contract | `datagramsession/` and `SessionManager` RPC | session registration via Cap'n Proto RPC, payload via `DatagramMuxerV2` | `cfdrs-cdc/src/datagram.rs`, `cfdrs-bin/src/transport/quic/datagram.rs` | audited, parity-backed | local tests | closed | session lifecycle tests, muxer tests | high | V2 session types, constants, and `format_session_id()` implemented with baseline-matching parity tests. Runtime wired: `DatagramSessionManager` in `cfdrs-bin` implements CDC `SessionManager` trait with `register_session`/`get_session`/`unregister_session`. `dispatch_datagram()` routes by `DatagramType`. `drain_datagrams()` in `lifecycle.rs` loops `dgram_recv`/`dgram_send` in the `serve_streams` loop. Datagrams enabled in quiche config via `enable_dgram(true, 16, 16)`. 12 runtime tests. |
| CDC-041 | datagram V3 wire contract | `quic/v3/` and inline registration | inline binary datagram registration (type 0x0=register, 0x1=payload, 0x2=ICMP, 0x3=response); response codes: OK, DestinationUnreachable, UnableToBindSocket, TooManyActiveFlows, ErrorWithMsg | `cfdrs-cdc/src/datagram.rs`, `cfdrs-bin/src/transport/quic/datagram.rs` | audited, parity-backed | local tests | closed | datagram type tests, response code tests, inline registration tests | medium | `RequestId`, `DatagramType`, all four datagram structs with binary wire marshal/unmarshal, `SessionRegistrationResp`, `SessionError`, and `SessionIdleErr` implemented with 26 CDC parity tests. Runtime wired: `DatagramSessionManager` dispatches registration/payload/ICMP/unknown types matching Go `datagramConn.Serve()` switch. Registration responses are sent back via `dgram_send`. 12 runtime tests. Session manager runtime closed; real UDP forwarding deferred to HIS. |

### Token And Credential Encoding

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-042 | tunnel token encoding | `connection/` tunnel token | JSON fields: `a` (accountTag), `s` (tunnelSecret), `t` (tunnelUUID), `e` (endpoint optional); then base64-encoded | cfdrs-shared `config/credentials/mod.rs` | audited, parity-backed | baseline-backed tests | closed | token encoding roundtrip tests, field mapping tests | high | `TunnelToken` struct with serde `a`/`s`/`t`/`e` renames matching Go. `encode()` uses `BASE64_STANDARD`. `decode()` round-trips correctly. `to_credentials_file()`/`from_credentials_file()` conversions. 4 parity tests: single-letter keys, encode/decode roundtrip, conversion roundtrip, omitted optional field. |
| CDC-043 | origin cert encoding | `connection/` origin cert PEM | PEM block type `ARGO TUNNEL TOKEN`; JSON fields: `zoneID`, `accountID`, `apiToken`, `endpoint` optional | cfdrs-shared `config/credentials/mod.rs` | audited, parity-backed | local tests | closed | PEM parsing tests, field extraction tests | high | `OriginCertToken` struct with serde renames matching Go. PEM block type `ARGO TUNNEL TOKEN` matches baseline. Endpoint lowercased via `to_ascii_lowercase()`. PEM roundtrip, JSON field name, endpoint normalization, error handling tests. 10 credential tests total. |

### QUIC Transport Wire Contract

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-044 | QUIC ALPN protocol | `connection/protocol.go` | QUIC connections use ALPN `"argotunnel"` during TLS handshake | cfdrs-cdc `protocol.rs`, `session.rs` | audited, parity-backed | baseline-backed tests | closed | ALPN negotiation tests, connection rejection tests | medium | `EDGE_QUIC_ALPN` constant in CDC `protocol.rs` is single source of truth. `session.rs` derives its `&[&[u8]]` ALPN from `cfdrs_cdc::protocol::EDGE_QUIC_ALPN.as_bytes()`. Baseline-matching test in protocol.rs. |

## Audit Summary

### Baseline CDC contract inventory (frozen Go)

Registration RPC: 3 active interfaces (`RegistrationServer`,
`SessionManager`, `ConfigurationManager`), 1 composite interface
(`CloudflaredServer`), 7 methods, 10 active Cap'n Proto structs, 5 deprecated
interfaces, ~10 deprecated structs.

Stream contracts: `ConnectRequest`, `ConnectResponse`, `ConnectionType`,
`Metadata` in `quic_metadata_protocol.capnp`. Transport headers for HTTP/2
multiplexing.

Management service: 5 HTTP routes (2 conditional), WebSocket log streaming
with 3 event types, JWT auth middleware, 3 custom close codes.

API client: 4 sub-interfaces, 20+ methods to Cloudflare REST API, JSON
envelope parsing, 4 error mappings.

Metrics/readiness: `/ready` JSON endpoint, `/metrics` Prometheus, `/healthcheck`
text, `/quicktunnel` JSON.

Datagram: V2 (RPC-based) and V3 (inline binary) UDP session management.

Token/credential: tunnel token (base64 JSON), origin cert (PEM).

### Current Rust CDC surface

Implemented: Full registration schema types in `cfdrs-cdc/registration.rs`
(`TunnelAuth`, `ClientInfo` with UUID and features, `ConnectionOptions` with
full field set, `ConnectionDetails`, `ConnectionError` with retry semantics,
`ConnectionResponse` as union enum, `RegisterConnectionRequest`,
`UpdateLocalConfigurationRequest`, `RegisterUdpSessionRequest/Response`,
`UnregisterUdpSessionRequest`, `UpdateConfigurationRequest/Response`).
Feature flags in `cfdrs-cdc/features.rs` with always-on/selector/deprecated
categorization and dedup filtering. Stream types (`ConnectRequest`,
`ConnectResponse`, `ConnectionType`, `Metadata`) in `cfdrs-cdc/stream.rs`.
CDC-owned Cap'n Proto wire codec in `cfdrs-cdc/stream_codec.rs` (request and
response encode/decode with `marshal_capnp`/`unmarshal_capnp`, 10 tests).
Stream contract constants in
`cfdrs-cdc/stream_contract.rs` (metadata keys, header keys, response meta,
control header detection, transport header serialization/deserialization).
Protocol types in `cfdrs-cdc/protocol.rs`
(stream signatures, TLS server names, ALPN, edge discovery DNS, `Protocol`
enum with `tls_settings()`/`fallback()`, `TlsSettings`, `ProtocolSelector`
trait and `StaticProtocolSelector`, `ConnectionStatus` enum with 6 variants,
`ConnectionEvent` struct, 30 tests).
Edge address management in `cfdrs-cdc/edge.rs` (`AddrSet`, `Region`,
`Regions`, `UsedBy` for two-region balanced allocation with IPv6→IPv4
failover; 16 tests).
Cap'n Proto generated bindings from frozen baseline schemas
(`tunnelrpc_capnp` and `quic_metadata_protocol_capnp` modules in cfdrs-cdc)
using `capnp` 0.25.2 runtime and `capnpc` 0.25.0 code generator.
Cap'n Proto binary codec for all six registration schema types plus
`RegisterUdpSessionResponse` and `UpdateConfigurationResponse` in
`cfdrs-cdc/registration_codec.rs` (25 tests). `RegisterUdpSessionRequest`
and `UpdateConfigurationRequest` have `from_rpc_params()` helpers.
`UpdateLocalConfigurationRequest` has `from_config_bytes()`/`to_capnp_bytes()`.

Runtime integration status: Cap'n Proto is now the live wire format for
registration (lifecycle.rs calls `encode_registration_request` /
`decode_registration_response`) and stream framing (lifecycle.rs calls
`decode_connect_request`, proxy/mod.rs calls `encode_connect_response`).
The JSON registration path and custom binary stream framing have been replaced.
Correct ALPN `argotunnel` derived from CDC constant in `cfdrs-bin`. Protocol
events `Registered`, `RegistrationComplete`, `IncomingStream`, `Unregistering`,
`Disconnected`, `ConfigPushed` are all wired with send sites in lifecycle.rs.
Origin HTTP dispatch via reqwest performs real round-trips. Response meta
constants and control header stripping are wired in proxy/origin.rs.
Tunnel credentials loading in `cfdrs-shared`.

Missing: RPC dispatch wiring (capnp-rpc admitted, dispatch not yet wired) for
`unregisterConnection`, `registerUdpSession`/`unregisterUdpSession`, and
`updateConfiguration`; management service and log streaming runtime; all API
client HTTP methods; quicktunnel endpoint runtime; datagram V2/V3 session
lifecycle runtime; feature selector integration with datagram version selection.

### Wire encoding evidence status

Cap'n Proto dependency admitted: `capnp` 0.25.2 (runtime), `capnpc` 0.25.0
(code generator), `capnp-rpc` 0.25.0 (RPC framework), and `capnp-futures`
0.25.0 (async serialization) are workspace-managed dependencies. The cfdrs-cdc build.rs
compiles both frozen baseline schemas (`tunnelrpc.capnp` and
`quic_metadata_protocol.capnp`) into generated Rust bindings. The generated
modules expose typed readers and builders matching the exact byte layout the
Cloudflare edge expects.

Registration Cap'n Proto codec wrappers are implemented in
`registration_codec.rs` with `marshal_capnp` / `unmarshal_capnp` for all six
registration schema types and 17 round-trip tests. Stream framing Cap'n Proto
codec wrappers are implemented in `stream_codec.rs` with
`marshal_capnp` / `unmarshal_capnp` for `ConnectRequest`, `ConnectResponse`,
and `Metadata` plus wire-level `encode_connect_request`/`decode_connect_request`
and `encode_connect_response`/`decode_connect_response` (10 tests). Runtime
integration is complete: `lifecycle.rs` uses `encode_registration_request` /
`decode_registration_response` for registration, `decode_connect_request` for
stream request parsing, and `proxy/mod.rs` uses `encode_connect_response` for
stream response encoding.

Wire encoding evidence artifacts needed before claiming wire parity:

- ~~Cap'n Proto codec wrappers for registration request and response~~ — done
- ~~Cap'n Proto codec wrappers for ConnectRequest and ConnectResponse~~ — done
- ~~runtime integration wiring to replace JSON registration in `lifecycle.rs`~~ — done
- frozen Go Cap'n Proto binary fixtures for roundtrip comparison
- ALPN handshake evidence from Go QUIC connection

### Divergence records

No CDC divergences are currently classified as intentional. Open rows show
`open gap` or partial status; 19 of 44 rows are now closed.

Previously noted structural divergences and their current state:

- **`TunnelAuth.tunnel_id` placement**: resolved. Rust now passes `tunnel_id`
  as a separate field on `RegisterConnectionRequest`, matching the Go
  `registerConnection(auth, tunnelId, ...)` signature.
- **Registration wire encoding**: Cap'n Proto binary codec implemented in
  `registration_codec.rs` with `marshal_capnp` / `unmarshal_capnp` for all
  six registration types (17 round-trip tests). Runtime integration complete:
  `lifecycle.rs` uses `encode_registration_request` /
  `decode_registration_response` as the live wire format.
- **Stream wire encoding**: Cap'n Proto binary codec implemented in
  `stream_codec.rs` with `marshal_capnp` / `unmarshal_capnp` for
  `ConnectRequest`, `ConnectResponse`, and `Metadata` (10 tests). Previously
  used custom big-endian binary format; now replaced with schema-derived
  Cap'n Proto encoding matching the Go `ToPogs()`/`FromPogs()` baseline.
  Runtime integration complete: `lifecycle.rs` calls `decode_connect_request`,
  `proxy/mod.rs` calls `encode_connect_response`.

### Gap ranking by priority

Closed rows (27 of 44):

- CDC-001: registration schema — closed
- CDC-002: registration wire encoding — closed
- CDC-003: registration response — closed
- CDC-004: ClientInfo nesting — closed
- CDC-005: ConnectionOptions full field set — closed
- CDC-006: feature flags — closed
- CDC-008: updateLocalConfiguration RPC — closed
- CDC-009: SessionManager — closed
- CDC-010: ConfigurationManager — closed
- CDC-011: ConnectRequest schema — closed
- CDC-012: ConnectRequest wire framing — closed
- CDC-013: ConnectResponse framing — closed
- CDC-014: metadata key conventions — closed
- CDC-015: transport header serialization — closed
- CDC-016: ResponseMeta contract — closed
- CDC-017: control header stripping — closed
- CDC-018: incoming stream round-trip — closed
- CDC-019: control stream lifecycle — closed
- CDC-020: connection status events — closed
- CDC-021: protocol negotiation — closed
- CDC-022: edge discovery — closed
- CDC-030: healthcheck endpoint — closed
- CDC-042: tunnel token encoding — closed
- CDC-043: origin cert encoding — closed
- CDC-044: QUIC ALPN protocol — closed
- CDC-040: datagram V2 — closed
- CDC-041: datagram V3 — closed

Open critical (5):

- CDC-023: management service routes (partial — route constants defined, management HTTP runtime absent)
- CDC-024: management auth middleware (partial — error types and JSON shape; JWT parsing absent)
- CDC-026: log streaming WebSocket (partial — event/filter types and 16 tests; WebSocket transport absent)
- CDC-033: tunnel CRUD API (partial — resource types with Go JSON matching; HTTP client absent)
- CDC-034: API response envelope (partial — envelope types with 8 tests; HTTP client absent)

Open high (6):

- CDC-007: unregisterConnection RPC (partial — RPC dispatch layer complete; runtime call-site requires capnp-rpc transport on control stream)
- CDC-025: host details contract (partial — `HostDetailsResponse` type and 4 tests; endpoint handler absent)
- CDC-029: readiness endpoint (partial — runtime serves `/ready`; CDC column defers to HIS+runtime)
- CDC-035: API auth and headers (partial — constants defined; HTTP client with auth injection absent)
- CDC-036: IP route API (partial — resource types; HTTP client absent)
- CDC-038: management token API (partial — `ManagementResource` enum; HTTP client absent)

Open medium (6):

- CDC-027: management CORS (deferred — constants defined, middleware absent)
- CDC-028: diagnostics conditional exposure (deferred — constant defined, conditional registration absent)
- CDC-031: Prometheus metrics endpoint (partial — runtime serves `/metrics`; full metric-name parity not exhaustively verified)
- CDC-032: quicktunnel endpoint (deferred — response type defined, runtime wiring absent)
- CDC-037: virtual network API (partial — resource types; HTTP client absent)
- CDC-039: hostname routing API (partial — request/result types; HTTP client absent)

## Scope Classification

Lane classification is recorded directly in this ledger for roadmap and promotion use.

All items not listed below are **lane-required** for the declared Linux
production-alpha lane.

### Deferred (lane-relevant, post-alpha)

- CDC-027: management CORS — enables dash browser access, not required for
  CLI-based `tail` and management workflows
- CDC-028: diagnostics conditional exposure — `/metrics` and `/debug/pprof`
  conditional registration on management service, debug tooling
- CDC-032: `/quicktunnel` endpoint response — convenience feature
- CDC-039: hostname routing API — legacy DNS routing via zones

## Immediate Work Queue

1. ~~extract the field-level registration schema and method set from the
   frozen Cap'n Proto baseline~~ — done; see
   [docs/parity/cdc/registration-rpc.md](registration-rpc.md)
2. ~~record the actual frozen registration wire encoding and framing behavior
   separately from Rust logical-type coverage~~ — done; wire encoding
   divergence (JSON vs Cap'n Proto) documented in registration-rpc.md
3. ~~compare current Rust registration and stream types against the frozen
   schemas field by field~~ — done; field-level comparison in
   registration-rpc.md and stream-contracts.md
4. ~~inventory current Rust actual wire behavior and record where it differs
   from the frozen contract~~ — done; divergences documented in both
   feature-group docs
5. ~~inventory management routes, auth gates, and diagnostics exposure from
   the frozen baseline~~ — done; see
   [docs/parity/cdc/management-and-diagnostics.md](management-and-diagnostics.md)
6. ~~inventory log-streaming session behavior, limits, and output contract
   from the frozen baseline~~ — done; see management-and-diagnostics.md
7. ~~inventory externally relevant readiness and metrics contracts for the
   declared lane~~ — done; see
   [docs/parity/cdc/metrics-readiness-and-api.md](metrics-readiness-and-api.md)
8. ~~split this ledger into feature-group documents~~ — done; four
   feature-group audit documents created:
   - [docs/parity/cdc/registration-rpc.md](registration-rpc.md)
   - [docs/parity/cdc/stream-contracts.md](stream-contracts.md)
   - [docs/parity/cdc/management-and-diagnostics.md](management-and-diagnostics.md)
   - [docs/parity/cdc/metrics-readiness-and-api.md](metrics-readiness-and-api.md)
