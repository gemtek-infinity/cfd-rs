//! Local metrics HTTP server surface.
//!
//! Covers HIS-024 through HIS-031.
//!
//! The server itself is trait-based: `MetricsServer` defines the contract,
//! and wiring to a real HTTP stack (hyper/axum) happens in cfdrs-bin.
//! This module provides the types, default constants, and response builders
//! that any HTTP implementation must use.

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// --- HIS-024: listener defaults ---

/// Default metrics address for host runtime.
pub const DEFAULT_METRICS_ADDRESS_HOST: &str = "localhost:0";

/// Default metrics address for virtual (container) runtime.
pub const DEFAULT_METRICS_ADDRESS_VIRTUAL: &str = "0.0.0.0:0";

/// HIS-024: known fallback ports for the metrics listener.
///
/// Go tries these in order when the primary address fails to bind.
pub const KNOWN_METRICS_PORTS: [u16; 5] = [20241, 20242, 20243, 20244, 20245];

/// HIS-024: server timeouts matching Go baseline.
pub const READ_TIMEOUT_SECS: u64 = 10;
pub const WRITE_TIMEOUT_SECS: u64 = 10;

/// Return the default metrics bind address for a runtime type.
pub fn default_metrics_address(is_virtual: bool) -> &'static str {
    if is_virtual {
        DEFAULT_METRICS_ADDRESS_VIRTUAL
    } else {
        DEFAULT_METRICS_ADDRESS_HOST
    }
}

/// Return the known addresses to try, matching Go `GetMetricsKnownAddresses`.
pub fn known_metrics_addresses(is_virtual: bool) -> Vec<SocketAddr> {
    let host = if is_virtual { "0.0.0.0" } else { "127.0.0.1" };

    KNOWN_METRICS_PORTS
        .iter()
        .filter_map(|&port| format!("{host}:{port}").parse().ok())
        .collect()
}

// --- HIS-025: readiness response ---

/// Readiness response body matching Go `readiness.go` `body` struct.
///
/// JSON: `{"status":200,"readyConnections":N,"connectorId":"uuid"}`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessResponse {
    pub status: u16,
    pub ready_connections: u32,
    pub connector_id: Uuid,
}

impl ReadinessResponse {
    /// Build a readiness response. Status is 200 if ready, 503 if not.
    pub fn new(connector_id: Uuid, ready_connections: u32) -> Self {
        let status = if ready_connections > 0 { 200 } else { 503 };

        Self {
            status,
            ready_connections,
            connector_id,
        }
    }

    /// HTTP status code to return.
    pub fn http_status(&self) -> u16 {
        self.status
    }
}

// --- HIS-026: healthcheck ---

/// Healthcheck response body, matching Go `/healthcheck` handler.
pub const HEALTHCHECK_RESPONSE: &str = "OK\n";

// --- HIS-027: build info ---

/// Build info for the Prometheus `build_info` gauge.
#[derive(Debug, Clone)]
pub struct BuildInfo {
    pub goversion: &'static str,
    pub version: &'static str,
    pub revision: &'static str,
    pub build_type: &'static str,
}

// --- HIS-028: quick tunnel ---

/// Quick tunnel hostname response.
///
/// JSON: `{"hostname":"..."}`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickTunnelResponse {
    pub hostname: String,
}

// --- HIS-029: config endpoint ---

/// Stub for the `/config` endpoint response.
///
/// The real implementation depends on the CDC orchestrator contract
/// (`CDC-044`). This type captures the shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub version: u32,
    pub config: serde_json::Value,
}

// --- HIS-031: --metrics flag ---

/// Parse a `--metrics` address string into a bind address.
///
/// Go accepts `ADDRESS` (e.g., `localhost:2000`, `:2000`).
pub fn parse_metrics_address(addr: &str) -> Option<SocketAddr> {
    if let Ok(address) = addr.parse() {
        return Some(address);
    }

    if let Some(port) = addr.strip_prefix(':') {
        return format!("127.0.0.1:{port}").parse().ok();
    }

    if let Some(port) = addr.strip_prefix("localhost:") {
        return format!("127.0.0.1:{port}").parse().ok();
    }

    None
}

// --- HIS-030: diagnostic/pprof surface ---

/// Marker: the `/debug/pprof/*` endpoints are deferred (`HIS-030`).
/// In Go these use `http.DefaultServeMux` with `net/http/pprof` side-effects.
/// Rust equivalent would be `pprof-rs` or a custom profiling endpoint.
pub const PPROF_DEFERRED: bool = true;

// --- Trait-based server contract ---

/// Trait that an HTTP metrics server must implement.
///
/// `cfdrs-bin` provides the async implementation; `cfdrs-his` defines the
/// contract and the type-level response helpers above.
pub trait MetricsServer: Send + Sync {
    /// Start serving on the given address. Blocks until shutdown.
    fn serve(&self, addr: SocketAddr) -> cfdrs_shared::Result<()>;

