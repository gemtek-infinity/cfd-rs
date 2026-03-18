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

/// Response shape for the `/config` endpoint.
///
/// Go: `Orchestrator.GetVersionedConfigJSON()` returns
/// `{"version": currentVersion, "config": {...}}` where `currentVersion`
/// is an `int32` starting at `-1` and incrementing on each remote config push.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub version: i32,
    pub config: serde_json::Value,
}

/// Build a `ConfigResponse` from a `ConfigOrchestrator`.
///
/// Go: `Orchestrator.GetVersionedConfigJSON()` — combines the monotonic
/// version counter with the current config snapshot.
pub fn versioned_config_response(
    orchestrator: &dyn crate::watcher::ConfigOrchestrator,
) -> cfdrs_shared::Result<ConfigResponse> {
    Ok(ConfigResponse {
        version: orchestrator.current_version(),
        config: orchestrator.get_config_json()?,
    })
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

// --- HIS-027: Go baseline Prometheus metric name inventory ---
//
// 19 metrics total across 5 Go source files. Listed here so parity
// tests can assert the expected registry surface without inspecting
// the Go baseline repeatedly.

/// Prometheus metric names from Go `metrics/metrics.go`.
pub mod baseline_metrics {
    // metrics/metrics.go (1)
    pub const BUILD_INFO: &str = "build_info";

    // connection/metrics.go (8)
    pub const TUNNEL_MAX_CONCURRENT_REQUESTS: &str = "cloudflared_tunnel_max_concurrent_requests_per_tunnel";
    pub const TUNNEL_SERVER_LOCATIONS: &str = "cloudflared_tunnel_server_locations";
    pub const TUNNEL_RPC_FAIL: &str = "cloudflared_tunnel_tunnel_rpc_fail";
    pub const TUNNEL_REGISTER_FAIL: &str = "cloudflared_tunnel_tunnel_register_fail";
    pub const TUNNEL_USER_HOSTNAMES: &str = "cloudflared_tunnel_user_hostnames_counts";
    pub const TUNNEL_REGISTER_SUCCESS: &str = "cloudflared_tunnel_tunnel_register_success";
    pub const CONFIG_LOCAL_PUSHES: &str = "cloudflared_config_local_config_pushes";
    pub const CONFIG_LOCAL_PUSHES_ERRORS: &str = "cloudflared_config_local_config_pushes_errors";

    // connection/tunnelsforha.go (1)
    pub const TUNNEL_IDS: &str = "tunnel_ids";

    // supervisor/metrics.go (1)
    pub const TUNNEL_HA_CONNECTIONS: &str = "cloudflared_tunnel_ha_connections";

    // proxy/metrics.go (8)
    pub const TUNNEL_TOTAL_REQUESTS: &str = "cloudflared_tunnel_total_requests";
    pub const TUNNEL_CONCURRENT_REQUESTS: &str = "cloudflared_tunnel_concurrent_requests_per_tunnel";
    pub const TUNNEL_RESPONSE_BY_CODE: &str = "cloudflared_tunnel_response_by_code";
    pub const TUNNEL_REQUEST_ERRORS: &str = "cloudflared_tunnel_request_errors";
    pub const TCP_ACTIVE_SESSIONS: &str = "cloudflared_tcp_active_sessions";
    pub const TCP_TOTAL_SESSIONS: &str = "cloudflared_tcp_total_sessions";
    pub const PROXY_CONNECT_LATENCY: &str = "cloudflared_proxy_connect_latency";
    pub const PROXY_CONNECT_STREAMS_ERRORS: &str = "cloudflared_proxy_connect_streams_errors";

    /// Total number of Go baseline Prometheus metrics.
    pub const BASELINE_METRIC_COUNT: usize = 19;

    /// All 19 Go baseline metric names for inventory assertion.
    pub const ALL: [&str; BASELINE_METRIC_COUNT] = [
        BUILD_INFO,
        TUNNEL_MAX_CONCURRENT_REQUESTS,
        TUNNEL_SERVER_LOCATIONS,
        TUNNEL_RPC_FAIL,
        TUNNEL_REGISTER_FAIL,
        TUNNEL_USER_HOSTNAMES,
        TUNNEL_REGISTER_SUCCESS,
        CONFIG_LOCAL_PUSHES,
        CONFIG_LOCAL_PUSHES_ERRORS,
        TUNNEL_IDS,
        TUNNEL_HA_CONNECTIONS,
        TUNNEL_TOTAL_REQUESTS,
        TUNNEL_CONCURRENT_REQUESTS,
        TUNNEL_RESPONSE_BY_CODE,
        TUNNEL_REQUEST_ERRORS,
        TCP_ACTIVE_SESSIONS,
        TCP_TOTAL_SESSIONS,
        PROXY_CONNECT_LATENCY,
        PROXY_CONNECT_STREAMS_ERRORS,
    ];
}

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

    // --- HIS-029: versioned config response from orchestrator ---

    #[test]
    fn versioned_config_response_reflects_initial_version() {
        use crate::watcher::InMemoryConfigOrchestrator;

        let orchestrator = InMemoryConfigOrchestrator::new(
            serde_json::json!({"ingress": [], "warp-routing": {}, "originRequest": {}}),
        );

        let response = versioned_config_response(&orchestrator).expect("should build response");
        assert_eq!(response.version, -1, "Go initial version is -1");
        assert!(response.config.get("ingress").is_some());
    }

    #[test]
    fn versioned_config_response_tracks_version_after_update() {
        use crate::watcher::{ConfigOrchestrator, InMemoryConfigOrchestrator};

        let orchestrator = InMemoryConfigOrchestrator::new(serde_json::json!({}));

        orchestrator.update_config(
            0,
            serde_json::json!({"ingress": [{"service": "http://localhost:8080"}]}),
        );

        let response = versioned_config_response(&orchestrator).expect("should build response");

        assert_eq!(response.version, 0);
        assert!(response.config.get("ingress").is_some());
    }

    #[test]
    fn versioned_config_response_shows_latest_version_after_multiple_updates() {
        use crate::watcher::{ConfigOrchestrator, InMemoryConfigOrchestrator};

        let orchestrator = InMemoryConfigOrchestrator::new(serde_json::json!({}));

        orchestrator.update_config(0, serde_json::json!({"v": 0}));
        orchestrator.update_config(1, serde_json::json!({"v": 1}));
        orchestrator.update_config(5, serde_json::json!({"v": 5}));

        let response = versioned_config_response(&orchestrator).expect("should build response");
        assert_eq!(response.version, 5);
    }

    // --- HIS-027: Go baseline Prometheus metric name inventory ---

    #[test]
    fn baseline_metric_inventory_has_19_entries() {
        use super::baseline_metrics;

        assert_eq!(baseline_metrics::ALL.len(), 19);
        assert_eq!(baseline_metrics::BASELINE_METRIC_COUNT, 19);
    }

    #[test]
    fn baseline_metric_names_are_unique() {
        use super::baseline_metrics;
        use std::collections::HashSet;

        let set: HashSet<&str> = baseline_metrics::ALL.iter().copied().collect();
        assert_eq!(
            set.len(),
            baseline_metrics::ALL.len(),
            "duplicate metric name in inventory"
        );
    }

    #[test]
    fn baseline_build_info_has_no_namespace() {
        // Go registers `build_info` without the `cloudflared_` namespace prefix.
        use super::baseline_metrics;

        assert_eq!(baseline_metrics::BUILD_INFO, "build_info");
        assert!(!baseline_metrics::BUILD_INFO.starts_with("cloudflared_"));
    }

    #[test]
    fn baseline_tunnel_ids_has_no_namespace() {
        // Go registers `tunnel_ids` without a namespace prefix.
        use super::baseline_metrics;

        assert_eq!(baseline_metrics::TUNNEL_IDS, "tunnel_ids");
        assert!(!baseline_metrics::TUNNEL_IDS.starts_with("cloudflared_"));
    }

    #[test]
    fn baseline_namespaced_metrics_all_start_with_cloudflared() {
        use super::baseline_metrics;

        let namespaced: Vec<&str> = baseline_metrics::ALL
            .iter()
            .copied()
            .filter(|name| *name != baseline_metrics::BUILD_INFO && *name != baseline_metrics::TUNNEL_IDS)
            .collect();

        assert_eq!(namespaced.len(), 17);

        for name in &namespaced {
            assert!(
                name.starts_with("cloudflared_"),
                "expected cloudflared_ prefix on {name}"
            );
        }
    }

    // --- HIS-031: container/runtime-class address routing ---

    #[test]
    fn container_runtime_binds_to_all_interfaces() {
        // Go: `metrics.Runtime = "virtual"` → default address `0.0.0.0:0`,
        //     known addresses use `0.0.0.0:2024x`.
        let default = default_metrics_address(true);
        assert!(
            default.starts_with("0.0.0.0"),
            "container runtime should bind to 0.0.0.0, got {default}"
        );

        let known = known_metrics_addresses(true);
        for addr in &known {
            assert!(
                addr.ip().is_unspecified(),
                "container known address should use 0.0.0.0, got {addr}"
            );
        }
    }

    #[test]
    fn host_runtime_binds_to_localhost() {
        // Go: `metrics.Runtime = "host"` (default) → `localhost:0`,
        //     known addresses use `localhost:2024x` (resolved as 127.0.0.1).
        let default = default_metrics_address(false);
        assert!(
            default.starts_with("localhost"),
            "host runtime should bind to localhost, got {default}"
        );

        let known = known_metrics_addresses(false);
        for addr in &known {
            assert!(
                addr.ip().is_loopback(),
                "host known address should use 127.0.0.1, got {addr}"
            );
        }
    }
}
