//! QUIC datagram wire contracts for UDP session proxying (CDC-040, CDC-041).
//!
//! ## V2 (CDC-040)
//!
//! V2 sessions use RPC-based registration through the `SessionManager`
//! capnp interface. The datagram muxer carries session payloads keyed by
//! UUID session IDs.
//!
//! See `baseline-2026.2.0/datagramsession/manager.go` and
//! `baseline-2026.2.0/connection/quic_datagram_v2.go`.
//!
//! ## V3 (CDC-041)
//!
//! V3 replaces RPC-based registration with inline binary datagram framing.
//! Each QUIC datagram carries a 1-byte type discriminator followed by
//! type-specific fields.
//!
//! See `baseline-2026.2.0/quic/v3/` and
//! `baseline-2026.2.0/connection/quic_datagram_v3.go`.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

// ---------------------------------------------------------------------------
// V2 session constants (CDC-040)
// ---------------------------------------------------------------------------

/// Channel capacity for incoming datagram session requests.
///
/// Matches `requestChanCapacity` in `datagramsession/manager.go`.
pub const V2_REQUEST_CHAN_CAPACITY: usize = 16;

/// Default timeout for session registration requests.
///
/// Matches `defaultReqTimeout` in `datagramsession/manager.go`.
pub const V2_DEFAULT_REQ_TIMEOUT: Duration = Duration::from_secs(5);

/// A V2 datagram payload bound to a session.
///
/// Matches `packet.Session` in `datagramsession/manager.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatagramSessionPayload {
    pub id: uuid::Uuid,
    pub payload: Vec<u8>,
}

/// Format a UUID as a dash-free hex string for V2 session logging.
///
/// Matches `FormatSessionID()` in `datagramsession/manager.go`.
pub fn format_session_id(id: &uuid::Uuid) -> String {
    id.as_simple().to_string()
}

// ---------------------------------------------------------------------------
// V3 wire constants (CDC-041)
// ---------------------------------------------------------------------------

/// Maximum QUIC datagram payload length (bytes).
///
/// Matches `maxDatagramPayloadLen` in `quic/v3/datagram.go`.
pub const MAX_DATAGRAM_PAYLOAD_LEN: usize = 1280;

/// Length of the type discriminator field.
pub const DATAGRAM_TYPE_LEN: usize = 1;

/// Length of a `RequestID` on the wire.
pub const REQUEST_ID_LEN: usize = 16;

/// Header length for a UDP session payload datagram: type(1) + request_id(16).
///
/// Matches `DatagramPayloadHeaderLen` in `quic/v3/datagram.go`.
pub const DATAGRAM_PAYLOAD_HEADER_LEN: usize = DATAGRAM_TYPE_LEN + REQUEST_ID_LEN;

/// IPv4 registration header length: type(1) + flags(1) + port(2) + idle(2) +
/// request_id(16) + ipv4(4).
///
/// Matches `sessionRegistrationIPv4DatagramHeaderLen` in `quic/v3/datagram.go`.
pub const SESSION_REGISTRATION_IPV4_HEADER_LEN: usize = 26;

/// IPv6 registration header length: type(1) + flags(1) + port(2) + idle(2) +
/// request_id(16) + ipv6(16).
///
/// Matches `sessionRegistrationIPv6DatagramHeaderLen` in `quic/v3/datagram.go`.
pub const SESSION_REGISTRATION_IPV6_HEADER_LEN: usize = 38;

/// Maximum ICMP payload length.
///
/// Matches `maxICMPPayloadLen` in `quic/v3/datagram.go`.
pub const MAX_ICMP_PAYLOAD_LEN: usize = 1280;

/// Default idle timeout for a V3 session.
///
/// Matches `defaultCloseIdleAfter` in `quic/v3/session.go`.
pub const V3_DEFAULT_CLOSE_IDLE_AFTER: Duration = Duration::from_secs(210);