    /// Initiate graceful shutdown.
    fn shutdown(&self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_response_200_when_connected() {
        let id = Uuid::new_v4();
        let resp = ReadinessResponse::new(id, 3);
        assert_eq!(resp.http_status(), 200);
        assert_eq!(resp.ready_connections, 3);
    }

    #[test]
    fn readiness_response_503_when_not_connected() {
        let id = Uuid::new_v4();
        let resp = ReadinessResponse::new(id, 0);
        assert_eq!(resp.http_status(), 503);
    }

    #[test]
    fn readiness_serializes_to_expected_json() {
        let id = Uuid::nil();
        let resp = ReadinessResponse::new(id, 2);
        let json = serde_json::to_string(&resp).expect("serialize");
        assert!(json.contains("\"status\":200"));
        assert!(json.contains("\"readyConnections\":2"));
        assert!(json.contains("\"connectorId\":\"00000000-0000-0000-0000-000000000000\""));
    }

    #[test]
    fn known_metrics_addresses_host() {
        let addrs = known_metrics_addresses(false);
        assert_eq!(addrs.len(), 5);
        assert!(addrs[0].to_string().contains("127.0.0.1:20241"));
    }

    #[test]
    fn known_metrics_addresses_virtual() {
        let addrs = known_metrics_addresses(true);
        assert_eq!(addrs.len(), 5);
        assert!(addrs[0].to_string().contains("0.0.0.0:20241"));
    }

    #[test]
    fn quick_tunnel_serializes() {
        let resp = QuickTunnelResponse {
            hostname: "example.trycloudflare.com".into(),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        assert!(json.contains("example.trycloudflare.com"));
    }

    #[test]
    fn parse_metrics_address_valid() {
        assert!(parse_metrics_address("127.0.0.1:9090").is_some());
        assert!(parse_metrics_address("localhost:9090").is_some());
        assert!(parse_metrics_address(":9090").is_some());
    }

    #[test]
    fn parse_metrics_address_invalid() {
        assert!(parse_metrics_address("not-an-address").is_none());
    }

    // --- HIS-025: readiness JSON field names match Go exactly ---

    #[test]
    fn readiness_json_field_names_match_go_baseline() {
        // Go: `{"status":200,"readyConnections":N,"connectorId":"uuid"}`
        let id = Uuid::nil();
        let resp = ReadinessResponse::new(id, 4);
        let json = serde_json::to_string(&resp).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");

        assert!(parsed.get("status").is_some(), "must have 'status' key");
        assert!(
            parsed.get("readyConnections").is_some(),
            "must have 'readyConnections' key (camelCase)"
        );
        assert!(
            parsed.get("connectorId").is_some(),
            "must have 'connectorId' key (camelCase)"
        );

        // Exactly 3 fields, no extras.
        assert_eq!(parsed.as_object().expect("object").len(), 3);
    }

    #[test]
    fn readiness_deserializes_from_go_json_shape() {
        let json =
            r#"{"status":200,"readyConnections":2,"connectorId":"00000000-0000-0000-0000-000000000000"}"#;
        let resp: ReadinessResponse = serde_json::from_str(json).expect("deserialize");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.ready_connections, 2);
        assert_eq!(resp.connector_id, Uuid::nil());
    }

    // --- HIS-026: healthcheck response body matches Go ---

    #[test]
    fn healthcheck_response_body_matches_go_baseline() {
        // Go: `OK\n` (text/plain)
        assert_eq!(HEALTHCHECK_RESPONSE, "OK\n");
    }

    // --- HIS-024: metrics server constants match Go ---

    #[test]
    fn default_metrics_address_host_matches_go() {
        // Go: `localhost:0`
        assert_eq!(DEFAULT_METRICS_ADDRESS_HOST, "localhost:0");
    }

    #[test]
    fn default_metrics_address_virtual_matches_go() {
        // Go: `0.0.0.0:0`
        assert_eq!(DEFAULT_METRICS_ADDRESS_VIRTUAL, "0.0.0.0:0");
    }

    #[test]
    fn known_metrics_ports_match_go_fallback_range() {
        // Go tries ports 20241-20245 in order.
        assert_eq!(KNOWN_METRICS_PORTS, [20241, 20242, 20243, 20244, 20245]);
    }

    #[test]
    fn read_write_timeouts_match_go_baseline() {
        // Go: ReadTimeout=10s, WriteTimeout=10s
        assert_eq!(READ_TIMEOUT_SECS, 10);
        assert_eq!(WRITE_TIMEOUT_SECS, 10);
    }

    #[test]
    fn default_metrics_address_routes_by_runtime_type() {
        assert_eq!(default_metrics_address(false), "localhost:0");
        assert_eq!(default_metrics_address(true), "0.0.0.0:0");
    }

    // --- HIS-031: --metrics flag address parsing ---

    #[test]
    fn parse_metrics_address_colon_port_binds_localhost() {
        // Go: `:PORT` → binds to localhost
        let addr = parse_metrics_address(":2000").expect("colon-port should parse");
        assert_eq!(addr.port(), 2000);
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
    }

    #[test]
    fn parse_metrics_address_localhost_port_resolves() {
        let addr = parse_metrics_address("localhost:9090").expect("localhost should parse");
        assert_eq!(addr.port(), 9090);
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
    }

    #[test]
    fn parse_metrics_address_explicit_ip() {
        let addr = parse_metrics_address("0.0.0.0:3000").expect("explicit ip should parse");
        assert_eq!(addr.port(), 3000);
    }

    // --- HIS-027: build info shape ---

    #[test]
    fn config_response_serializes_to_expected_shape() {
        // Go: `{"version":N,"config":{ingress, warp-routing, originRequest}}`
        let resp = ConfigResponse {
            version: 1,
            config: serde_json::json!({"ingress": [], "warp-routing": {}, "originRequest": {}}),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");

        assert!(parsed.get("version").is_some());
        assert!(parsed.get("config").is_some());

        let config = parsed.get("config").expect("config");
        assert!(config.get("ingress").is_some());
        assert!(config.get("warp-routing").is_some());
        assert!(config.get("originRequest").is_some());
    }
}
