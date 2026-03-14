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
- open gap
- intentional divergence
- unknown
- blocked

## Audited Checklist

This checklist was produced by source-level audit of the frozen Go baseline
in [baseline-2026.2.0/old-impl/](../../../baseline-2026.2.0/old-impl/) and comparison against the current Rust CDC
surface in [crates/cfdrs-cdc/](../../../crates/cfdrs-cdc/) and [crates/cfdrs-bin/](../../../crates/cfdrs-bin/).

The frozen Go CDC surface uses Cap'n Proto for registration and stream
framing. The current Rust CDC surface uses JSON for registration and a custom
big-endian binary format for stream framing.

### Registration RPC

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-001 | registration RPC schema | `tunnelrpc/proto/tunnelrpc.capnp` | `RegistrationServer.registerConnection(auth: TunnelAuth, tunnelId: Data, connIndex: UInt8, options: ConnectionOptions) -> ConnectionResponse`. Schema IDs: RegistrationServer `@0xf71695ec7fe85497`, TunnelAuth `@0x9496331ab9cd463f`, ConnectionOptions `@0xb4bf9861fe035d04`, ConnectionResponse `@0xdbaa9d03d52b62dc` | cfdrs-cdc `registration.rs` | audited, partial | minimal | open gap | schema field compare, method signature tests, Cap'n Proto codec tests | critical | Rust has `TunnelAuth`, `ConnectionOptions`, `ConnectionDetails` but nesting and field set differ from Cap'n Proto schema. See [docs/parity/cdc/registration-rpc.md](registration-rpc.md) |
| CDC-002 | registration wire encoding | `tunnelrpc/registration_client.go`, capnp-go marshal | registration request/response encoded as Cap'n Proto binary over QUIC control stream (stream ID 0) | current QUIC transport `lifecycle.rs` | audited, partial | weak | open gap | frozen-fixture wire tests, Cap'n Proto binary roundtrip tests | critical | Rust uses JSON via `serde_json`; Go uses Cap'n Proto binary. This is the primary wire encoding divergence. |
| CDC-003 | registration response contract | `ConnectionResponse` union: `error(ConnectionError)` or `connectionDetails(ConnectionDetails)`. `ConnectionError` has `cause`, `retryAfter` (Int64 ns), `shouldRetry` (Bool). `ConnectionDetails` has `uuid`, `locationName`, `tunnelIsRemotelyManaged` | success returns `ConnectionDetails`; error returns structured `ConnectionError` with retry semantics | cfdrs-cdc `registration.rs` | audited, partial | weak | open gap | response golden tests, error retry-semantics tests, ConnectionError field tests | high | Rust `RegisterConnectionResponse` uses flat `error: String` + `Option<ConnectionDetails>` instead of union. Missing `retryAfter` and `shouldRetry` fields. |
| CDC-004 | ClientInfo nesting and fields | `ClientInfo` struct: `clientId` (Data, 16-byte UUID), `features` (List(Text)), `version` (Text), `arch` (Text). Nested inside `ConnectionOptions.client` | registration sends client identity with UUID and capability list | cfdrs-cdc `registration.rs` | audited, partial | minimal | open gap | clientId UUID tests, features list tests, nesting shape tests | high | Rust flattens `ClientInfo` fields into `ConnectionOptions`; missing `clientId` and `features` fields entirely |
| CDC-005 | ConnectionOptions full field set | `ConnectionOptions`: `client` (ClientInfo), `originLocalIp` (Data), `replaceExisting` (Bool), `compressionQuality` (UInt8), `numPreviousAttempts` (UInt8) | all fields sent to edge during registration | cfdrs-cdc `registration.rs` | audited, partial | minimal | open gap | field-level tests, default-value tests | high | Rust missing `replaceExisting` and `compressionQuality`; has extra `edge_addr` (local-only) |
| CDC-006 | feature flags sent during registration | `ConnectionOptions.Client.Features`: default set `allow_remote_config`, `serialized_headers`, `support_datagram_v2`, `support_quic_eof`, `management_logs`; selector-added: `support_datagram_v3_2`, `postquantum`; CLI-passthrough only: `quick_reconnects`; deprecated (filtered before send): `support_datagram_v3`, `support_datagram_v3_1` | capability list negotiates edge behavior at registration time | none | audited, absent | not present | open gap | feature list tests, deprecated-feature filtering tests, selector logic tests | high | no feature flag mechanism in Rust; `quick_reconnects` is a constant but never added by Go selector's `clientFeatures()` — only appears if passed via CLI |
| CDC-007 | unregisterConnection RPC | `RegistrationServer.unregisterConnection()` | graceful shutdown over control stream with configurable grace period | none | audited, absent | not present | open gap | graceful shutdown tests, grace period tests | medium | Rust has no graceful disconnect RPC |
| CDC-008 | updateLocalConfiguration RPC | `RegistrationServer.updateLocalConfiguration(config: Data)` | pushes tunnel config to edge on connIndex==0 when not remotely managed | none | audited, absent | not present | open gap | config push tests, connIndex==0 guard tests | medium | Rust has no config push to edge |
| CDC-009 | SessionManager interface | `SessionManager.registerUdpSession()` and `unregisterUdpSession()` | UDP session lifecycle over Cap'n Proto RPC | none | audited, absent | not present | open gap | session registration tests, session cleanup tests | high | entire UDP session RPC absent |
| CDC-010 | ConfigurationManager interface | `ConfigurationManager.updateConfiguration(version: Int32, config: Data) -> UpdateConfigurationResponse` | remote edge pushes config updates to cloudflared | none | audited, absent | not present | open gap | config update tests, version tracking tests | medium | remote config management absent |

