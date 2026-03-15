//! Protocol-level constants and stream demux signatures (CDC-021).
//!
//! Every QUIC data or RPC stream begins with a 6-byte protocol signature
//! followed by a 2-byte ASCII version string. The signature determines
//! whether the stream carries proxied data (`ConnectRequest`/
//! `ConnectResponse`) or Cap'n Proto RPC traffic (registration,
//! session management, configuration push).
//!
//! Schema truth: `baseline-2026.2.0/tunnelrpc/quic/protocol.go`
//! TLS truth: `baseline-2026.2.0/connection/protocol.go`

// ---------------------------------------------------------------------------
// Stream protocol signatures
// ---------------------------------------------------------------------------

/// 6-byte signature identifying a data stream (proxied request/response).
///
/// Written at the start of every QUIC data stream before the version
/// and Cap'n Proto encoded `ConnectRequest`/`ConnectResponse`.
pub const DATA_STREAM_SIGNATURE: [u8; 6] = [0x0a, 0x36, 0xcd, 0x12, 0xa1, 0x3e];

/// 6-byte signature identifying an RPC stream (registration, session, config).
///
/// Written at the start of QUIC streams that carry Cap'n Proto RPC
/// traffic for `RegistrationServer`, `SessionManager`, and
/// `ConfigurationManager` interfaces.
pub const RPC_STREAM_SIGNATURE: [u8; 6] = [0x52, 0xbb, 0x82, 0x5c, 0xdb, 0x65];

/// Stream protocol version string (ASCII `"01"`).
pub const PROTOCOL_VERSION: &[u8; 2] = b"01";

/// Total preamble length: 6-byte signature + 2-byte version.
pub const STREAM_PREAMBLE_LEN: usize = DATA_STREAM_SIGNATURE.len() + PROTOCOL_VERSION.len();

/// Stream type determined from the 6-byte protocol signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamProtocol {
    /// Proxied data stream carrying `ConnectRequest`/`ConnectResponse`.
    Data,
    /// Cap'n Proto RPC stream (registration, session, configuration).
    Rpc,
}

/// Determine the stream protocol from the first 6 bytes.
///
/// Returns `None` if the signature does not match either known protocol.
pub fn determine_protocol(signature: &[u8; 6]) -> Option<StreamProtocol> {
    if *signature == DATA_STREAM_SIGNATURE {
        Some(StreamProtocol::Data)
    } else if *signature == RPC_STREAM_SIGNATURE {
        Some(StreamProtocol::Rpc)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Edge TLS server names and ALPN (CDC-021)
// ---------------------------------------------------------------------------

/// TLS server name for HTTP/2 edge connections.
pub const EDGE_H2_TLS_SERVER_NAME: &str = "h2.cftunnel.com";

/// TLS server name for QUIC edge connections.
pub const EDGE_QUIC_TLS_SERVER_NAME: &str = "quic.cftunnel.com";

/// QUIC ALPN protocol identifier for the tunnel protocol.
///
/// Matches Go's `edgeQUICServerName` combined with NextProtos `["argotunnel"]`.
pub const EDGE_QUIC_ALPN: &str = "argotunnel";

/// User-facing description of available protocol options.
pub const AVAILABLE_PROTOCOL_FLAG_MESSAGE: &str =
    "Available protocols: 'auto' - automatically chooses the best protocol over time (the default; and also \
     the recommended one); 'quic' - based on QUIC, relying on UDP egress to Cloudflare edge; 'http2' - \
     using Go's HTTP2 library, relying on TCP egress to Cloudflare edge";

// ---------------------------------------------------------------------------
// Protocol type and TLS settings (CDC-021)
// ---------------------------------------------------------------------------

/// Edge transport protocol.
///
/// Matches Go's `Protocol` iota enum in `connection/protocol.go`.
/// The ordering follows Go: `HTTP2 = 0, QUIC = 1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    Http2,
    Quic,
}

/// Ordered list of supported protocols for remote percentage selection.
///
/// Matches Go's `ProtocolList = []Protocol{QUIC, HTTP2}`.
pub const PROTOCOL_LIST: &[Protocol] = &[Protocol::Quic, Protocol::Http2];

/// TLS settings for an edge transport protocol.
///
/// Matches Go's `TLSSettings` in `connection/protocol.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsSettings {
    pub server_name: &'static str,
    pub next_protos: &'static [&'static str],
}