/// Write channel capacity for V3 sessions.
///
/// Matches `writeChanCapacity` in `quic/v3/session.go`.
pub const V3_WRITE_CHAN_CAPACITY: usize = 512;

/// Maximum origin UDP packet size accepted by V3 sessions.
///
/// Matches `maxOriginUDPPacketSize` in `quic/v3/session.go`.
pub const V3_MAX_ORIGIN_UDP_PACKET_SIZE: usize = 1500;

/// Maximum error message length in a registration response datagram.
///
/// Matches `maxResponseErrorMessageLen` in `quic/v3/datagram.go`:
/// `MAX_DATAGRAM_PAYLOAD_LEN - registration_response_header_len`.
pub const MAX_RESPONSE_ERROR_MESSAGE_LEN: usize = MAX_DATAGRAM_PAYLOAD_LEN - 20;

// ---------------------------------------------------------------------------
// V3 type discriminator
// ---------------------------------------------------------------------------

/// QUIC datagram type discriminator byte (V3 wire format).
///
/// Matches `DatagramType` in `quic/v3/datagram.go`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DatagramType {
    /// Edge → cloudflared: register a new UDP session.
    UdpSessionRegistration = 0x0,
    /// Bidirectional: UDP session payload.
    UdpSessionPayload = 0x1,
    /// ICMP packet relay.
    Icmp = 0x2,
    /// cloudflared → edge: registration response.
    UdpSessionRegistrationResponse = 0x3,
}

impl DatagramType {
    /// Parse from a wire byte.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(Self::UdpSessionRegistration),
            0x1 => Some(Self::UdpSessionPayload),
            0x2 => Some(Self::Icmp),
            0x3 => Some(Self::UdpSessionRegistrationResponse),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// V3 registration flags
// ---------------------------------------------------------------------------

/// IPv6 address flag in the registration datagram.
///
/// When set, the destination address is 16-byte IPv6 instead of 4-byte IPv4.
pub const FLAG_IP_V6: u8 = 0b0000_0001;

/// Tracing enabled flag.
pub const FLAG_TRACED: u8 = 0b0000_0010;

/// Bundled first-packet flag.
///
/// When set, the datagram includes an initial UDP payload after the header.
pub const FLAG_BUNDLED: u8 = 0b0000_0100;

// ---------------------------------------------------------------------------
// V3 registration response status
// ---------------------------------------------------------------------------

/// Session registration response status code.
///
/// Matches `SessionRegistrationResp` in `quic/v3/datagram.go`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SessionRegistrationResp {
    Ok = 0x00,
    DestinationUnreachable = 0x01,
    UnableToBindSocket = 0x02,
    TooManyActiveFlows = 0x03,
    ErrorWithMsg = 0xff,
}

impl SessionRegistrationResp {
    /// Parse from a wire byte.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::Ok),
            0x01 => Some(Self::DestinationUnreachable),
            0x02 => Some(Self::UnableToBindSocket),
            0x03 => Some(Self::TooManyActiveFlows),
            0xff => Some(Self::ErrorWithMsg),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// V3 RequestID (128-bit)
// ---------------------------------------------------------------------------

/// 128-bit session identifier for V3 datagrams.
///
/// Stored as big-endian `(hi, lo)` pair and serialized as 16 raw bytes on
/// the wire. Displayed as 32 lowercase hex digits.
///
/// Matches `RequestID` / `uint128` in `quic/v3/request_id.go`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId {
    hi: u64,
    lo: u64,
}

impl RequestId {
    /// Construct from high and low 64-bit halves (big-endian convention).
    pub const fn new(hi: u64, lo: u64) -> Self {
        Self { hi, lo }
    }

    /// Construct from a 128-bit integer.
    pub const fn from_u128(value: u128) -> Self {
        Self {
            hi: (value >> 64) as u64,
            lo: value as u64,
        }
    }

    /// Convert to a 128-bit integer.
    pub const fn to_u128(self) -> u128 {
        ((self.hi as u128) << 64) | (self.lo as u128)
    }