### Stream Contracts

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-011 | ConnectRequest schema | `quic_metadata_protocol.capnp`: `ConnectRequest` with `dest` (Text), `type` (ConnectionType), `metadata` (List(Metadata)). `ConnectionType` enum: http=0, websocket=1, tcp=2 | per-stream request shape over QUIC data streams | cfdrs-cdc `stream.rs` | audited, partial | local tests | open gap | schema compare, enum value tests, metadata key convention tests | critical | Rust logical types match Go schema fields; metadata key constants match. Wire encoding differs. |
| CDC-012 | ConnectRequest wire framing | `tunnelrpc/pogs/quic_metadata_protocol.go` uses Cap'n Proto binary marshaling via `ToPogs()` | stream requests encoded as Cap'n Proto binary | current QUIC transport `lifecycle.rs` | audited, partial | local tests | open gap | frozen-fixture wire tests, binary format comparison, malformed-input tests | critical | Rust uses custom big-endian binary format (type u16, dest len u16, dest bytes, metadata count u16, per-entry key-len/key/val-len/val). Not Cap'n Proto. |
| CDC-013 | ConnectResponse schema and framing | `quic_metadata_protocol.capnp`: `ConnectResponse` with `error` (Text), `metadata` (List(Metadata)). Cap'n Proto binary encoding. | per-stream response shape back to edge | cfdrs-cdc `stream.rs` | audited, partial | minimal | open gap | schema compare, response construction tests, wire encoding tests | high | Rust types match logically but ConnectResponse is not wired into the response path. No wire encoding for responses. |
| CDC-014 | metadata key conventions | `quic_metadata_protocol.go` and `connection/header.go` | keys: `HttpMethod`, `HttpHost`, `HttpHeader:<name>`, `HttpStatus`, `FlowID`, `cf-trace-id`, `HttpHeader:Content-Length` | cfdrs-cdc `stream.rs` | audited, partial | local tests | open gap | metadata key inventory tests, accessor tests | medium | Rust defines matching constants. Accessors exist. Missing evidence that all edge-expected keys are handled. |
| CDC-015 | transport header serialization | `connection/header.go` | base64.RawStdEncoding pairs joined by `;` for `cf-cloudflared-request-headers` and `cf-cloudflared-response-headers`; JSON for `cf-cloudflared-response-meta` | none | audited, absent | not present | open gap | header serialization roundtrip tests, base64 encoding tests | high | entire header serialization layer absent; required for HTTP/2 transport and for correct edge communication over QUIC where metadata carry headers |
| CDC-016 | ResponseMeta contract | `connection/header.go` | pre-generated JSON: `{"src":"origin"}`, `{"src":"cloudflared"}`, `{"src":"cloudflared","flow_rate_limited":true}` | none | audited, absent | not present | open gap | response meta shape tests | medium | response source attribution absent |
| CDC-017 | control header stripping | `connection/header.go` `IsControlResponseHeader` | headers with prefixes `:`, `cf-int-`, `cf-cloudflared-`, `cf-proxy-` stripped from user-visible responses | none | audited, absent | not present | open gap | control header detection tests, stripping tests | medium | internal headers would leak to users without this |
| CDC-018 | incoming stream round-trip | stream-serving runtime path (proxy/origin). Go path: AcceptStream → runStream → ReadConnectRequestData → dispatchRequest → GetOriginProxy → type switch (HTTP/WS/TCP) → ingressRules.FindMatchingRule → origin service → response via httpResponseAdapter | request accepted, matched to ingress, proxied to origin, response returned through tunnel | current proxy `origin.rs` | audited, partial | weak | open gap | end-to-end stream tests, origin round-trip tests, error handling tests | critical | dispatch path wired for HttpStatus (status code), HelloWorld (200+HTML), Http(url) (dispatch wired but actual origin HTTP round-trip returns 502 — origin connection NOT implemented); TCP/WebSocket/Unix/Bastion/SocksProxy/NamedToken are explicit `Unimplemented` stubs |

