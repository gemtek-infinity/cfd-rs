//! Registration RPC types for the cloudflare tunnel protocol.
//!
//! These types model the tunnel registration handshake that happens over
//! the control stream (QUIC stream 0). The tunnel client sends auth
//! credentials and connection options; the edge returns connection details
//! or a retry-aware error.
//!
//! Schema truth: `baseline-2026.2.0/tunnelrpc/proto/tunnelrpc.capnp`
//! Go POGS truth: `baseline-2026.2.0/tunnelrpc/pogs/registration_server.go`

use std::net::IpAddr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::features;

// ---------------------------------------------------------------------------
// TunnelAuth — schema struct @0x9496331ab9cd463f
// ---------------------------------------------------------------------------

/// Authentication credentials for tunnel registration.
///
/// Matches `TunnelAuth` in `tunnelrpc.capnp` (accountTag, tunnelSecret).
/// `tunnel_id` is a separate RPC parameter, not part of this struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelAuth {
    pub account_tag: String,
    pub tunnel_secret: Vec<u8>,
}

// ---------------------------------------------------------------------------
// ClientInfo — schema struct @0x83ced0145b2f114b
// ---------------------------------------------------------------------------

/// Client identification sent as part of registration.
///
/// Matches `ClientInfo` in `tunnelrpc.capnp`:
/// - `clientId @0 :Data` — 16-byte UUID
/// - `features @1 :List(Text)`
/// - `version @2 :Text`
/// - `arch @3 :Text`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientInfo {
    /// 16-byte client identifier (UUID bytes).
    pub client_id: Vec<u8>,
    /// Feature flags advertised to the edge.
    pub features: Vec<String>,
    /// Client version string (e.g. "2026.2.0").
    pub version: String,
    /// Client OS and CPU info (e.g. "linux_amd64").
    pub arch: String,
}

impl ClientInfo {
    /// Build client info for the current platform and default feature set.
    pub fn for_current_platform(client_id: Uuid) -> Self {
        Self {
            client_id: client_id.as_bytes().to_vec(),
            features: features::default_feature_list(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            arch: String::from("linux_amd64"),
        }
    }
}

// ---------------------------------------------------------------------------
// ConnectionOptions — schema struct @0xb4bf9861fe035d04
// ---------------------------------------------------------------------------

/// Options sent with a tunnel registration request.
///
/// Matches `ConnectionOptions` in `tunnelrpc.capnp`:
/// - `client @0 :ClientInfo`
/// - `originLocalIp @1 :Data`
/// - `replaceExisting @2 :Bool`
/// - `compressionQuality @3 :UInt8`
/// - `numPreviousAttempts @4 :UInt8`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionOptions {
    pub client: ClientInfo,
    pub origin_local_ip: Option<IpAddr>,
    pub replace_existing: bool,
    /// Cross-stream compression setting: 0 = off, 3 = high.
    pub compression_quality: u8,
    pub num_previous_attempts: u8,
}

impl ConnectionOptions {
    /// Build options for the current platform with default values.
    pub fn for_current_platform(client_id: Uuid, num_previous_attempts: u8) -> Self {
        Self {
            client: ClientInfo::for_current_platform(client_id),
            origin_local_ip: None,
            replace_existing: false,
            compression_quality: 0,
            num_previous_attempts,
        }
    }
}

// ---------------------------------------------------------------------------
// ConnectionDetails — schema struct @0xb5f39f082b9ac18a
// ---------------------------------------------------------------------------

/// Connection details returned by the edge after successful registration.
///
/// Matches `ConnectionDetails` in `tunnelrpc.capnp`:
/// - `uuid @0 :Data` — 16-byte connection UUID
/// - `locationName @1 :Text` — colo airport code
/// - `tunnelIsRemotelyManaged @2 :Bool`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionDetails {
    pub uuid: Uuid,
    pub location: String,
    pub is_remotely_managed: bool,
}

// ---------------------------------------------------------------------------
// ConnectionError — schema struct @0xf5f383d2785edb86
// ---------------------------------------------------------------------------

/// Error returned by the edge when registration fails.
///
/// Matches `ConnectionError` in `tunnelrpc.capnp`:
/// - `cause @0 :Text`
/// - `retryAfter @1 :Int64` — retry delay in nanoseconds
/// - `shouldRetry @2 :Bool`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionError {
    pub cause: String,
    /// How long this connection should wait before retrying, in nanoseconds.
    pub retry_after_ns: i64,
    pub should_retry: bool,
}