    /// Read from a 16-byte big-endian slice.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < REQUEST_ID_LEN {
            return None;
        }

        let hi = u64::from_be_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);

        let lo = u64::from_be_bytes([
            data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
        ]);

        Some(Self { hi, lo })
    }

    /// Write to a 16-byte big-endian buffer.
    pub fn write_to(&self, buf: &mut [u8]) -> bool {
        if buf.len() < REQUEST_ID_LEN {
            return false;
        }
        buf[..8].copy_from_slice(&self.hi.to_be_bytes());
        buf[8..16].copy_from_slice(&self.lo.to_be_bytes());
        true
    }

    /// Serialize to a 16-byte big-endian array.
    pub fn to_bytes(self) -> [u8; REQUEST_ID_LEN] {
        let mut buf = [0u8; REQUEST_ID_LEN];
        buf[..8].copy_from_slice(&self.hi.to_be_bytes());
        buf[8..16].copy_from_slice(&self.lo.to_be_bytes());
        buf
    }
}

impl std::fmt::Debug for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RequestId({self})")
    }
}

impl std::fmt::Display for RequestId {
    /// 32-char lowercase hex, matching Go's `%016x%016x` format.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}{:016x}", self.hi, self.lo)
    }
}

// ---------------------------------------------------------------------------
// V3 datagram structs
// ---------------------------------------------------------------------------

/// Registration datagram: edge → cloudflared (type 0x0).
///
/// Matches `UDPSessionRegistrationDatagram` in `quic/v3/datagram.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpSessionRegistrationDatagram {
    pub request_id: RequestId,
    pub dest: SocketAddr,
    pub traced: bool,
    pub idle_duration_hint: Duration,
    pub payload: Vec<u8>,
}

/// Payload datagram: bidirectional (type 0x1).
///
/// Matches `UDPSessionPayloadDatagram` in `quic/v3/datagram.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpSessionPayloadDatagram {
    pub request_id: RequestId,
    pub payload: Vec<u8>,
}

/// Registration response datagram: cloudflared → edge (type 0x3).
///
/// Matches `UDPSessionRegistrationResponseDatagram` in `quic/v3/datagram.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpSessionRegistrationResponseDatagram {
    pub request_id: RequestId,
    pub response_type: SessionRegistrationResp,
    pub error_msg: String,
}

/// ICMP datagram: bidirectional (type 0x2).
///
/// Matches `ICMPDatagram` in `quic/v3/datagram.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcmpDatagram {
    pub payload: Vec<u8>,
}

// ---------------------------------------------------------------------------
// V3 wire encoding
// ---------------------------------------------------------------------------

impl UdpSessionRegistrationDatagram {
    /// Encode to bytes matching the Go V3 wire format.
    pub fn marshal(&self) -> Vec<u8> {
        let is_ipv6 = self.dest.ip().is_ipv6();

        let header_len = if is_ipv6 {
            SESSION_REGISTRATION_IPV6_HEADER_LEN
        } else {
            SESSION_REGISTRATION_IPV4_HEADER_LEN
        };

        let mut buf = Vec::with_capacity(header_len + self.payload.len());

        // Type byte
        buf.push(DatagramType::UdpSessionRegistration as u8);

        // Flags
        let mut flags: u8 = 0;

        if is_ipv6 {
            flags |= FLAG_IP_V6;
        }

        if self.traced {
            flags |= FLAG_TRACED;
        }

        if !self.payload.is_empty() {
            flags |= FLAG_BUNDLED;
        }

        buf.push(flags);

        // Destination port (big-endian)
        buf.extend_from_slice(&self.dest.port().to_be_bytes());

        // Idle duration hint (seconds, u16 big-endian)
        let idle_secs = self.idle_duration_hint.as_secs().min(u16::MAX as u64) as u16;
        buf.extend_from_slice(&idle_secs.to_be_bytes());

        // Request ID (16 bytes)
        buf.extend_from_slice(&self.request_id.to_bytes());

        // Destination IP
        match self.dest.ip() {
            IpAddr::V4(ip) => buf.extend_from_slice(&ip.octets()),
            IpAddr::V6(ip) => buf.extend_from_slice(&ip.octets()),
        }

        // Optional payload
        buf.extend_from_slice(&self.payload);

        buf
    }