### Control Stream And Lifecycle

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-019 | control stream lifecycle | `connection/control.go` `ControlStreamHandler` | open control stream → register → optionally send local config → monitor for graceful shutdown → unregister | current QUIC transport `lifecycle.rs` and `protocol.rs` | audited, partial | partial | open gap | lifecycle integration tests, stage transition tests | high | Rust reports `Registered`, `RegistrationComplete`, `IncomingStream` events. Missing: config push, graceful shutdown, unregister. |
| CDC-020 | connection status events | `connection/event.go` | `Event` with Index, EventType (Disconnected=0, Connected=1, Reconnecting=2, SetURL=3, RegisteringTunnel=4, Unregistering=5), Location, Protocol, URL, EdgeAddress | current protocol `protocol.rs` | audited, partial | local tests | open gap | event type inventory tests, transition tests | medium | Rust `ProtocolBridgeState` has: BridgeUnavailable, BridgeCreated, RegistrationSent, RegistrationObserved, BridgeClosed. Different granularity from Go. |
| CDC-021 | protocol negotiation | `connection/protocol.go` | Protocol enum: HTTP2=0, QUIC=1. TLS server names: `h2.cftunnel.com` (HTTP/2), `quic.cftunnel.com` (QUIC). QUIC ALPN: `argotunnel`. Fallback: QUIC→HTTP/2. | current QUIC transport `edge.rs` | audited, partial | partial | open gap | protocol selection tests, SNI tests, ALPN tests, fallback tests | high | Rust QUIC-only with `quic.cftunnel.com` SNI. ALPN `argotunnel` not verified in Rust. No HTTP/2 transport or fallback. |
| CDC-022 | edge discovery | `edgediscovery/` | SRV record `_v2-origintunneld._tcp.argotunnel.com`, DNS-over-TLS fallback (dial `1.1.1.1:853`, TLS serverName `cloudflare-dns.com`), priority+weight sorting via Go stdlib, region1+region2 redundancy | current QUIC transport `edge.rs` | audited, partial | weak | open gap | SRV record tests, DoT fallback tests, region failover tests | high | Rust uses DNS A/AAAA via `tokio::net::lookup_host` (no SRV), only `region1.v2.argotunnel.com` (no region2 fallback), hardcoded `quic.cftunnel.com` SNI |

