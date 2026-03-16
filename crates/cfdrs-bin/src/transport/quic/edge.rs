use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use cfdrs_cdc::edge::Regions;
use cfdrs_cdc::protocol::{
    ConfigIPVersion, DOT_RESOLVER_ADDR, DOT_SERVER_NAME, DOT_TIMEOUT_SECS, EDGE_QUIC_TLS_SERVER_NAME,
    EdgeAddr, EdgeIPVersion, SRV_NAME, SRV_PROTO, regional_service_name,
};
use cfdrs_shared::FED_ENDPOINT;
use hickory_resolver::Resolver;
use hickory_resolver::config::{NameServerConfig, ResolverConfig, ResolverOpts};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::proto::xfer::Protocol as DnsProtocol;

use super::identity::TransportIdentity;

pub(super) const EDGE_DEFAULT_REGION: &str = "region1";
const EDGE_DEFAULT_HOST: &str = "region1.v2.argotunnel.com";

/// Peer verification state for QUIC edge connections.
///
/// Encodes the verification requirement in the type so that a verified
/// connection always carries the CA bundle path and an unverified
/// connection cannot accidentally enable verification without one.
#[derive(Debug, Clone)]
pub(in crate::transport) enum PeerVerification {
    Verified {
        ca_bundle_path: PathBuf,
    },
    #[cfg_attr(not(test), allow(dead_code))]
    Unverified,
}

#[derive(Debug, Clone)]
pub(crate) struct QuicEdgeTarget {
    pub(in crate::transport) connect_addr: SocketAddr,
    pub(in crate::transport) host_label: String,
    pub(in crate::transport) server_name: String,
    pub(in crate::transport) verification: PeerVerification,
}

/// Resolve the edge target for a QUIC connection via SRV discovery.
///
/// 1. SRV lookup (system resolver, falling back to DNS-over-TLS)
/// 2. Build `Regions` from resolved per-CNAME address lists
/// 3. Pick an unused address from the balanced region pool
///
/// Matches Go's `ResolveEdge()` → `edgeDiscovery()` flow in
/// `edgediscovery/allregions/`.
pub(super) async fn resolve_edge_target(identity: &TransportIdentity) -> Result<QuicEdgeTarget, String> {
    let region = edge_region(identity.endpoint_hint.as_deref());
    let host_label = edge_host_label(identity.endpoint_hint.as_deref());

    let region_addrs = edge_discovery(&region).await?;

    let mut regions = Regions::from_resolved(&region_addrs, ConfigIPVersion::Auto)?;

    let edge_addr = regions
        .get_unused_addr(None, 0)
        .ok_or_else(|| "edge discovery: no available addresses after SRV resolution".to_owned())?;

    let ca_bundle_path = default_ca_bundle_path().ok_or_else(|| {
        String::from("no Linux CA bundle path found for QUIC edge certificate verification")
    })?;

    Ok(QuicEdgeTarget {
        connect_addr: edge_addr.udp,
        host_label,
        server_name: EDGE_QUIC_TLS_SERVER_NAME.to_owned(),
        verification: PeerVerification::Verified { ca_bundle_path },
    })
}

/// Determine the SRV region prefix from an endpoint hint.
///
/// Returns `""` for global (default or FedRAMP), which maps to the bare
/// `"v2-origintunneld"` service name. Any other non-empty value is used
/// as the region prefix (e.g. `"us"` → `"us-v2-origintunneld"`).
fn edge_region(endpoint_hint: Option<&str>) -> String {
    match endpoint_hint {
        Some(FED_ENDPOINT) | None => String::new(),
        Some(region) if !region.is_empty() => region.to_owned(),
        Some(_) => String::new(),
    }
}

pub(super) fn edge_host_label(endpoint_hint: Option<&str>) -> String {
    match endpoint_hint {
        Some(FED_ENDPOINT) | None => EDGE_DEFAULT_HOST.to_owned(),
        Some(region) if !region.is_empty() => format!("{region}.v2.argotunnel.com"),
        Some(_) => EDGE_DEFAULT_HOST.to_owned(),
    }
}

// ---------------------------------------------------------------------------
// SRV discovery (CDC-022)
//
// Matches Go's `edgeDiscovery()` in
// `edgediscovery/allregions/discovery.go`.
// ---------------------------------------------------------------------------

/// Perform SRV-based edge discovery for the given region.
///
/// 1. SRV lookup via system resolver
/// 2. On failure, fallback to DNS-over-TLS (1.1.1.1:853)
/// 3. For each SRV target, resolve A/AAAA records into `EdgeAddr`
///
/// Returns per-CNAME address lists — one `Vec<EdgeAddr>` per SRV target
/// (each SRV target is a Cloudflare edge region).
async fn edge_discovery(region: &str) -> Result<Vec<Vec<EdgeAddr>>, String> {
    let service = regional_service_name(region);

    let srv_records = match srv_lookup_system(&service).await {
        Ok(records) if !records.is_empty() => records,
        _ => srv_lookup_dot(&service).await.map_err(|_| {
            format!("edge discovery: could not lookup SRV records on _{service}._{SRV_PROTO}.{SRV_NAME}")
        })?,
    };

    if srv_records.is_empty() {
        return Err(format!(
            "edge discovery: SRV lookup returned no records for _{service}._{SRV_PROTO}.{SRV_NAME}"
        ));
    }

    let mut per_cname = Vec::with_capacity(srv_records.len());

    for (target, port) in &srv_records {
        let addrs = resolve_srv_target(target, *port).await?;
        per_cname.push(addrs);
    }

    Ok(per_cname)
}

