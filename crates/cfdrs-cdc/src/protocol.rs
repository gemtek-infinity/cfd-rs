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
// Edge discovery DNS (CDC-022)
// ---------------------------------------------------------------------------

/// SRV record name for edge discovery (region 1).
pub const EDGE_SRV_REGION1: &str = "_v2-origintunneld._tcp.argotunnel.com";

/// DNS-over-TLS resolver address used for edge discovery fallback.
pub const DOT_RESOLVER_ADDR: &str = "1.1.1.1:853";

/// DNS-over-TLS server name for certificate validation.
pub const DOT_SERVER_NAME: &str = "cloudflare-dns.com";

/// SRV TTL for edge connection resolution caching.
pub const RESOLVE_TTL_SECS: u64 = 3600;

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
}