    /// Decode from bytes matching the Go V3 wire format.
    pub fn unmarshal(data: &[u8]) -> Option<Self> {
        if data.len() < SESSION_REGISTRATION_IPV4_HEADER_LEN {
            return None;
        }

        let typ = DatagramType::from_u8(data[0])?;

        if typ != DatagramType::UdpSessionRegistration {
            return None;
        }

        let flags = data[1];
        let is_ipv6 = flags & FLAG_IP_V6 != 0;
        let traced = flags & FLAG_TRACED != 0;
        let bundled = flags & FLAG_BUNDLED != 0;

        let expected_header = if is_ipv6 {
            SESSION_REGISTRATION_IPV6_HEADER_LEN
        } else {
            SESSION_REGISTRATION_IPV4_HEADER_LEN
        };

        if data.len() < expected_header {
            return None;
        }

        let dest_port = u16::from_be_bytes([data[2], data[3]]);
        let idle_secs = u16::from_be_bytes([data[4], data[5]]);
        let request_id = RequestId::from_bytes(&data[6..22])?;

        let (ip, payload_start) = if is_ipv6 {
            let octets: [u8; 16] = data[22..38].try_into().ok()?;
            (IpAddr::V6(Ipv6Addr::from(octets)), 38)
        } else {
            let octets: [u8; 4] = data[22..26].try_into().ok()?;
            (IpAddr::V4(Ipv4Addr::from(octets)), 26)
        };

        let payload = if bundled && data.len() > payload_start {
            data[payload_start..].to_vec()
        } else {
            Vec::new()
        };

        Some(Self {
            request_id,
            dest: SocketAddr::new(ip, dest_port),
            traced,
            idle_duration_hint: Duration::from_secs(idle_secs as u64),
            payload,
        })
    }
}