impl Protocol {
    /// Return the TLS settings required to connect to the edge with this
    /// protocol.
    pub fn tls_settings(self) -> TlsSettings {
        match self {
            Protocol::Http2 => TlsSettings {
                server_name: EDGE_H2_TLS_SERVER_NAME,
                next_protos: &[],
            },
            Protocol::Quic => TlsSettings {
                server_name: EDGE_QUIC_TLS_SERVER_NAME,
                next_protos: &[EDGE_QUIC_ALPN],
            },
        }
    }

    /// Return the next protocol to try if this one fails, or `None` if there is
    /// no fallback.
    ///
    /// Matches Go's `(p Protocol) fallback() (Protocol, bool)`.
    pub fn fallback(self) -> Option<Protocol> {
        match self {
            Protocol::Quic => Some(Protocol::Http2),
            Protocol::Http2 => None,
        }
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Http2 => f.write_str("http2"),
            Protocol::Quic => f.write_str("quic"),
        }
    }
}

/// Select the current edge transport protocol and optionally fall back.
///
/// Matches Go's `ProtocolSelector` interface in `connection/protocol.go`.
pub trait ProtocolSelector {
    fn current(&self) -> Protocol;
    fn fallback(&self) -> Option<Protocol>;
}

/// A protocol selector that always returns the same protocol and never falls
/// back.
///
/// Matches Go's `staticProtocolSelector`.
pub struct StaticProtocolSelector {
    protocol: Protocol,
}

impl StaticProtocolSelector {
    pub fn new(protocol: Protocol) -> Self {
        Self { protocol }
    }
}

impl ProtocolSelector for StaticProtocolSelector {
    fn current(&self) -> Protocol {
        self.protocol
    }

    fn fallback(&self) -> Option<Protocol> {
        None
    }
}

// ---------------------------------------------------------------------------
// Connection status / events (CDC-020)
// ---------------------------------------------------------------------------

/// Status of a single edge connection.
///
/// Matches Go's `Status` iota enum in `connection/event.go`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionStatus {
    Disconnected,
    Connected,
    Reconnecting,
    SetURL,
    RegisteringTunnel,
    Unregistering,
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Disconnected => f.write_str("disconnected"),
            ConnectionStatus::Connected => f.write_str("connected"),
            ConnectionStatus::Reconnecting => f.write_str("reconnecting"),
            ConnectionStatus::SetURL => f.write_str("set_url"),
            ConnectionStatus::RegisteringTunnel => f.write_str("registering_tunnel"),
            ConnectionStatus::Unregistering => f.write_str("unregistering"),
        }
    }
}

/// Something that happened to a connection (registration, disconnection, etc.).
///
/// Matches Go's `Event` struct in `connection/event.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionEvent {
    pub index: u8,
    pub event_type: ConnectionStatus,
    pub location: String,
    pub protocol: Protocol,
    pub url: String,
    pub edge_address: Option<std::net::IpAddr>,
}

// ---------------------------------------------------------------------------
// Edge discovery DNS (CDC-022)
// ---------------------------------------------------------------------------

/// SRV service name component for edge discovery.
///
/// Matches Go's `srvService = "v2-origintunneld"`.
pub const SRV_SERVICE: &str = "v2-origintunneld";

/// SRV protocol component for edge discovery.
///
/// Matches Go's `srvProto = "tcp"`.
pub const SRV_PROTO: &str = "tcp";