### Management And Log Streaming

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-023 | management service routes | `management/service.go` | chi router routes: `/ping` (GET/HEAD), `/logs` (GET→WS), `/host_details` (GET), `/metrics` (GET, conditional), `/debug/pprof/{heap or goroutine}` (GET, conditional). All require token query middleware. | none | audited, absent | not present | open gap | endpoint contract tests, route inventory tests, conditional route tests | critical | entire management HTTP service absent. See [docs/parity/cdc/management-and-diagnostics.md](management-and-diagnostics.md) |
| CDC-024 | management auth middleware | `management/middleware.go` | `?access_token=<JWT>` query param required; parsed via `ParseToken`; error: `{"success":false,"errors":[{"code":1001,"message":"missing access_token..."}]}` with 400 status | none | audited, absent | not present | open gap | auth middleware tests, error response tests, JWT validation tests | critical | no JWT token validation for management routes |
| CDC-025 | host details contract | `management/service.go` `getHostDetailsResponse` | JSON: `{"connector_id":"uuid","ip":"10.0.0.4","hostname":"custom:label"}` | none | audited, absent | not present | open gap | response shape tests, field derivation tests | high | connector identity endpoint absent |
| CDC-026 | log streaming WebSocket | `management/events.go` and `session.go` | WebSocket upgrade on `/logs`; client sends `start_streaming` / `stop_streaming`; server sends `logs` with `[{time, level, message, event, fields}]`; filters: events (cloudflared/http/tcp/udp), level (debug/info/warn/error), sampling (0-1); close codes: 4001/4002/4003 | none | audited, absent | not present | open gap | WebSocket event tests, filter tests, sampling tests, close code tests, session limit tests | critical | entire log streaming protocol absent |
| CDC-027 | management CORS | `management/service.go` corsHandler | allowed origins: `https://*.cloudflare.com`; credentials: true; maxAge: 300 | none | audited, absent | not present | open gap | CORS header tests | medium | dash access requires CORS |
| CDC-028 | diagnostics conditional exposure | `management/service.go` | `/metrics` and `/debug/pprof` only registered when `enableDiagServices=true` | none | audited, absent | not present | open gap | conditional route tests, gating tests | medium | diagnostic routes must be conditionally exposed |

### Metrics And Readiness

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-029 | readiness endpoint contract | `metrics/readiness.go` | `GET /ready` returns JSON `{"status":200,"readyConnections":N,"connectorId":"uuid"}` with HTTP 200 if active conns > 0, else 503 | none | audited, absent | not present | open gap | HTTP contract tests, ready/not-ready semantics tests | high | CDC owns response contract; HIS-025 owns local HTTP exposure. See [docs/parity/cdc/metrics-readiness-and-api.md](metrics-readiness-and-api.md) |
| CDC-030 | healthcheck endpoint | `metrics/metrics.go` | `GET /healthcheck` returns text `OK\n` with HTTP 200 | none | audited, absent | not present | open gap | liveness tests | medium | CDC owns response contract; HIS-026 owns local HTTP exposure |
| CDC-031 | Prometheus metrics endpoint | `metrics/metrics.go` | `GET /metrics` served by `promhttp.Handler()` | none | audited, absent | not present | open gap | endpoint tests, metric-name tests | medium | CDC owns metric names and labels; HIS-027 owns local HTTP exposure |
| CDC-032 | quicktunnel endpoint | `metrics/metrics.go` | `GET /quicktunnel` returns `{"hostname":"<hostname>"}` | none | audited, absent | not present | open gap | quicktunnel response tests | low | CDC owns response contract; HIS-028 owns local HTTP exposure |