impl UdpSessionPayloadDatagram {
    /// Encode to bytes.
    pub fn marshal(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(DATAGRAM_PAYLOAD_HEADER_LEN + self.payload.len());
        buf.push(DatagramType::UdpSessionPayload as u8);
        buf.extend_from_slice(&self.request_id.to_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Decode from bytes.
    pub fn unmarshal(data: &[u8]) -> Option<Self> {
        if data.len() < DATAGRAM_PAYLOAD_HEADER_LEN {
            return None;
        }

        let typ = DatagramType::from_u8(data[0])?;

        if typ != DatagramType::UdpSessionPayload {
            return None;
        }

        let request_id = RequestId::from_bytes(&data[1..17])?;
        let payload = data[17..].to_vec();

        Some(Self { request_id, payload })
    }
}

impl UdpSessionRegistrationResponseDatagram {
    /// Response header length: type(1) + resp_type(1) + request_id(16) +
    /// error_len(2) = 20.
    const HEADER_LEN: usize = 20;

    /// Encode to bytes.
    pub fn marshal(&self) -> Vec<u8> {
        let msg_bytes = self.error_msg.as_bytes();
        let msg_len = msg_bytes.len().min(MAX_RESPONSE_ERROR_MESSAGE_LEN);
        let mut buf = Vec::with_capacity(Self::HEADER_LEN + msg_len);

        buf.push(DatagramType::UdpSessionRegistrationResponse as u8);
        buf.push(self.response_type as u8);
        buf.extend_from_slice(&self.request_id.to_bytes());
        buf.extend_from_slice(&(msg_len as u16).to_be_bytes());

        if msg_len > 0 {
            buf.extend_from_slice(&msg_bytes[..msg_len]);
        }

        buf
    }

    /// Decode from bytes.
    pub fn unmarshal(data: &[u8]) -> Option<Self> {
        if data.len() < Self::HEADER_LEN {
            return None;
        }

        let typ = DatagramType::from_u8(data[0])?;

        if typ != DatagramType::UdpSessionRegistrationResponse {
            return None;
        }

        let response_type = SessionRegistrationResp::from_u8(data[1])?;
        let request_id = RequestId::from_bytes(&data[2..18])?;
        let error_len = u16::from_be_bytes([data[18], data[19]]) as usize;

        if error_len > MAX_RESPONSE_ERROR_MESSAGE_LEN {
            return None;
        }

        let error_msg = if error_len > 0 {
            if data.len() < Self::HEADER_LEN + error_len {
                return None;
            }
            String::from_utf8(data[Self::HEADER_LEN..Self::HEADER_LEN + error_len].to_vec()).ok()?
        } else {
            String::new()
        };

        Some(Self {
            request_id,
            response_type,
            error_msg,
        })
    }
}

impl IcmpDatagram {
    /// Encode to bytes.
    pub fn marshal(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(DATAGRAM_TYPE_LEN + self.payload.len());
        buf.push(DatagramType::Icmp as u8);
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Decode from bytes.
    pub fn unmarshal(data: &[u8]) -> Option<Self> {
        if data.len() < DATAGRAM_TYPE_LEN + 1 {
            return None;
        }

        let typ = DatagramType::from_u8(data[0])?;

        if typ != DatagramType::Icmp {
            return None;
        }

        let payload = data[DATAGRAM_TYPE_LEN..].to_vec();

        if payload.len() > MAX_ICMP_PAYLOAD_LEN {
            return None;
        }

        Some(Self { payload })
    }
}

// ---------------------------------------------------------------------------
// V3 session errors
// ---------------------------------------------------------------------------

/// V3 session manager errors.
///
/// Matches sentinel errors in `quic/v3/manager.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    NotFound,
    BoundToOtherConn,
    AlreadyRegistered,
    RegistrationRateLimited,
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => f.write_str("flow not found"),
            Self::BoundToOtherConn => f.write_str("flow is in use by another connection"),
            Self::AlreadyRegistered => f.write_str("flow is already registered for this connection"),
            Self::RegistrationRateLimited => f.write_str("flow registration rate limited"),
        }
    }
}

impl std::error::Error for SessionError {}

/// V3 session idle timeout error.
///
/// Matches `SessionIdleErr` in `quic/v3/session.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionIdleErr {
    pub timeout: Duration,
}

impl std::fmt::Display for SessionIdleErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "flow was idle for {:?}", self.timeout)
    }
}

impl std::error::Error for SessionIdleErr {}

// ---------------------------------------------------------------------------
// V3 session manager trait (CDC-041)
// ---------------------------------------------------------------------------

/// Connection identifier for associating sessions with QUIC connections.
///
/// Matches the `conn.ID()` used in Go's `SessionManager.RegisterSession`.
pub type ConnectionId = u8;

/// V3 datagram session manager trait.
///
/// Matches the `SessionManager` interface in `quic/v3/manager.go`.
/// Implementations coordinate UDP session lifecycle: registration,
/// lookup, and teardown.
pub trait SessionManager: Send + Sync {
    /// Register a new session for the given request.
    ///
    /// If the request ID already exists for a different connection,
    /// returns `SessionError::BoundToOtherConn`. If already registered
    /// for the same connection, returns `SessionError::AlreadyRegistered`.
    /// Rate limiting may return `SessionError::RegistrationRateLimited`.
    fn register_session(
        &self,
        request: &UdpSessionRegistrationDatagram,
        connection_id: ConnectionId,
    ) -> Result<RequestId, SessionError>;

    /// Look up an active session by request ID.
    ///
    /// Returns `SessionError::NotFound` if no session exists.
    fn get_session(&self, request_id: &RequestId) -> Result<ConnectionId, SessionError>;

