# CDC Feature Group: Registration RPC

## Scope

This document covers the registration RPC contract between cloudflared and the
Cloudflare edge, as defined by the frozen Go baseline Cap'n Proto schema and
the Go registration client/server implementation.

Registration is the control-plane handshake that establishes a tunnel
connection with the edge.

## Frozen Baseline Schema

Source: [baseline-2026.2.0/tunnelrpc/proto/tunnelrpc.capnp](../../../baseline-2026.2.0/tunnelrpc/proto/tunnelrpc.capnp)

### Active Interfaces

#### RegistrationServer (`@0xf71695ec7fe85497`)

Methods:

- `registerConnection(auth: TunnelAuth, tunnelId: Data, connIndex: UInt8, options: ConnectionOptions) -> (result: ConnectionResponse)`
- `unregisterConnection() -> ()`
- `updateLocalConfiguration(config: Data) -> ()`

#### SessionManager (`@0x839445a59fb01686`)

Methods:

- `registerUdpSession(sessionId: Data, dstIp: Data, dstPort: UInt16, closeAfterIdleHint: Int64, traceContext: Text = "") -> (result: RegisterUdpSessionResponse)`
- `unregisterUdpSession(sessionId: Data, message: Text) -> ()`

#### ConfigurationManager (`@0xb48edfbdaa25db04`)

Methods:

- `updateConfiguration(version: Int32, config: Data) -> (result: UpdateConfigurationResponse)`

#### CloudflaredServer (`@0xf548cef9dea2a4a1`)

Extends `SessionManager` and `ConfigurationManager`. No additional methods.

### Active Structs

#### TunnelAuth (`@0x9496331ab9cd463f`)

| Field | Type | Ordinal |
| --- | --- | --- |
| `accountTag` | Text | @0 |
| `tunnelSecret` | Data | @1 |

#### ClientInfo (`@0x83ced0145b2f114b`)

| Field | Type | Ordinal |
| --- | --- | --- |
| `clientId` | Data | @0 |
| `features` | List(Text) | @1 |
| `version` | Text | @2 |
| `arch` | Text | @3 |

#### ConnectionOptions (`@0xb4bf9861fe035d04`)

| Field | Type | Ordinal |
| --- | --- | --- |
| `client` | ClientInfo | @0 |
| `originLocalIp` | Data | @1 |
| `replaceExisting` | Bool | @2 |
| `compressionQuality` | UInt8 | @3 |
| `numPreviousAttempts` | UInt8 | @4 |

#### ConnectionResponse (`@0xdbaa9d03d52b62dc`)

Union of:

- `error: ConnectionError` (@0)
- `connectionDetails: ConnectionDetails` (@1)

#### ConnectionError (`@0xf5f383d2785edb86`)

| Field | Type | Ordinal |
| --- | --- | --- |
| `cause` | Text | @0 |
| `retryAfter` | Int64 | @1 |
| `shouldRetry` | Bool | @2 |

#### ConnectionDetails (`@0xb5f39f082b9ac18a`)

| Field | Type | Ordinal |
| --- | --- | --- |
| `uuid` | Data | @0 |
| `locationName` | Text | @1 |
| `tunnelIsRemotelyManaged` | Bool | @2 |

#### RegisterUdpSessionResponse (`@0xab6d5210c1f26687`)

| Field | Type | Ordinal |
| --- | --- | --- |
| `err` | Text | @0 |
| `spans` | Data | @1 |

#### UpdateConfigurationResponse (`@0xdb58ff694ba05cf9`)

| Field | Type | Ordinal |
| --- | --- | --- |
| `latestAppliedVersion` | Int32 | @0 |
| `err` | Text | @1 |

### Deprecated Interfaces (Schema Retained For Wire Compat)

- `TunnelServer` (`@0xea58385c65416035`): `registerTunnel`, `getServerInfo`,
  `unregisterTunnel`, `obsoleteDeclarativeTunnelConnect`, `authenticate`,
  `reconnectTunnel`

Deprecated structs: `Authentication`, `TunnelRegistration`,
`RegistrationOptions`, `ServerInfo`, `AuthenticateResponse`, `Tag`,
`ExistingTunnelPolicy`

These remain in the schema to prevent Cap'n Proto ID reuse collisions.

## Feature Flags Sent During Registration

Default features in `ConnectionOptions.Client.Features`:

- `allow_remote_config`
- `serialized_headers`
- `support_datagram_v2`
- `support_quic_eof`
- `management_logs`

Optional features:

- `support_datagram_v3_2`
- `postquantum`
- `quick_reconnects`

Deprecated features (filtered before sending):

- `support_datagram_v3` (TUN-9291)
- `support_datagram_v3_1` (TUN-9883)

## Frozen Go Registration Client Behavior

Source: [baseline-2026.2.0/tunnelrpc/registration_client.go](../../../baseline-2026.2.0/tunnelrpc/registration_client.go)

### RegisterConnection Flow

1. builds `TunnelAuth` from credentials (accountTag + tunnelSecret)
2. sends `registerConnection` RPC with auth, tunnelId (as Data), connIndex
   (UInt8), and ConnectionOptions (including nested ClientInfo)
3. receives `ConnectionResponse` union
4. on success: extracts `ConnectionDetails` (uuid, locationName,
   tunnelIsRemotelyManaged)
5. on error: extracts `ConnectionError` (cause, retryAfter, shouldRetry)

### SendLocalConfiguration

- called only on `connIndex == 0` and only when not remotely managed
- sends raw config bytes via `updateLocalConfiguration` RPC

### GracefulShutdown

- sends `unregisterConnection` RPC
- used with configurable grace period

### Wire Transport

- Cap'n Proto binary encoding over the QUIC control stream (stream ID 0)
- Go uses `capnp-go` library for marshal/unmarshal
- QUIC connection established with ALPN `"argotunnel"` and TLS server name
  `quic.cftunnel.com`

## Control Stream Lifecycle

Source: [baseline-2026.2.0/connection/control.go](../../../baseline-2026.2.0/connection/control.go) (lines 78-147)
and [baseline-2026.2.0/connection/quic_connection.go](../../../baseline-2026.2.0/connection/quic_connection.go) (line 89)

### Control Stream Opening

The control stream is the **first QUIC stream** opened on the connection:

```go
controlStream, err := q.conn.OpenStream()
```

This is called from `QUICConnection.Serve()` in `quic_connection.go`. The
control stream carries all registration RPC traffic.

### Lifecycle Stages

The `ControlStreamHandler.ServeControlStream()` method executes this exact
sequence:

1. **create RPC client**: `registrationClient = registerClientFunc(ctx, rw,
   registerTimeout)` — wraps the control stream in a Cap'n Proto RPC transport
2. **register**: `registrationClient.RegisterConnection(ctx, auth, tunnelID,
   connOptions, connIndex, edgeAddress)` — sends the registration RPC
3. **log connected**: `observer.logConnected(...)` and
   `observer.sendConnectedEvent(...)` and `connectedFuse.Connected()`
4. **optionally send local config**: only when `connIndex == 0` AND
   `tunnelIsRemotelyManaged == false` — calls
   `registrationClient.SendLocalConfiguration(ctx, tunnelConfig)`
5. **wait for shutdown**: blocks on `ctx.Done()` or `gracefulShutdownC` channel
6. **unregister**: sends `observer.sendUnregisteringEvent(connIndex)`, then
   calls `registrationClient.GracefulShutdown(ctx, gracePeriod)`, then logs
   "Unregistered tunnel connection"

### Current Rust Control Stream

Source: [crates/cfdrs-bin/src/transport/quic/lifecycle.rs](../../../crates/cfdrs-bin/src/transport/quic/lifecycle.rs)

The Rust control stream now uses the CDC-owned Cap'n Proto codec in the live
runtime path:

- opens the first QUIC stream (bidirectional)
- encodes `RegisterConnectionRequest` with
  `encode_registration_request()` and writes the Cap'n Proto payload on
  stream 0
- decodes `ConnectionResponse` with `decode_registration_response()`
- emits `ProtocolEvent::Registered` and
  `ProtocolEvent::RegistrationComplete`
- when `connIndex == 0` and the edge reports
  `tunnelIsRemotelyManaged == false`, serializes
  `UpdateLocalConfigurationRequest` and pushes it on the same control stream
- on graceful shutdown, emits `Unregistering`, encodes
  `unregisterConnection`, and sends the final control-stream message with
  `fin=true`

Remaining differences from Go:

- no `connectedFuse` / channel-style readiness fuse; the Rust runtime uses
  `ProtocolEvent` delivery and runtime status recording instead
- origin-cert-backed registration content remains a bounded runtime gap; the
  admitted live path is credential-file-backed registration auth

## Current Rust Registration Surface

Source: [crates/cfdrs-cdc/src/registration.rs](../../../crates/cfdrs-cdc/src/registration.rs)
and [crates/cfdrs-cdc/src/registration_codec.rs](../../../crates/cfdrs-cdc/src/registration_codec.rs)

### TunnelAuth

```rust
pub struct TunnelAuth {
    pub account_tag: String,
    pub tunnel_secret: Vec<u8>,
}
```

**Status:** field-level match with baseline schema. `tunnel_id` is a separate
parameter on `RegisterConnectionRequest`, matching the Go
`registerConnection(auth, tunnelId, ...)` signature.

### ClientInfo

```rust
pub struct ClientInfo {
    pub client_id: Vec<u8>,
    pub features: Vec<String>,
    pub version: String,
    pub arch: String,
}
```

**Status:** field-level match. `client_id` is 16-byte UUID bytes.
`for_current_platform()` constructor builds default values.

### ConnectionOptions

```rust
pub struct ConnectionOptions {
    pub client: ClientInfo,
    pub origin_local_ip: Option<IpAddr>,
    pub replace_existing: bool,
    pub compression_quality: u8,
    pub num_previous_attempts: u8,
}
```

**Status:** field-level match with `ClientInfo` properly nested. All five
Cap'n Proto schema fields present. `for_current_platform()` constructor
available.

### ConnectionDetails

```rust
pub struct ConnectionDetails {
    pub uuid: Uuid,
    pub location: String,
    pub is_remotely_managed: bool,
}
```

**Status:** field-level match (naming differs: Go `locationName` vs Rust
`location`, Go `tunnelIsRemotelyManaged` vs Rust `is_remotely_managed`).

### ConnectionError

```rust
pub struct ConnectionError {
    pub cause: String,
    pub retry_after_ns: i64,
    pub should_retry: bool,
}
```

**Status:** field-level match with retry semantics. `retry_after()` method
converts nanoseconds to `Duration`, clamping negatives to zero.

### ConnectionResponse

```rust
pub enum ConnectionResponse {
    Success(ConnectionDetails),
    Error(ConnectionError),
}
```

**Status:** proper union enum matching the `ConnectionResponse` union in the
Cap'n Proto schema (`error @0 :ConnectionError | connectionDetails @1 :ConnectionDetails`).

### RegisterConnectionRequest

```rust
pub struct RegisterConnectionRequest {
    pub auth: TunnelAuth,
    pub tunnel_id: Uuid,
    pub conn_index: u8,
    pub options: ConnectionOptions,
}
```

**Status:** maps all four `registerConnection` RPC parameters. `tunnel_id`
is a separate field, matching the Go registration client signature.

### Wire Encoding

Cap'n Proto binary codec implemented in `registration_codec.rs` with
`marshal_capnp` / `unmarshal_capnp` methods for all six schema types:
`TunnelAuth`, `ClientInfo`, `ConnectionOptions`, `ConnectionDetails`,
`ConnectionError`, `ConnectionResponse`. 17 round-trip tests including
wire serialization pass. Codec matches Go `pogs` field mapping.

Runtime integration is live: `lifecycle.rs` now calls
`encode_registration_request()` for the outbound register path and
`decode_registration_response()` for the inbound response path on the QUIC
control stream. The earlier JSON control-stream path is gone.

QUIC connection ALPN `argotunnel` is correctly set in Rust
([crates/cfdrs-bin/src/transport/quic/session.rs](../../../crates/cfdrs-bin/src/transport/quic/session.rs)
`EDGE_QUIC_ALPN`) — matches Go.

## Gap Summary

| Gap | Severity | Detail |
| --- | --- | --- |
| origin-cert registration content remains bounded | medium | `build_registration_request()` only emits live registration auth for the admitted credential-file-backed path |
| no `connectedFuse`-style readiness signal | low | runtime uses bridge/status machinery instead of Go's channel-style fuse |
| capnp-rpc client/server wrappers are not the live transport path | low | runtime uses raw `capnp::serialize` on the control stream; `rpc_dispatch.rs` remains the local-dispatch and future-maintenance surface |