/// SRV domain name for edge discovery.
///
/// Matches Go's `srvName = "argotunnel.com"`.
pub const SRV_NAME: &str = "argotunnel.com";

/// Composed SRV record name for edge discovery (global, region 1).
///
/// Built from `_{SRV_SERVICE}._{SRV_PROTO}.{SRV_NAME}`.
pub const EDGE_SRV_REGION1: &str = "_v2-origintunneld._tcp.argotunnel.com";

/// DNS-over-TLS resolver address used for edge discovery fallback.
///
/// Matches Go's `dotServerAddr = "1.1.1.1:853"`.
pub const DOT_RESOLVER_ADDR: &str = "1.1.1.1:853";

/// DNS-over-TLS server name for certificate validation.
///
/// Matches Go's `dotServerName = "cloudflare-dns.com"`.
pub const DOT_SERVER_NAME: &str = "cloudflare-dns.com";

/// DNS-over-TLS query timeout in seconds.
///
/// Matches Go's `dotTimeout = 15 * time.Second`.
pub const DOT_TIMEOUT_SECS: u64 = 15;

/// SRV TTL for edge connection resolution caching, in seconds.
///
/// Matches Go's `ResolveTTL = time.Hour` (3600 seconds).
pub const RESOLVE_TTL_SECS: u64 = 3600;

/// Timeout before failing over from primary (IPv6) to secondary (IPv4)
/// address set, in seconds.
///
/// Matches Go's `timeoutDuration = 10 * time.Minute` (600 seconds).
pub const REGION_FAILOVER_TIMEOUT_SECS: u64 = 600;

/// DNS TXT record domain for protocol percentage selection.
///
/// Matches Go's `protocolRecord = "protocol-v2.argotunnel.com"`.
pub const PROTOCOL_PERCENTAGE_RECORD: &str = "protocol-v2.argotunnel.com";

// ---------------------------------------------------------------------------
// Edge IP version (CDC-022)
// ---------------------------------------------------------------------------

/// IP version of a resolved edge address.
///
/// Matches Go's `EdgeIPVersion` in `edgediscovery/allregions/discovery.go`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeIPVersion {
    V4 = 4,
    V6 = 6,
}

impl std::fmt::Display for EdgeIPVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdgeIPVersion::V4 => f.write_str("4"),
            EdgeIPVersion::V6 => f.write_str("6"),
        }
    }
}

/// User-configured IP version preference for edge discovery.
///
/// Matches Go's `ConfigIPVersion` in `edgediscovery/allregions/discovery.go`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigIPVersion {
    /// Automatically select IPv6 with IPv4 fallback on failure.
    Auto = 2,
    /// Use IPv4 only.
    IPv4Only = 4,
    /// Use IPv6 only.
    IPv6Only = 6,
}

impl std::fmt::Display for ConfigIPVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigIPVersion::Auto => f.write_str("auto"),
            ConfigIPVersion::IPv4Only => f.write_str("4"),
            ConfigIPVersion::IPv6Only => f.write_str("6"),
        }
    }
}

/// Resolved edge address with TCP and UDP socket addresses and IP version.
///
/// Matches Go's `EdgeAddr` struct in `edgediscovery/allregions/discovery.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeAddr {
    pub tcp: std::net::SocketAddr,
    pub udp: std::net::SocketAddr,
    pub ip_version: EdgeIPVersion,
}

/// Build the SRV service name for a specific region.
///
/// Matches Go's `getRegionalServiceName(region)`:
/// - empty region → `"v2-origintunneld"` (global)
/// - `"us"` → `"us-v2-origintunneld"`
pub fn regional_service_name(region: &str) -> String {
    if region.is_empty() {
        SRV_SERVICE.to_owned()
    } else {
        format!("{region}-{SRV_SERVICE}")
    }
}