    /// Unregister and close a session by request ID.
    ///
    /// No-op if the session does not exist.
    fn unregister_session(&self, request_id: &RequestId);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- V2 constants (CDC-040) -------------------------------------------

    #[test]
    fn v2_request_chan_capacity_matches_go() {
        assert_eq!(V2_REQUEST_CHAN_CAPACITY, 16);
    }

    #[test]
    fn v2_default_req_timeout_matches_go() {
        assert_eq!(V2_DEFAULT_REQ_TIMEOUT, Duration::from_secs(5));
    }

    #[test]
    fn format_session_id_removes_dashes() {
        let id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("valid uuid");
        let formatted = format_session_id(&id);
        assert_eq!(formatted, "550e8400e29b41d4a716446655440000");
        assert!(!formatted.contains('-'));
    }

    // -- V3 wire constants (CDC-041) --------------------------------------

    #[test]
    fn v3_constants_match_go_baseline() {
        assert_eq!(MAX_DATAGRAM_PAYLOAD_LEN, 1280);
        assert_eq!(DATAGRAM_TYPE_LEN, 1);
        assert_eq!(REQUEST_ID_LEN, 16);
        assert_eq!(DATAGRAM_PAYLOAD_HEADER_LEN, 17);
        assert_eq!(SESSION_REGISTRATION_IPV4_HEADER_LEN, 26);
        assert_eq!(SESSION_REGISTRATION_IPV6_HEADER_LEN, 38);
        assert_eq!(MAX_ICMP_PAYLOAD_LEN, 1280);
        assert_eq!(V3_DEFAULT_CLOSE_IDLE_AFTER, Duration::from_secs(210));
        assert_eq!(V3_WRITE_CHAN_CAPACITY, 512);
        assert_eq!(V3_MAX_ORIGIN_UDP_PACKET_SIZE, 1500);
        assert_eq!(MAX_RESPONSE_ERROR_MESSAGE_LEN, 1260);
    }

    #[test]
    fn datagram_type_discriminators_match_go() {
        assert_eq!(DatagramType::UdpSessionRegistration as u8, 0x0);
        assert_eq!(DatagramType::UdpSessionPayload as u8, 0x1);
        assert_eq!(DatagramType::Icmp as u8, 0x2);
        assert_eq!(DatagramType::UdpSessionRegistrationResponse as u8, 0x3);
    }

    #[test]
    fn datagram_type_round_trip() {
        for byte in 0..=0x3u8 {
            let dt = DatagramType::from_u8(byte).expect("valid type");
            assert_eq!(dt as u8, byte);
        }
        assert!(DatagramType::from_u8(0x04).is_none());
        assert!(DatagramType::from_u8(0xff).is_none());
    }

    #[test]
    fn registration_flags_match_go() {
        assert_eq!(FLAG_IP_V6, 0b0000_0001);
        assert_eq!(FLAG_TRACED, 0b0000_0010);
        assert_eq!(FLAG_BUNDLED, 0b0000_0100);
    }

    #[test]
    fn session_registration_resp_discriminators_match_go() {
        assert_eq!(SessionRegistrationResp::Ok as u8, 0x00);
        assert_eq!(SessionRegistrationResp::DestinationUnreachable as u8, 0x01);
        assert_eq!(SessionRegistrationResp::UnableToBindSocket as u8, 0x02);
        assert_eq!(SessionRegistrationResp::TooManyActiveFlows as u8, 0x03);
        assert_eq!(SessionRegistrationResp::ErrorWithMsg as u8, 0xff);
    }

    // -- RequestId --------------------------------------------------------

    #[test]
    fn request_id_bytes_round_trip() {
        let id = RequestId::new(0x0123_4567_89ab_cdef, 0xfedcba9876543210);
        let bytes = id.to_bytes();
        let recovered = RequestId::from_bytes(&bytes).expect("valid bytes");
        assert_eq!(id, recovered);
    }

