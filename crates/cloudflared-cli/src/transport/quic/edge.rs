use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};

use cloudflared_config::FED_ENDPOINT;
use tokio::net::lookup_host;

use super::identity::TransportIdentity;

pub(super) const EDGE_DEFAULT_REGION: &str = "region1";
const EDGE_DEFAULT_HOST: &str = "region1.v2.argotunnel.com";
const EDGE_QUIC_PORT: u16 = 7844;

/// Peer verification state for QUIC edge connections.
///
/// Encodes the verification requirement in the type so that a verified
/// connection always carries the CA bundle path and an unverified
/// connection cannot accidentally enable verification without one.
#[derive(Debug, Clone)]
pub(super) enum PeerVerification {
    Verified {
        ca_bundle_path: PathBuf,
    },
    #[cfg_attr(not(test), allow(dead_code))]
    Unverified,
}

#[derive(Debug, Clone)]
pub(super) struct QuicEdgeTarget {
    pub(super) connect_addr: SocketAddr,
    pub(super) host_label: String,
    pub(super) server_name: String,
    pub(super) verification: PeerVerification,
}

pub(super) async fn resolve_edge_target(identity: &TransportIdentity) -> Result<QuicEdgeTarget, String> {
    let host_label = edge_host_label(identity.endpoint_hint.as_deref());
    let mut addrs = lookup_host((host_label.clone(), EDGE_QUIC_PORT))
        .await
        .map_err(|error| {
            format!("failed to resolve QUIC edge target {host_label}:{EDGE_QUIC_PORT}: {error}")
        })?;

    let connect_addr = addrs.next().ok_or_else(|| {
        format!("no socket addresses resolved for QUIC edge target {host_label}:{EDGE_QUIC_PORT}")
    })?;

    let ca_bundle_path = default_ca_bundle_path().ok_or_else(|| {
        String::from("no Linux CA bundle path found for QUIC edge certificate verification")
    })?;

    Ok(QuicEdgeTarget {
        connect_addr,
        host_label,
        server_name: "quic.cftunnel.com".to_owned(),
        verification: PeerVerification::Verified { ca_bundle_path },
    })
}

pub(super) fn edge_host_label(endpoint_hint: Option<&str>) -> String {
    match endpoint_hint {
        Some(FED_ENDPOINT) | None => EDGE_DEFAULT_HOST.to_owned(),
        Some(region) if !region.is_empty() => format!("{region}.v2.argotunnel.com"),
        Some(_) => EDGE_DEFAULT_HOST.to_owned(),
    }
}

pub(super) fn wildcard_bind_addr(peer: SocketAddr) -> SocketAddr {
    match peer.ip() {
        IpAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        IpAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    }
}

pub(super) fn default_ca_bundle_path() -> Option<PathBuf> {
    const CANDIDATES: &[&str] = &[
        "/etc/ssl/certs/ca-certificates.crt",
        "/etc/pki/tls/certs/ca-bundle.crt",
        "/etc/ssl/cert.pem",
    ];

    CANDIDATES
        .iter()
        .map(Path::new)
        .find(|path| path.exists())
        .map(Path::to_path_buf)
}