/// SRV lookup using the system DNS resolver.
async fn srv_lookup_system(service: &str) -> Result<Vec<(String, u16)>, String> {
    let resolver = Resolver::builder_tokio()
        .map_err(|e| format!("failed to create system resolver: {e}"))?
        .build();

    let query = format!("_{service}._{SRV_PROTO}.{SRV_NAME}.");

    let lookup = resolver
        .srv_lookup(query.as_str())
        .await
        .map_err(|e| e.to_string())?;

    Ok(lookup
        .iter()
        .map(|srv| (srv.target().to_string(), srv.port()))
        .collect())
}

/// SRV lookup using DNS-over-TLS to 1.1.1.1:853.
///
/// Matches Go's `lookupSRVWithDOT()`.
async fn srv_lookup_dot(service: &str) -> Result<Vec<(String, u16)>, String> {
    let dot_addr: SocketAddr = DOT_RESOLVER_ADDR
        .parse()
        .map_err(|e| format!("invalid DoT resolver address: {e}"))?;

    let mut ns = NameServerConfig::new(dot_addr, DnsProtocol::Tls);
    ns.tls_dns_name = Some(DOT_SERVER_NAME.to_owned());

    let config = ResolverConfig::from_parts(None, vec![], vec![ns]);
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_secs(DOT_TIMEOUT_SECS);

    let resolver = Resolver::builder_with_config(config, TokioConnectionProvider::default())
        .with_options(opts)
        .build();

    let query = format!("_{service}._{SRV_PROTO}.{SRV_NAME}.");

    let lookup = resolver
        .srv_lookup(query.as_str())
        .await
        .map_err(|e| e.to_string())?;

    Ok(lookup
        .iter()
        .map(|srv| (srv.target().to_string(), srv.port()))
        .collect())
}

/// Resolve a single SRV target hostname to `EdgeAddr` entries.
///
/// Matches Go's `resolveSRV(srv)`.
async fn resolve_srv_target(target: &str, port: u16) -> Result<Vec<EdgeAddr>, String> {
    let resolver = Resolver::builder_tokio()
        .map_err(|e| format!("failed to create resolver for SRV target {target}: {e}"))?
        .build();

    let lookup = resolver
        .lookup_ip(target)
        .await
        .map_err(|e| format!("couldn't resolve SRV target {target}: {e}"))?;

    let ips: Vec<IpAddr> = lookup.iter().collect();

    if ips.is_empty() {
        return Err(format!("SRV target {target} had no IPs"));
    }

    Ok(ips
        .iter()
        .map(|ip| {
            let ip_version = if ip.is_ipv4() {
                EdgeIPVersion::V4
            } else {
                EdgeIPVersion::V6
            };

            EdgeAddr {
                tcp: SocketAddr::new(*ip, port),
                udp: SocketAddr::new(*ip, port),
                ip_version,
            }
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_region_none_returns_global() {
        assert_eq!(edge_region(None), "");
    }

    #[test]
    fn edge_region_fed_returns_global() {
        assert_eq!(edge_region(Some(FED_ENDPOINT)), "");
    }

    #[test]
    fn edge_region_empty_returns_global() {
        assert_eq!(edge_region(Some("")), "");
    }

    #[test]
    fn edge_region_with_value_returns_region() {
        assert_eq!(edge_region(Some("us")), "us");
        assert_eq!(edge_region(Some("region1")), "region1");
    }

    #[test]
    fn edge_region_maps_to_correct_srv_service() {
        // Global (default)
        let service = regional_service_name(&edge_region(None));
        assert_eq!(service, "v2-origintunneld");

        // Regionalized
        let service = regional_service_name(&edge_region(Some("us")));
        assert_eq!(service, "us-v2-origintunneld");

        // FedRAMP treated as global
        let service = regional_service_name(&edge_region(Some(FED_ENDPOINT)));
        assert_eq!(service, "v2-origintunneld");
    }

    #[test]
    fn edge_host_label_default() {
        assert_eq!(edge_host_label(None), "region1.v2.argotunnel.com");
    }

    #[test]
    fn edge_host_label_fed() {
        assert_eq!(edge_host_label(Some(FED_ENDPOINT)), "region1.v2.argotunnel.com");
    }

    #[test]
    fn edge_host_label_region() {
        assert_eq!(edge_host_label(Some("us")), "us.v2.argotunnel.com");
    }

    #[test]
    fn edge_host_label_empty_falls_back() {
        assert_eq!(edge_host_label(Some("")), "region1.v2.argotunnel.com");
    }
}