### Cloudflare REST API

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-033 | tunnel CRUD API | `cfapi/tunnel.go` and `client.go` | `CreateTunnel`, `GetTunnel`, `GetTunnelToken`, `DeleteTunnel`, `ListTunnels`, `ListActiveClients`, `CleanupConnections` to `/accounts/{accountTag}/cfd_tunnel/...` | none | audited, absent | not present | open gap | API request shape tests, response envelope tests, error mapping tests | critical | blocks tunnel create/list/delete CLI commands. See [docs/parity/cdc/metrics-readiness-and-api.md](metrics-readiness-and-api.md) |
| CDC-034 | API response envelope | `cfapi/base_client.go` | JSON envelope: `{"success":true,"errors":[],"messages":[],"result":...,"result_info":{...}}`. Error mapping: 400→ErrBadRequest, 401/403→ErrUnauthorized, 404→ErrNotFound | none | audited, absent | not present | open gap | envelope parsing tests, error mapping tests | critical | required by all API methods |
| CDC-035 | API auth and headers | `cfapi/base_client.go` | `Authorization: Bearer <token>`, `Accept: application/json;version=1`, `Content-Type: application/json`, timeout 15s, HTTP/2 enabled | none | audited, absent | not present | open gap | auth header tests, content-type tests | high | all API calls require this |
| CDC-036 | IP route API | `cfapi/ip_route.go` | `ListRoutes`, `AddRoute`, `DeleteRoute`, `GetByIP` to `/accounts/{accountTag}/teamnet/routes/...` | none | audited, absent | not present | open gap | route API tests, filter query tests | high | blocks tunnel route CLI commands |
| CDC-037 | virtual network API | `cfapi/virtual_network.go` | `CreateVirtualNetwork`, `ListVirtualNetworks`, `DeleteVirtualNetwork`, `UpdateVirtualNetwork` to `/accounts/{accountTag}/teamnet/virtual_networks/...` | none | audited, absent | not present | open gap | vnet API tests | medium | blocks tunnel vnet CLI commands |
| CDC-038 | management token API | `cfapi/client.go` `GetManagementToken` | `GetManagementToken(tunnelID, resource)` with resource: logs, admin, host_details | none | audited, absent | not present | open gap | token request tests, resource scope tests | high | blocks management CLI workflows |
| CDC-039 | hostname routing API | `cfapi/hostname_route.go` `RouteTunnel` | `RouteTunnel(tunnelID, route)` to `/zones/{zoneTag}/tunnels/{tunnelID}/routes` | none | audited, absent | not present | open gap | route API tests | medium | legacy DNS routing |

### Datagram And UDP

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-040 | datagram V2 wire contract | `datagramsession/` and `SessionManager` RPC | session registration via Cap'n Proto RPC, payload via `DatagramMuxerV2` | none | audited, absent | not present | open gap | session lifecycle tests, muxer tests | high | UDP session lifecycle not implemented |
| CDC-041 | datagram V3 wire contract | `quic/v3/` and inline registration | inline binary datagram registration (type 0x0=register, 0x1=payload, 0x2=ICMP, 0x3=response); response codes: OK, DestinationUnreachable, UnableToBindSocket, TooManyActiveFlows, ErrorWithMsg | none | audited, absent | not present | open gap | datagram type tests, response code tests, inline registration tests | medium | V3 replaces RPC registration with inline binary framing |

### Token And Credential Encoding

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-042 | tunnel token encoding | `connection/` tunnel token | JSON fields: `a` (accountTag), `s` (tunnelSecret), `t` (tunnelUUID), `e` (endpoint optional); then base64-encoded | cfdrs-shared `config/credentials/mod.rs` | audited, partial | local tests | open gap | token encoding roundtrip tests, field mapping tests | high | Rust `TunnelCredentialsFile` has matching fields but single-letter JSON key mapping not verified |
| CDC-043 | origin cert encoding | `connection/` origin cert PEM | PEM block type `ARGO TUNNEL TOKEN`; JSON fields: `zoneID`, `accountID`, `apiToken`, `endpoint` optional | cfdrs-shared `config/credentials/mod.rs` | audited, partial | local tests | open gap | PEM parsing tests, field extraction tests | high | Rust `OriginCertToken` exists with matching fields |

### QUIC Transport Wire Contract

| ID | Feature group | Baseline source | Baseline behavior or contract | Rust owner now | Rust status now | Parity evidence status | Divergence status | Required tests | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CDC-044 | QUIC ALPN protocol | `connection/protocol.go` | QUIC connections use ALPN `\"argotunnel\"` during TLS handshake | current QUIC transport `session.rs` | audited, parity-backed | local tests | none recorded | ALPN negotiation tests, connection rejection tests | medium | Rust sets ALPN `argotunnel` via `EDGE_QUIC_ALPN` constant in `session.rs` — matches Go |

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