/// Build the full SRV query domain for a region.
///
/// Uses the `_{service}._{proto}.{name}` pattern.
pub fn regional_srv_domain(region: &str) -> String {
    let service = regional_service_name(region);
    format!("_{service}._{SRV_PROTO}.{SRV_NAME}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_stream_signature_matches_baseline() {
        assert_eq!(DATA_STREAM_SIGNATURE, [0x0a, 0x36, 0xcd, 0x12, 0xa1, 0x3e]);
    }

    #[test]
    fn rpc_stream_signature_matches_baseline() {
        assert_eq!(RPC_STREAM_SIGNATURE, [0x52, 0xbb, 0x82, 0x5c, 0xdb, 0x65]);
    }

    #[test]
    fn determine_protocol_demux() {
        assert_eq!(
            determine_protocol(&DATA_STREAM_SIGNATURE),
            Some(StreamProtocol::Data)
        );
        assert_eq!(
            determine_protocol(&RPC_STREAM_SIGNATURE),
            Some(StreamProtocol::Rpc)
        );
        assert_eq!(determine_protocol(&[0x00; 6]), None);
    }

    #[test]
    fn preamble_length() {
        assert_eq!(STREAM_PREAMBLE_LEN, 8);
    }

    #[test]
    fn alpn_matches_baseline() {
        assert_eq!(EDGE_QUIC_ALPN, "argotunnel");
    }

    #[test]
    fn edge_server_names_match_baseline() {
        assert_eq!(EDGE_H2_TLS_SERVER_NAME, "h2.cftunnel.com");
        assert_eq!(EDGE_QUIC_TLS_SERVER_NAME, "quic.cftunnel.com");
    }

    #[test]
    fn edge_discovery_srv_matches_baseline() {
        assert_eq!(EDGE_SRV_REGION1, "_v2-origintunneld._tcp.argotunnel.com");
    }

    #[test]
    fn srv_components_compose_to_region1() {
        let composed = format!("_{SRV_SERVICE}._{SRV_PROTO}.{SRV_NAME}");
        assert_eq!(composed, EDGE_SRV_REGION1);
    }

    #[test]
    fn dot_constants_match_baseline() {
        assert_eq!(DOT_RESOLVER_ADDR, "1.1.1.1:853");
        assert_eq!(DOT_SERVER_NAME, "cloudflare-dns.com");
        assert_eq!(DOT_TIMEOUT_SECS, 15);
    }

    #[test]
    fn resolve_ttl_matches_baseline() {
        assert_eq!(RESOLVE_TTL_SECS, 3600);
    }

    #[test]
    fn region_failover_timeout_matches_baseline() {
        assert_eq!(REGION_FAILOVER_TIMEOUT_SECS, 600);
    }

    #[test]
    fn protocol_percentage_record_matches_baseline() {
        assert_eq!(PROTOCOL_PERCENTAGE_RECORD, "protocol-v2.argotunnel.com");
    }

    #[test]
    fn regional_service_name_global() {
        assert_eq!(regional_service_name(""), "v2-origintunneld");
    }

    #[test]
    fn regional_service_name_with_region() {
        assert_eq!(regional_service_name("us"), "us-v2-origintunneld");
    }

    #[test]
    fn regional_srv_domain_global() {
        assert_eq!(regional_srv_domain(""), "_v2-origintunneld._tcp.argotunnel.com");
    }

    #[test]
    fn regional_srv_domain_with_region() {
        assert_eq!(
            regional_srv_domain("us"),
            "_us-v2-origintunneld._tcp.argotunnel.com"
        );
    }

    // --- Edge IP version / address types (CDC-022) ---

    #[test]
    fn edge_ip_version_display() {
        assert_eq!(EdgeIPVersion::V4.to_string(), "4");
        assert_eq!(EdgeIPVersion::V6.to_string(), "6");
    }

    #[test]
    fn edge_ip_version_discriminant() {
        assert_eq!(EdgeIPVersion::V4 as i8, 4);
        assert_eq!(EdgeIPVersion::V6 as i8, 6);
    }

    #[test]
    fn config_ip_version_display() {
        assert_eq!(ConfigIPVersion::Auto.to_string(), "auto");
        assert_eq!(ConfigIPVersion::IPv4Only.to_string(), "4");
        assert_eq!(ConfigIPVersion::IPv6Only.to_string(), "6");
    }

    #[test]
    fn config_ip_version_discriminant() {
        assert_eq!(ConfigIPVersion::Auto as i8, 2);
        assert_eq!(ConfigIPVersion::IPv4Only as i8, 4);
        assert_eq!(ConfigIPVersion::IPv6Only as i8, 6);
    }

    #[test]
    fn edge_addr_construction() {
        use std::net::{Ipv4Addr, SocketAddr};
        let addr = EdgeAddr {
            tcp: SocketAddr::new(Ipv4Addr::new(198, 41, 200, 1).into(), 7844),
            udp: SocketAddr::new(Ipv4Addr::new(198, 41, 200, 1).into(), 7844),
            ip_version: EdgeIPVersion::V4,
        };
        assert_eq!(addr.ip_version, EdgeIPVersion::V4);
        assert_eq!(addr.tcp.port(), 7844);
    }

    // --- Protocol type (CDC-021) ---

    #[test]
    fn protocol_display() {
        assert_eq!(Protocol::Http2.to_string(), "http2");
        assert_eq!(Protocol::Quic.to_string(), "quic");
    }

    #[test]
    fn protocol_fallback_quic_to_http2() {
        assert_eq!(Protocol::Quic.fallback(), Some(Protocol::Http2));
    }

    #[test]
    fn protocol_fallback_http2_none() {
        assert_eq!(Protocol::Http2.fallback(), None);
    }

    #[test]
    fn protocol_tls_settings_http2() {
        let s = Protocol::Http2.tls_settings();
        assert_eq!(s.server_name, "h2.cftunnel.com");
        assert!(s.next_protos.is_empty());
    }

    #[test]
    fn protocol_tls_settings_quic() {
        let s = Protocol::Quic.tls_settings();
        assert_eq!(s.server_name, "quic.cftunnel.com");
        assert_eq!(s.next_protos, &["argotunnel"]);
    }

    #[test]
    fn protocol_list_order_matches_baseline() {
        assert_eq!(PROTOCOL_LIST, &[Protocol::Quic, Protocol::Http2]);
    }

    #[test]
    fn static_protocol_selector_no_fallback() {
        let sel = StaticProtocolSelector::new(Protocol::Quic);
        assert_eq!(sel.current(), Protocol::Quic);
        assert_eq!(sel.fallback(), None);
    }

    // --- Connection status / events (CDC-020) ---

    #[test]
    fn connection_status_display() {
        assert_eq!(ConnectionStatus::Disconnected.to_string(), "disconnected");
        assert_eq!(ConnectionStatus::Connected.to_string(), "connected");
        assert_eq!(ConnectionStatus::Reconnecting.to_string(), "reconnecting");
        assert_eq!(ConnectionStatus::SetURL.to_string(), "set_url");
        assert_eq!(
            ConnectionStatus::RegisteringTunnel.to_string(),
            "registering_tunnel"
        );
        assert_eq!(ConnectionStatus::Unregistering.to_string(), "unregistering");
    }

    #[test]
    fn connection_event_construction() {
        let event = ConnectionEvent {
            index: 0,
            event_type: ConnectionStatus::Connected,
            location: "LAX".to_string(),
            protocol: Protocol::Quic,
            url: String::new(),
            edge_address: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(198, 41, 200, 1))),
        };
        assert_eq!(event.index, 0);
        assert_eq!(event.event_type, ConnectionStatus::Connected);
        assert_eq!(event.location, "LAX");
        assert_eq!(event.protocol, Protocol::Quic);
        assert!(event.edge_address.is_some());
    }
}
