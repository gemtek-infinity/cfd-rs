# CDC Feature Group: Registration RPC

## Scope

This document covers the registration RPC contract between cloudflared and the
Cloudflare edge, as defined by the frozen Go baseline Cap'n Proto schema and
the Go registration client/server implementation.

Registration is the control-plane handshake that establishes a tunnel
connection with the edge.

## Frozen Baseline Schema

Source: `baseline-2026.2.0/old-impl/tunnelrpc/proto/tunnelrpc.capnp`

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

Source: `baseline-2026.2.0/old-impl/tunnelrpc/registration_client.go`

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

Source: `baseline-2026.2.0/old-impl/connection/control.go` (lines 78-147)
and `baseline-2026.2.0/old-impl/connection/quic_connection.go` (line 89)

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

Source: `crates/cfdrs-bin/src/transport/quic/lifecycle.rs`

The Rust control stream follows a simplified version of this lifecycle:

- opens first QUIC stream (bidirectional)
- sends registration request as JSON
- reads registration response as JSON
- emits `ProtocolEvent::Registered` and `ProtocolEvent::RegistrationComplete`
- then enters the stream-accept loop

Missing lifecycle stages in Rust:

- no local config push (step 4)
- no graceful shutdown / unregister RPC (steps 5-6)
- no `connectedFuse` / fuse-based readiness signaling

## Current Rust Registration Surface

Source: `crates/cfdrs-cdc/src/registration.rs`

### TunnelAuth

```rust
pub struct TunnelAuth {
    pub account_tag: String,
    pub tunnel_secret: Vec<u8>,
    pub tunnel_id: Uuid,    // extra field vs Go
}
```

**Divergence:** Rust adds `tunnel_id` into `TunnelAuth`. Go passes
`tunnelId` as a separate `registerConnection` parameter.

### ConnectionOptions

```rust
pub struct ConnectionOptions {
    pub client: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub conn_index: u8,
    pub edge_addr: SocketAddr,
    pub num_previous_attempts: u8,
    pub origin_local_ip: Option<IpAddr>,
}
```

**Divergences from Go:**

- Go nests `ClientInfo` (with `clientId`, `features`, `version`, `arch`)
  inside `ConnectionOptions`; Rust flattens these fields
- Missing from Rust: `clientId` (UUID), `features` (capability list),
  `replaceExisting`, `compressionQuality`
- Extra in Rust: `edge_addr` (not a Cap'n Proto field; used locally)

### ConnectionDetails

```rust
pub struct ConnectionDetails {
    pub uuid: Uuid,
    pub location: String,
    pub is_remotely_managed: bool,
}
```

**Status:** field-level match (naming differs: Go `locationName` vs Rust
`location`, Go `tunnelIsRemotelyManaged` vs Rust `is_remotely_managed`)

### RegisterConnectionRequest / RegisterConnectionResponse

```rust
pub struct RegisterConnectionRequest {
    pub auth: TunnelAuth,
    pub options: ConnectionOptions,
}

pub struct RegisterConnectionResponse {
    pub error: String,
    pub details: Option<ConnectionDetails>,
}
```

**Divergence:** Go uses a `ConnectionResponse` union
(`error | connectionDetails`); Rust uses a flat struct with optional details.
Go also has `ConnectionError` with `retryAfter` and `shouldRetry`; Rust has
only `error: String`.

### Wire Encoding

- Rust serializes registration request as **JSON** via `serde_json`
- Go serializes registration as **Cap'n Proto binary**
- This is the primary wire encoding divergence for the registration path
- QUIC connection ALPN `argotunnel` is correctly set in Rust (`session.rs`
  `EDGE_QUIC_ALPN`) — matches Go

## Gap Summary

| Gap | Severity | Detail |
| --- | --- | --- |
| wire encoding mismatch | critical | Rust uses JSON; Go uses Cap'n Proto binary |
| missing `ClientInfo` nesting | high | Rust flattens; Go nests with separate struct |
| missing `clientId` field | high | Go sends 16-byte UUID; Rust omits |
| missing `features` field | high | Go sends capability list; Rust omits |
| missing `replaceExisting` field | medium | Go boolean for connection eviction |
| missing `compressionQuality` field | medium | Go UInt8 for stream compression |
| missing `ConnectionError` richness | high | Go has `retryAfter` + `shouldRetry`; Rust has string only |
| missing `unregisterConnection` | medium | Rust has no graceful shutdown RPC |
| missing `updateLocalConfiguration` | medium | Rust has no config push RPC |
| missing `SessionManager` interface | high | UDP session lifecycle missing |
| missing `ConfigurationManager` interface | medium | remote config update missing |
| missing feature flags | high | no capability list sent to edge |
| `tunnel_id` placement | low | Rust includes in TunnelAuth; Go passes separately |