impl ConnectionError {
    /// Retry delay as a `Duration`, clamping negative values to zero.
    pub fn retry_after(&self) -> Duration {
        if self.retry_after_ns > 0 {
            Duration::from_nanos(self.retry_after_ns as u64)
        } else {
            Duration::ZERO
        }
    }
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.cause)
    }
}

impl std::error::Error for ConnectionError {}

// ---------------------------------------------------------------------------
// ConnectionResponse — schema struct @0xdbaa9d03d52b62dc (union)
// ---------------------------------------------------------------------------

/// Registration response from the edge, as a proper union.
///
/// Matches the `result :union` in `ConnectionResponse` from `tunnelrpc.capnp`:
/// - `error @0 :ConnectionError`
/// - `connectionDetails @1 :ConnectionDetails`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionResponse {
    Success(ConnectionDetails),
    Error(ConnectionError),
}

impl ConnectionResponse {
    /// Convenience constructor for a successful response.
    pub fn success(details: ConnectionDetails) -> Self {
        Self::Success(details)
    }

    /// Convenience constructor for a non-retryable error.
    pub fn error(cause: impl Into<String>) -> Self {
        Self::Error(ConnectionError {
            cause: cause.into(),
            retry_after_ns: 0,
            should_retry: false,
        })
    }

    /// Whether the response indicates a successful registration.
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Extract connection details if successful.
    pub fn details(&self) -> Option<&ConnectionDetails> {
        match self {
            Self::Success(details) => Some(details),
            Self::Error(_) => None,
        }
    }

    /// Extract the error if registration failed.
    pub fn connection_error(&self) -> Option<&ConnectionError> {
        match self {
            Self::Error(err) => Some(err),
            Self::Success(_) => None,
        }
    }
}

// ---------------------------------------------------------------------------
// RegisterConnectionRequest — aggregated RPC parameters
// ---------------------------------------------------------------------------

/// Registration request sent over the control stream.
///
/// Combines the separate RPC parameters from
/// `RegistrationServer.registerConnection(auth, tunnelId, connIndex, options)`
/// into a single message boundary for the control stream handshake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterConnectionRequest {
    pub auth: TunnelAuth,
    pub tunnel_id: Uuid,
    pub conn_index: u8,
    pub options: ConnectionOptions,
}

// ---------------------------------------------------------------------------
// UnregisterConnection — CDC-007
// ---------------------------------------------------------------------------

/// Marker for the `RegistrationServer.unregisterConnection()` RPC.
///
/// The schema defines this as `unregisterConnection @1 () -> ()` — zero
/// parameters, void return.  The Go server handler simply ACKs the call
/// and invokes `impl.UnregisterConnection(ctx)` to perform graceful
/// disconnect cleanup.  No request or response payload is exchanged.
///
/// This type exists so the control-stream lifecycle code can name the
/// operation explicitly rather than treating it as an anonymous void call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnregisterConnectionRequest;

// ---------------------------------------------------------------------------
// UpdateLocalConfiguration — CDC-008
// ---------------------------------------------------------------------------

/// Request to push the current local configuration to the edge.
///
/// Matches `RegistrationServer.updateLocalConfiguration(config: Data)`
/// in `tunnelrpc.capnp`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateLocalConfigurationRequest {
    /// Opaque configuration payload (typically serialized JSON).
    pub config: Vec<u8>,
}

// ---------------------------------------------------------------------------
// RegisterUdpSessionResponse — schema struct @0xab6d5210c1f26687; CDC-009
// ---------------------------------------------------------------------------

/// Response from a UDP session registration RPC.
///
/// Matches `RegisterUdpSessionResponse` in `tunnelrpc.capnp`:
/// - `err @0 :Text`
/// - `spans @1 :Data`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterUdpSessionResponse {
    /// Error message if registration failed; empty means success.
    pub err: String,
    /// Trace spans in protobuf wire format (opaque pass-through).
    pub spans: Vec<u8>,
}

impl RegisterUdpSessionResponse {
    /// Whether the registration succeeded (no error text).
    pub fn is_ok(&self) -> bool {
        self.err.is_empty()
    }
}

