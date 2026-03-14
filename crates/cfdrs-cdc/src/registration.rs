//! Registration RPC types for the cloudflare tunnel protocol.
//!
//! These types model the tunnel registration handshake that happens over
//! the control stream (QUIC stream 0). The tunnel client sends auth
//! credentials and connection options; the edge returns connection details.
//!
//! Matches the behavioral contract from
//! `baseline-2026.2.0/tunnelrpc/pogs/tunnelrpc.go` and
//! `baseline-2026.2.0/connection/connection.go`.

use std::net::{IpAddr, SocketAddr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Authentication credentials for tunnel registration.
///
/// Matches Go's `TunnelAuth` from `connection/connection.go`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelAuth {
    pub account_tag: String,
    pub tunnel_secret: Vec<u8>,
    pub tunnel_id: Uuid,
}

/// Options sent with a tunnel registration request.
///
/// Matches Go's `RegistrationOptions` / `ConnectionOptions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionOptions {
    /// Client identifier string (e.g. "cloudflared/2026.2.0").
    pub client: String,
    /// Client version string.
    pub version: String,
    /// Operating system identifier.
    pub os: String,
    /// Architecture identifier.
    pub arch: String,
    /// The numerical index of this connection within the HA pool.
    pub conn_index: u8,
    /// The edge address the client is connecting to.
    pub edge_addr: SocketAddr,
    /// Number of previous connection attempts (for cold vs resumed
    /// path distinction).
    pub num_previous_attempts: u8,
    /// Origin local IP for the tunnel, if any.
    pub origin_local_ip: Option<IpAddr>,
}

impl ConnectionOptions {
    /// Build options for the current platform.
    pub fn for_current_platform(conn_index: u8, num_previous_attempts: u8, edge_addr: SocketAddr) -> Self {
        Self {
            client: String::from("cloudflared-rs"),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            os: String::from("linux"),
            arch: String::from("x86_64"),
            conn_index,
            edge_addr,
            num_previous_attempts,
            origin_local_ip: None,
        }
    }
}

/// Connection details returned by the edge after registration.
///
/// Matches Go's `ConnectionDetails` from `connection/connection.go`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionDetails {
    /// UUID assigned to this connection by the edge.
    pub uuid: Uuid,
    /// Edge location code (e.g. "SFO", "LAX").
    pub location: String,
    /// Whether the tunnel is remotely managed (dashboard-configured).
    pub is_remotely_managed: bool,
}

/// Registration request sent over the control stream.
///
/// Combines auth and options into a single message boundary for the
/// control stream handshake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterConnectionRequest {
    pub auth: TunnelAuth,
    pub options: ConnectionOptions,
}

/// Registration response received over the control stream.
///
/// Either a successful registration with connection details, or an error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterConnectionResponse {
    pub error: String,
    pub details: Option<ConnectionDetails>,
}

impl RegisterConnectionResponse {
    pub fn success(details: ConnectionDetails) -> Self {
        Self {
            error: String::new(),
            details: Some(details),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            details: None,
        }
    }

    pub fn is_ok(&self) -> bool {
        self.error.is_empty() && self.details.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_options_for_current_platform() {
        let opts = ConnectionOptions::for_current_platform(
            0,
            0,
            "127.0.0.1:7844".parse().expect("socket addr should parse"),
        );
        assert_eq!(opts.client, "cloudflared-rs");
        assert_eq!(opts.os, "linux");
        assert_eq!(opts.arch, "x86_64");
        assert_eq!(opts.conn_index, 0);
        assert_eq!(
            opts.edge_addr,
            "127.0.0.1:7844".parse().expect("socket addr should parse")
        );
    }

    #[test]
    fn register_response_success() {
        let resp = RegisterConnectionResponse::success(ConnectionDetails {
            uuid: Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid should parse"),
            location: "SFO".into(),
            is_remotely_managed: false,
        });
        assert!(resp.is_ok());
        assert_eq!(resp.details.as_ref().map(|d| d.location.as_str()), Some("SFO"));
    }

    #[test]
    fn register_response_error() {
        let resp = RegisterConnectionResponse::error("unauthorized");
        assert!(!resp.is_ok());
        assert!(resp.details.is_none());
    }
}