Implemented: `TunnelAuth`, `ConnectionOptions` (partial), `ConnectionDetails`,
`ConnectRequest`, `ConnectResponse`, `ConnectionType`, `Metadata` types in
`cfdrs-cdc`. QUIC transport with JSON registration, custom binary
stream framing, and correct ALPN `argotunnel` in `cfdrs-bin`. Tunnel
credentials loading in `cfdrs-shared`.

Missing: Cap'n Proto wire encoding, `ClientInfo` nesting, feature flags,
`ConnectionError` richness, `SessionManager`, `ConfigurationManager`,
graceful shutdown, management service, log streaming, all API client methods,
readiness/metrics/healthcheck endpoints, datagram V2/V3, transport header
serialization, SRV-based edge discovery, region2
fallback, actual origin HTTP round-trip for `Http(url)` dispatch.

### Wire encoding evidence status

The audit documents describe the wire encoding differences (JSON vs Cap'n
Proto for registration, custom big-endian binary vs Cap'n Proto for stream
framing) but no actual binary fixture captures from the frozen Go baseline
exist. The Rust codebase has test-only helpers (`serialize_connect_request`,
`serialize_registration_response`) but no golden fixtures from the Go Cap'n
Proto serialization for comparison.

Wire encoding evidence artifacts needed before claiming wire parity:

- frozen Go Cap'n Proto binary fixture for `registerConnection` request
- frozen Go Cap'n Proto binary fixture for `ConnectRequest`
- frozen Go Cap'n Proto binary fixture for `ConnectResponse`
- ALPN handshake evidence from Go QUIC connection

These fixtures should be generated by running Go test code against the frozen
baseline capnp schemas. This is deferred to the active implementation milestones.

### Divergence records

No CDC divergences are currently classified as intentional. All 44 checklist
entries with divergences show `open gap` status. Two structural
divergences may be candidates for intentional classification in later stages:

- **`TunnelAuth.tunnel_id` placement**: Rust bundles `tunnel_id` into
  `TunnelAuth`; Go passes it as a separate `registerConnection` parameter.
  Whether this is retained or aligned depends on the Cap'n Proto wire encoding
  decision.
- **JSON registration encoding**: Rust uses JSON for registration
  request/response; Go uses Cap'n Proto binary. Whether this is an
  intentional simplification or a temporary gap is unknown — it depends on
  whether the Cloudflare edge accepts JSON as an alternative encoding, which
  requires edge-side confirmation.

### Gap ranking by priority

Critical gaps:

- CDC-001: registration schema (Cap'n Proto schema vs Rust types)
- CDC-002: registration wire encoding (Cap'n Proto binary vs JSON)
- CDC-011: ConnectRequest schema (logical match, wire divergence)
- CDC-012: ConnectRequest wire framing (custom binary vs Cap'n Proto)
- CDC-018: incoming stream round-trip (partial proxy)
- CDC-023: management service routes (entirely absent)
- CDC-024: management auth middleware (entirely absent)
- CDC-026: log streaming WebSocket (entirely absent)
- CDC-033: tunnel CRUD API (entirely absent)
- CDC-034: API response envelope (entirely absent)

High gaps:

- CDC-003: registration response (missing error richness)
- CDC-004: ClientInfo nesting (missing clientId, features)
- CDC-005: ConnectionOptions full field set
- CDC-006: feature flags (none sent to edge)
- CDC-009: SessionManager (UDP lifecycle absent)
- CDC-013: ConnectResponse (not wired)
- CDC-015: transport header serialization
- CDC-019: control stream lifecycle (partial)
- CDC-021: protocol negotiation (QUIC-only, no fallback, ALPN unverified)
- CDC-022: edge discovery (no SRV, no region2 fallback)
- CDC-025: host details contract
- CDC-029: readiness endpoint
- CDC-035: API auth and headers
- CDC-036: IP route API
- CDC-038: management token API
- CDC-040: datagram V2
- CDC-042: tunnel token encoding
- CDC-043: origin cert encoding
- CDC-044: QUIC ALPN protocol (parity-backed)

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