/// Request to register a new UDP session with the edge.
///
/// Matches `SessionManager.registerUdpSession` parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterUdpSessionRequest {
    pub session_id: Uuid,
    /// Destination IP in raw bytes (4 for IPv4, 16 for IPv6).
    pub dst_ip: Vec<u8>,
    pub dst_port: u16,
    /// Idle timeout hint in nanoseconds.
    pub close_after_idle_hint_ns: i64,
    pub trace_context: String,
}

impl RegisterUdpSessionRequest {
    /// Idle timeout hint as a `Duration`, clamping negative values to zero.
    pub fn close_after_idle_hint(&self) -> Duration {
        if self.close_after_idle_hint_ns > 0 {
            Duration::from_nanos(self.close_after_idle_hint_ns as u64)
        } else {
            Duration::ZERO
        }
    }
}

/// Request to unregister an existing UDP session.
///
/// Matches `SessionManager.unregisterUdpSession(sessionId, message)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnregisterUdpSessionRequest {
    pub session_id: Uuid,
    pub message: String,
}

// ---------------------------------------------------------------------------
// UpdateConfigurationResponse — schema struct @0xdb58ff694ba05cf9; CDC-010
// ---------------------------------------------------------------------------

/// Response from a remote configuration update via the edge.
///
/// Matches `UpdateConfigurationResponse` in `tunnelrpc.capnp`:
/// - `latestAppliedVersion @0 :Int32`
/// - `err @1 :Text`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateConfigurationResponse {
    pub latest_applied_version: i32,
    /// Error message if the update failed; empty means success.
    pub err: String,
}

impl UpdateConfigurationResponse {
    /// Whether the update succeeded (no error text).
    pub fn is_ok(&self) -> bool {
        self.err.is_empty()
    }
}

