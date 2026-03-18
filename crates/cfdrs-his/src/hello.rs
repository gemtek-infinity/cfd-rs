//! Hello world origin test server.
//!
//! Covers HIS-072.
//!
//! Go: `hello/hello.go` serves a test origin with routes:
//! `/`, `/uptime`, `/ws`, `/sse`, `/_health`.
//!
//! The response contracts, route set, and behavioral constants are
//! defined here. Runtime server wiring (TLS listener, axum router)
//! lives in `cfdrs-bin`.

use serde::{Deserialize, Serialize};

/// Routes served by the Go hello world server.
pub const HELLO_ROUTES: &[&str] = &["/", "/uptime", "/ws", "/sse", "/_health"];

/// Go: `UptimeRoute = "/uptime"`.
pub const UPTIME_ROUTE: &str = "/uptime";

/// Go: `WSRoute = "/ws"`.
pub const WS_ROUTE: &str = "/ws";

/// Go: `SSERoute = "/sse"`.
pub const SSE_ROUTE: &str = "/sse";

/// Go: `HealthRoute = "/_health"`.
pub const HEALTH_ROUTE: &str = "/_health";

/// Default server name displayed in the root HTML page.
///
/// Go: `defaultServerName = "the Cloudflare Tunnel test server"`.
pub const DEFAULT_SERVER_NAME: &str = "the Cloudflare Tunnel test server";

/// Default SSE event frequency in seconds.
///
/// Go: `defaultSSEFreq = time.Second * 10`.
pub const DEFAULT_SSE_FREQ_SECS: u64 = 10;

/// Health check response body.
///
/// Go: `healthHandler()` writes `"ok"`.
pub const HEALTH_RESPONSE: &str = "ok";

/// JSON response for the `/uptime` endpoint.
///
/// Go: `OriginUpTime { StartTime time.Time, UpTime string }`.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UptimeResponse {
    /// RFC 3339 timestamp of server start.
    pub start_time: String,
    /// Human-readable duration since start (e.g. `"1h30m0s"`).
    pub uptime: String,
}

/// Trait for the hello world test origin.
pub trait HelloServer: Send + Sync {
    /// Start serving. Blocks until shutdown.
    fn serve(&self, addr: std::net::SocketAddr) -> cfdrs_shared::Result<()>;

    /// Signal shutdown.
    fn shutdown(&self);
}

/// Stub hello server.
pub struct StubHelloServer;

impl HelloServer for StubHelloServer {
    fn serve(&self, _addr: std::net::SocketAddr) -> cfdrs_shared::Result<()> {
        Err(cfdrs_shared::ConfigError::deferred("hello world server"))
    }

    fn shutdown(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_routes_match_go() {
        assert!(HELLO_ROUTES.contains(&"/"));
        assert!(HELLO_ROUTES.contains(&UPTIME_ROUTE));
        assert!(HELLO_ROUTES.contains(&WS_ROUTE));
        assert!(HELLO_ROUTES.contains(&SSE_ROUTE));
        assert!(HELLO_ROUTES.contains(&HEALTH_ROUTE));
    }

    #[test]
    fn hello_route_count_is_five() {
        assert_eq!(HELLO_ROUTES.len(), 5);
    }

    #[test]
    fn stub_hello_server_returns_deferred() {
        let server = StubHelloServer;
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 0));
        assert!(server.serve(addr).is_err());
    }

    // --- HIS-072: response contract parity ---

    #[test]
    fn route_constants_match_go_baseline() {
        assert_eq!(UPTIME_ROUTE, "/uptime");
        assert_eq!(WS_ROUTE, "/ws");
        assert_eq!(SSE_ROUTE, "/sse");
        assert_eq!(HEALTH_ROUTE, "/_health");
    }

    #[test]
    fn default_server_name_matches_go() {
        assert_eq!(DEFAULT_SERVER_NAME, "the Cloudflare Tunnel test server");
    }

    #[test]
    fn default_sse_freq_is_ten_seconds() {
        assert_eq!(DEFAULT_SSE_FREQ_SECS, 10);
    }

    #[test]
    fn health_response_is_ok() {
        assert_eq!(HEALTH_RESPONSE, "ok");
    }

    #[test]
    fn uptime_response_json_field_names_match_go() {
        let resp = UptimeResponse {
            start_time: "2025-01-01T00:00:00Z".to_owned(),
            uptime: "1h30m0s".to_owned(),
        };
        let json = serde_json::to_string(&resp).expect("serialize UptimeResponse");

        assert!(json.contains("\"startTime\""), "Go uses startTime");
        assert!(json.contains("\"uptime\""), "Go uses uptime");

        let parsed: UptimeResponse = serde_json::from_str(&json).expect("deserialize UptimeResponse");
        assert_eq!(parsed.start_time, "2025-01-01T00:00:00Z");
        assert_eq!(parsed.uptime, "1h30m0s");
    }
}