    #[test]
    fn request_id_display_32_hex_digits() {
        let id = RequestId::new(0x0000_0000_0000_00ff, 0x0000_0000_0000_0001);
        let s = id.to_string();
        assert_eq!(s.len(), 32);
        assert_eq!(s, "00000000000000ff0000000000000001");
    }

    #[test]
    fn request_id_u128_round_trip() {
        let value: u128 = 0xdeadbeef_12345678_aabbccdd_eeff0011;
        let id = RequestId::from_u128(value);
        assert_eq!(id.to_u128(), value);
    }

    #[test]
    fn request_id_from_short_slice_returns_none() {
        assert!(RequestId::from_bytes(&[0u8; 15]).is_none());
        assert!(RequestId::from_bytes(&[]).is_none());
    }

    // -- Registration datagram round-trip ---------------------------------

    #[test]
    fn registration_datagram_ipv4_round_trip() {
        let dg = UdpSessionRegistrationDatagram {
            request_id: RequestId::new(1, 2),
            dest: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8080),
            traced: false,
            idle_duration_hint: Duration::from_secs(30),
            payload: Vec::new(),
        };

        let wire = dg.marshal();
        assert_eq!(wire.len(), SESSION_REGISTRATION_IPV4_HEADER_LEN);
        assert_eq!(wire[0], DatagramType::UdpSessionRegistration as u8);
        assert_eq!(wire[1] & FLAG_IP_V6, 0);