/// Request sent by the edge to push a remote configuration update.
///
/// Matches `ConfigurationManager.updateConfiguration(version, config)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateConfigurationRequest {
    pub version: i32,
    /// Opaque configuration payload.
    pub config: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_info_for_current_platform() {
        let id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid should parse");
        let info = ClientInfo::for_current_platform(id);

        assert_eq!(info.client_id, id.as_bytes().to_vec());
        assert_eq!(info.arch, "linux_amd64");
        assert!(!info.features.is_empty());
        assert!(!info.version.is_empty());
    }

    #[test]
    fn connection_options_for_current_platform() {
        let id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid should parse");
        let opts = ConnectionOptions::for_current_platform(id, 0);

        assert_eq!(opts.client.arch, "linux_amd64");
        assert_eq!(opts.num_previous_attempts, 0);
        assert_eq!(opts.compression_quality, 0);
        assert!(!opts.replace_existing);
        assert!(opts.origin_local_ip.is_none());
    }

    #[test]
    fn connection_response_success() {
        let resp = ConnectionResponse::success(ConnectionDetails {
            uuid: Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid should parse"),
            location: "SFO".into(),
            is_remotely_managed: false,
        });

        assert!(resp.is_ok());
        assert_eq!(resp.details().map(|d| d.location.as_str()), Some("SFO"));
        assert!(resp.connection_error().is_none());
    }

    #[test]
    fn connection_response_error() {
        let resp = ConnectionResponse::error("unauthorized");

        assert!(!resp.is_ok());
        assert!(resp.details().is_none());
        let err = resp.connection_error().expect("should have error");
        assert_eq!(err.cause, "unauthorized");
        assert!(!err.should_retry);
    }

    #[test]
    fn connection_error_retry_delay() {
        let err = ConnectionError {
            cause: "overloaded".into(),
            retry_after_ns: 5_000_000_000,
            should_retry: true,
        };

        assert_eq!(err.retry_after(), Duration::from_secs(5));

        let err_no_retry = ConnectionError {
            cause: "fatal".into(),
            retry_after_ns: -1,
            should_retry: false,
        };

        assert_eq!(err_no_retry.retry_after(), Duration::ZERO);
    }

    #[test]
    fn register_request_includes_tunnel_id_and_conn_index() {
        let id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").expect("uuid should parse");
        let request = RegisterConnectionRequest {
            auth: TunnelAuth {
                account_tag: "acct".into(),
                tunnel_secret: vec![1, 2, 3],
            },
            tunnel_id: id,
            conn_index: 2,
            options: ConnectionOptions::for_current_platform(id, 0),
        };

        assert_eq!(request.tunnel_id, id);
        assert_eq!(request.conn_index, 2);
    }

    #[test]
    fn udp_session_response_success() {
        let resp = RegisterUdpSessionResponse {
            err: String::new(),
            spans: vec![0xca, 0xfe],
        };

        assert!(resp.is_ok());
    }

    #[test]
    fn udp_session_response_error() {
        let resp = RegisterUdpSessionResponse {
            err: "session limit reached".into(),
            spans: Vec::new(),
        };

        assert!(!resp.is_ok());
    }

    #[test]
    fn udp_session_request_idle_hint() {
        let req = RegisterUdpSessionRequest {
            session_id: Uuid::parse_str("33333333-3333-3333-3333-333333333333").expect("uuid should parse"),
            dst_ip: vec![10, 0, 0, 1],
            dst_port: 8080,
            close_after_idle_hint_ns: 30_000_000_000, // 30 seconds
            trace_context: String::new(),
        };

        assert_eq!(req.close_after_idle_hint(), Duration::from_secs(30));

        let negative = RegisterUdpSessionRequest {
            close_after_idle_hint_ns: -1,
            ..req
        };

        assert_eq!(negative.close_after_idle_hint(), Duration::ZERO);
    }

    #[test]
    fn update_configuration_response_success() {
        let resp = UpdateConfigurationResponse {
            latest_applied_version: 5,
            err: String::new(),
        };

        assert!(resp.is_ok());
        assert_eq!(resp.latest_applied_version, 5);
    }

    #[test]
    fn update_configuration_response_error() {
        let resp = UpdateConfigurationResponse {
            latest_applied_version: 4,
            err: "invalid config".into(),
        };

        assert!(!resp.is_ok());
    }

    /// CDC-007: `unregisterConnection @1 () -> ()` — zero params, void return.
    #[test]
    fn unregister_connection_request_is_zero_sized() {
        assert_eq!(
            std::mem::size_of::<UnregisterConnectionRequest>(),
            0,
            "marker type for void RPC must be zero-sized"
        );
        // cloneable and comparable
        let req = UnregisterConnectionRequest;
        assert_eq!(req, req.clone());
    }

    /// CDC-009: `unregisterUdpSession(sessionId, message)` preserves both
    /// parameters matching the Go two-parameter RPC shape.
    #[test]
    fn unregister_udp_session_request_preserves_message() {
        let sid = Uuid::parse_str("44444444-4444-4444-4444-444444444444").expect("uuid");
        let req = UnregisterUdpSessionRequest {
            session_id: sid,
            message: "session closed by client".into(),
        };
        assert_eq!(req.session_id, sid);
        assert_eq!(req.message, "session closed by client");

        // empty message is valid (Go allows it)
        let empty = UnregisterUdpSessionRequest {
            session_id: sid,
            message: String::new(),
        };
        assert!(empty.message.is_empty());
    }

    /// CDC-003: `retryAfter @1 :Int64` — zero nanoseconds yields
    /// `Duration::ZERO`, not an error or panic.
    #[test]
    fn connection_error_retry_after_zero_ns_is_zero_duration() {
        let err = ConnectionError {
            cause: "transient".into(),
            retry_after_ns: 0,
            should_retry: true,
        };
        assert_eq!(err.retry_after(), Duration::ZERO);
    }

    /// CDC-008: `updateLocalConfiguration(config :Data)` accepts arbitrary
    /// byte payloads — the field is opaque Data in the schema.
    #[test]
    fn update_local_configuration_request_accepts_arbitrary_bytes() {
        // Empty payload
        let empty = UpdateLocalConfigurationRequest { config: Vec::new() };
        assert!(empty.config.is_empty());

        // JSON payload (typical case)
        let json = UpdateLocalConfigurationRequest {
            config: b"{\"ingress\":[]}".to_vec(),
        };
        assert!(!json.config.is_empty());

        // Binary payload (schema allows arbitrary Data)
        let binary = UpdateLocalConfigurationRequest {
            config: vec![0x00, 0xff, 0xfe, 0xca],
        };
        assert_eq!(binary.config.len(), 4);
    }
}