        let recovered = UdpSessionRegistrationDatagram::unmarshal(&wire).expect("valid unmarshal");
        assert_eq!(recovered, dg);
    }

    #[test]
    fn registration_datagram_ipv6_round_trip() {
        let dg = UdpSessionRegistrationDatagram {
            request_id: RequestId::new(0xff, 0xee),
            dest: SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 443),
            traced: true,
            idle_duration_hint: Duration::from_secs(60),
            payload: Vec::new(),
        };

        let wire = dg.marshal();
        assert_eq!(wire.len(), SESSION_REGISTRATION_IPV6_HEADER_LEN);
        assert!(wire[1] & FLAG_IP_V6 != 0);
        assert!(wire[1] & FLAG_TRACED != 0);

        let recovered = UdpSessionRegistrationDatagram::unmarshal(&wire).expect("valid unmarshal");
        assert_eq!(recovered, dg);
    }

    #[test]
    fn registration_datagram_bundled_payload() {
        let dg = UdpSessionRegistrationDatagram {
            request_id: RequestId::new(42, 99),
            dest: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 53),
            traced: false,
            idle_duration_hint: Duration::from_secs(5),
            payload: vec![0xde, 0xad, 0xbe, 0xef],
        };

        let wire = dg.marshal();
        assert_eq!(wire.len(), SESSION_REGISTRATION_IPV4_HEADER_LEN + 4);
        assert!(wire[1] & FLAG_BUNDLED != 0);

        let recovered = UdpSessionRegistrationDatagram::unmarshal(&wire).expect("valid unmarshal");
        assert_eq!(recovered.payload, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    // -- Payload datagram round-trip --------------------------------------

    #[test]
    fn payload_datagram_round_trip() {
        let dg = UdpSessionPayloadDatagram {
            request_id: RequestId::new(7, 8),
            payload: b"hello udp".to_vec(),
        };

        let wire = dg.marshal();
        assert_eq!(wire[0], DatagramType::UdpSessionPayload as u8);

        let recovered = UdpSessionPayloadDatagram::unmarshal(&wire).expect("valid unmarshal");
        assert_eq!(recovered, dg);
    }

    #[test]
    fn payload_datagram_too_short() {
        assert!(UdpSessionPayloadDatagram::unmarshal(&[0x1]).is_none());
        assert!(UdpSessionPayloadDatagram::unmarshal(&[]).is_none());
    }

    // -- Response datagram round-trip -------------------------------------

    #[test]
    fn response_datagram_ok_round_trip() {
        let dg = UdpSessionRegistrationResponseDatagram {
            request_id: RequestId::new(100, 200),
            response_type: SessionRegistrationResp::Ok,
            error_msg: String::new(),
        };

        let wire = dg.marshal();
        assert_eq!(wire[0], DatagramType::UdpSessionRegistrationResponse as u8);
        assert_eq!(wire[1], SessionRegistrationResp::Ok as u8);

        let recovered = UdpSessionRegistrationResponseDatagram::unmarshal(&wire).expect("valid unmarshal");
        assert_eq!(recovered, dg);
    }

    #[test]
    fn response_datagram_error_with_msg_round_trip() {
        let dg = UdpSessionRegistrationResponseDatagram {
            request_id: RequestId::new(0, 1),
            response_type: SessionRegistrationResp::ErrorWithMsg,
            error_msg: "destination unreachable: connection refused".to_string(),
        };

        let wire = dg.marshal();

        let recovered = UdpSessionRegistrationResponseDatagram::unmarshal(&wire).expect("valid unmarshal");
        assert_eq!(recovered, dg);
    }

    #[test]
    fn response_datagram_too_short() {
        assert!(UdpSessionRegistrationResponseDatagram::unmarshal(&[0x3; 19]).is_none());
    }

    // -- ICMP datagram round-trip -----------------------------------------

    #[test]
    fn icmp_datagram_round_trip() {
        let dg = IcmpDatagram {
            payload: vec![0x08, 0x00, 0x4a, 0x5c],
        };

        let wire = dg.marshal();
        assert_eq!(wire[0], DatagramType::Icmp as u8);

        let recovered = IcmpDatagram::unmarshal(&wire).expect("valid unmarshal");
        assert_eq!(recovered, dg);
    }

    #[test]
    fn icmp_datagram_empty_returns_none() {
        // ICMP needs at least 1 byte of payload after the type byte
        assert!(IcmpDatagram::unmarshal(&[DatagramType::Icmp as u8]).is_none());
    }

    #[test]
    fn icmp_datagram_oversized_returns_none() {
        let mut data = vec![DatagramType::Icmp as u8];
        data.extend(vec![0u8; MAX_ICMP_PAYLOAD_LEN + 1]);
        assert!(IcmpDatagram::unmarshal(&data).is_none());
    }

    // -- Session errors ---------------------------------------------------

    #[test]
    fn session_error_messages_match_go() {
        assert_eq!(SessionError::NotFound.to_string(), "flow not found");
        assert_eq!(
            SessionError::BoundToOtherConn.to_string(),
            "flow is in use by another connection"
        );
        assert_eq!(
            SessionError::AlreadyRegistered.to_string(),
            "flow is already registered for this connection"
        );
        assert_eq!(
            SessionError::RegistrationRateLimited.to_string(),
            "flow registration rate limited"
        );
    }

    #[test]
    fn session_idle_error_format() {
        let err = SessionIdleErr {
            timeout: Duration::from_secs(210),
        };
        let msg = err.to_string();
        assert!(msg.contains("210"), "expected timeout in message: {msg}");
        assert!(msg.contains("idle"), "expected 'idle' in message: {msg}");
    }

    // -- Wrong-type rejection ---------------------------------------------

    #[test]
    fn unmarshal_rejects_wrong_type_byte() {
        // Build a valid registration datagram then try to parse as payload
        let reg = UdpSessionRegistrationDatagram {
            request_id: RequestId::new(1, 1),
            dest: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80),
            traced: false,
            idle_duration_hint: Duration::from_secs(10),
            payload: Vec::new(),
        };
        let wire = reg.marshal();
        assert!(UdpSessionPayloadDatagram::unmarshal(&wire).is_none());
        assert!(IcmpDatagram::unmarshal(&wire).is_none());
        assert!(UdpSessionRegistrationResponseDatagram::unmarshal(&wire).is_none());
    }
}
