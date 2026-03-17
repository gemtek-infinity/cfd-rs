//! Cloudflare REST API client contracts (CDC-033, CDC-034, CDC-035).
//!
//! Response envelope, pagination, auth header constants, client trait,
//! and error types matching `baseline-2026.2.0/cfapi/base_client.go` and
//! `baseline-2026.2.0/cfapi/client.go`.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::api_resources::{
    ActiveClient, DetailedRoute, HostnameRoute, HostnameRouteResult, IpRouteFilter, ManagementResource,
    NewRoute, NewVirtualNetwork, Route, Tunnel, TunnelFilter, TunnelWithToken, UpdateVirtualNetwork,
    VirtualNetwork, VnetFilter,
};

// ---------------------------------------------------------------------------
// API response envelope (CDC-034)
// ---------------------------------------------------------------------------

/// Cloudflare API response envelope.
///
/// Matches `response` in `cfapi/base_client.go`:
/// ```text
/// type response struct {
///     Success    bool            `json:"success"`
///     Errors     []apiError      `json:"errors"`
///     Messages   []string        `json:"messages"`
///     Result     json.RawMessage `json:"result"`
///     ResultInfo *Pagination      `json:"result_info"`
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<ApiError>,
    #[serde(default)]
    pub messages: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_info: Option<Pagination>,
}

/// A single Cloudflare API error entry.
///
/// Matches `apiError` in `cfapi/base_client.go`:
/// ```text
/// type apiError struct {
///     Code    json.Number `json:"code,omitempty"`
///     Message string      `json:"message,omitempty"`
/// }
/// ```
///
/// Go uses `json.Number` for code, which serializes as an unquoted JSON
/// number. We use `u32` which matches the observed values.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct ApiError {
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub code: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
}

fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

/// Pagination info for list endpoints.
///
/// Matches `Pagination` in `cfapi/base_client.go`:
/// ```text
/// type Pagination struct {
///     Count      int `json:"count,omitempty"`
///     Page       int `json:"page,omitempty"`
///     PerPage    int `json:"per_page,omitempty"`
///     TotalCount int `json:"total_count,omitempty"`
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct Pagination {
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub count: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub page: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub per_page: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub total_count: usize,
}

fn is_zero_usize(v: &usize) -> bool {
    *v == 0
}

// ---------------------------------------------------------------------------
// Auth and request constants (CDC-035)
// ---------------------------------------------------------------------------

/// Authorization header prefix for Bearer token auth.
pub const AUTHORIZATION_BEARER_PREFIX: &str = "Bearer ";

/// Accept header value for Cloudflare API requests.
///
/// Matches `accept` in `cfapi/base_client.go`:
/// `application/json;version=1`
pub const API_ACCEPT_HEADER: &str = "application/json;version=1";

/// Content-Type for JSON request bodies.
pub const JSON_CONTENT_TYPE: &str = "application/json";

/// Default HTTP client timeout for Cloudflare API requests.
///
/// Matches `defaultTimeout` in `cfapi/base_client.go` (15 seconds).
pub const DEFAULT_API_TIMEOUT: Duration = Duration::from_secs(15);

// ---------------------------------------------------------------------------
// API path templates
// ---------------------------------------------------------------------------

/// Account-scoped tunnel API path prefix.
///
/// Matches patterns in `cfapi/tunnel.go`:
/// `/accounts/{accountTag}/cfd_tunnel`
pub const ACCOUNT_TUNNEL_PATH: &str = "/accounts/{accountTag}/cfd_tunnel";

/// Account-scoped route API path prefix.
///
/// Matches patterns in `cfapi/ip_route.go`:
/// `/accounts/{accountTag}/teamnet/routes`
pub const ACCOUNT_ROUTE_PATH: &str = "/accounts/{accountTag}/teamnet/routes";

/// Account-scoped virtual network API path prefix.
///
/// Matches patterns in `cfapi/virtual_network.go`:
/// `/accounts/{accountTag}/teamnet/virtual_networks`
pub const ACCOUNT_VNET_PATH: &str = "/accounts/{accountTag}/teamnet/virtual_networks";

// ---------------------------------------------------------------------------
// Client error types
// ---------------------------------------------------------------------------

/// Errors from the Cloudflare REST API client.
///
/// Matches the sentinel errors in `cfapi/base_client.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiClientError {
    /// HTTP 401 — unauthorized / invalid token.
    Unauthorized,
    /// HTTP 400 — bad request.
    BadRequest,
    /// HTTP 404 — resource not found.
    NotFound,
    /// API returned `success: false` in the envelope.
    NoSuccess { errors: Vec<ApiError> },
    /// HTTP 409 — tunnel name conflict.
    TunnelNameConflict,
    /// Transport-level failure (DNS, timeout, connection reset).
    Transport(String),
}

impl std::fmt::Display for ApiClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::BadRequest => write!(f, "bad request"),
            Self::NotFound => write!(f, "not found"),
            Self::TunnelNameConflict => write!(f, "tunnel name already in use"),
            Self::Transport(msg) => write!(f, "transport error: {msg}"),
            Self::NoSuccess { errors } => {
                write!(f, "API request failed:")?;
                for e in errors {
                    write!(f, " [{}] {}", e.code, e.message)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ApiClientError {}

// ---------------------------------------------------------------------------
// Response parsing helpers (CDC-034)
// ---------------------------------------------------------------------------

impl ApiResponse {
    /// Parse the `result` field into a typed value.
    ///
    /// Matches Go's `parseResponse` → `parseResponseBody` in
    /// `cfapi/base_client.go`.
    pub fn parse_result<T: serde::de::DeserializeOwned>(&self) -> Result<T, ApiClientError> {
        let result = self.result.as_ref().ok_or_else(|| ApiClientError::NoSuccess {
            errors: self.errors.clone(),
        })?;

        serde_json::from_value(result.clone()).map_err(|e| ApiClientError::Transport(e.to_string()))
    }

    /// Check the envelope for API-level errors.
    ///
    /// Returns `Ok(())` when the response is successful, or an appropriate
    /// `ApiClientError` when the API returned errors.
    pub fn check(&self) -> Result<(), ApiClientError> {
        if self.success {
            return Ok(());
        }

        Err(ApiClientError::NoSuccess {
            errors: self.errors.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Client configuration (CDC-035)
// ---------------------------------------------------------------------------

/// Default Cloudflare API base URL.
///
/// Matches `DEFAULT_URL` in `cfapi/base_client.go`.
pub const DEFAULT_API_BASE_URL: &str = "https://api.cloudflare.com/client/v4";

/// FedRAMP API base URL.
pub const FED_API_BASE_URL: &str = "https://api.fed.cloudflare.com/client/v4";

/// Configuration for constructing a Cloudflare API client.
///
/// Matches the fields passed to `NewRESTClient` in `cfapi/base_client.go`.
#[derive(Debug, Clone)]
pub struct ApiClientConfig {
    pub base_url: String,
    pub account_tag: String,
    pub zone_tag: String,
    pub auth_token: String,
    pub user_agent: String,
}

impl ApiClientConfig {
    /// Build the account-level tunnel API URL prefix.
    pub fn account_tunnel_url(&self) -> String {
        format!("{}/accounts/{}/cfd_tunnel", self.base_url, self.account_tag)
    }

    /// Build the account-level route API URL prefix.
    pub fn account_route_url(&self) -> String {
        format!("{}/accounts/{}/teamnet/routes", self.base_url, self.account_tag)
    }

    /// Build the account-level virtual network API URL prefix.
    pub fn account_vnet_url(&self) -> String {
        format!(
            "{}/accounts/{}/teamnet/virtual_networks",
            self.base_url, self.account_tag
        )
    }

    /// Build the zone-level tunnel routing URL prefix.
    pub fn zone_tunnel_url(&self) -> String {
        format!("{}/zones/{}/tunnels", self.base_url, self.zone_tag)
    }
}

// ---------------------------------------------------------------------------
// Client trait (CDC-033, CDC-036, CDC-037, CDC-038, CDC-039)
// ---------------------------------------------------------------------------

/// Cloudflare REST API client contract.
///
/// Matches the `Client` interface composition in `cfapi/client.go`:
/// `TunnelClient + HostnameClient + IPRouteClient + VnetClient`.
///
/// The trait lives in `cfdrs-cdc` (contract owner); the `reqwest`
/// implementation lives in `cfdrs-bin` (composition root).
pub trait CloudflareApiClient {
    // -- TunnelClient (CDC-033) -------------------------------------------

    /// Create a named tunnel.
    ///
    /// POST `/accounts/{a}/cfd_tunnel`
    fn create_tunnel(&self, name: &str, tunnel_secret: &[u8]) -> Result<TunnelWithToken, ApiClientError>;

    /// Get a single tunnel by ID.
    ///
    /// GET `/accounts/{a}/cfd_tunnel/{tunnelID}`
    fn get_tunnel(&self, tunnel_id: Uuid) -> Result<Tunnel, ApiClientError>;

    /// Get the token string for a tunnel.
    ///
    /// GET `/accounts/{a}/cfd_tunnel/{tunnelID}/token`
    fn get_tunnel_token(&self, tunnel_id: Uuid) -> Result<String, ApiClientError>;

    /// Delete a tunnel (with optional cascade).
    ///
    /// DELETE `/accounts/{a}/cfd_tunnel/{tunnelID}?cascade={cascade}`
    fn delete_tunnel(&self, tunnel_id: Uuid, cascade: bool) -> Result<(), ApiClientError>;

    /// List tunnels with filterable query parameters.
    ///
    /// GET `/accounts/{a}/cfd_tunnel?{filter}` (paginated)
    fn list_tunnels(&self, filter: &TunnelFilter) -> Result<Vec<Tunnel>, ApiClientError>;

    /// List active connectors for a tunnel.
    ///
    /// GET `/accounts/{a}/cfd_tunnel/{tunnelID}/connections`
    fn list_active_clients(&self, tunnel_id: Uuid) -> Result<Vec<ActiveClient>, ApiClientError>;

    /// Clean up stale connections for a tunnel.
    ///
    /// DELETE `/accounts/{a}/cfd_tunnel/{tunnelID}/connections?client_id={id}`
    fn cleanup_connections(&self, tunnel_id: Uuid, connector_id: Option<Uuid>) -> Result<(), ApiClientError>;

    // -- IPRouteClient (CDC-036) ------------------------------------------

    /// List IP routes with filterable query parameters.
    ///
    /// GET `/accounts/{a}/teamnet/routes?{filter}` (paginated)
    fn list_routes(&self, filter: &IpRouteFilter) -> Result<Vec<DetailedRoute>, ApiClientError>;

    /// Add an IP route.
    ///
    /// POST `/accounts/{a}/teamnet/routes`
    fn add_route(&self, new_route: &NewRoute) -> Result<Route, ApiClientError>;

    /// Delete an IP route by ID.
    ///
    /// DELETE `/accounts/{a}/teamnet/routes/{routeID}`
    fn delete_route(&self, route_id: Uuid) -> Result<(), ApiClientError>;

    /// Look up the route for a specific IP.
    ///
    /// GET `/accounts/{a}/teamnet/routes/ip/{ip}?virtual_network_id={vnetID}`
    fn get_route_by_ip(&self, ip: &str, vnet_id: Option<Uuid>) -> Result<DetailedRoute, ApiClientError>;

    // -- VnetClient (CDC-037) ---------------------------------------------

    /// Create a virtual network.
    ///
    /// POST `/accounts/{a}/teamnet/virtual_networks`
    fn create_virtual_network(&self, new_vnet: &NewVirtualNetwork) -> Result<VirtualNetwork, ApiClientError>;

    /// List virtual networks.
    ///
    /// GET `/accounts/{a}/teamnet/virtual_networks?{filter}`
    fn list_virtual_networks(&self, filter: &VnetFilter) -> Result<Vec<VirtualNetwork>, ApiClientError>;

    /// Delete a virtual network.
    ///
    /// DELETE `/accounts/{a}/teamnet/virtual_networks/{id}?force={force}`
    fn delete_virtual_network(&self, id: Uuid, force: bool) -> Result<(), ApiClientError>;

    /// Update a virtual network.
    ///
    /// PATCH `/accounts/{a}/teamnet/virtual_networks/{id}`
    fn update_virtual_network(&self, id: Uuid, updates: &UpdateVirtualNetwork) -> Result<(), ApiClientError>;

    // -- Management (CDC-038) ---------------------------------------------

    /// Get a management token for a tunnel resource.
    ///
    /// POST `/accounts/{a}/cfd_tunnel/{tunnelID}/management/{resource}`
    fn get_management_token(
        &self,
        tunnel_id: Uuid,
        resource: ManagementResource,
    ) -> Result<String, ApiClientError>;

    // -- HostnameClient (CDC-039) -----------------------------------------

    /// Route a tunnel to a hostname (DNS or LB).
    ///
    /// PUT `/zones/{z}/tunnels/{tunnelID}/routes`
    fn route_tunnel(
        &self,
        tunnel_id: Uuid,
        route: &HostnameRoute,
    ) -> Result<HostnameRouteResult, ApiClientError>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Response envelope (CDC-034) --------------------------------------

    #[test]
    fn api_response_success_json_matches_go() {
        let json_str = r#"{
            "success": true,
            "errors": [],
            "messages": [],
            "result": {"id": "abc-123"},
            "result_info": {"count": 1, "page": 1, "per_page": 25, "total_count": 1}
        }"#;
        let resp: ApiResponse = serde_json::from_str(json_str).expect("deserialize");
        assert!(resp.success);
        assert!(resp.errors.is_empty());
        assert!(resp.messages.is_empty());
        let result = resp.result.expect("result present");
        assert_eq!(result["id"], "abc-123");
        let info = resp.result_info.expect("result_info present");
        assert_eq!(info.count, 1);
        assert_eq!(info.page, 1);
        assert_eq!(info.per_page, 25);
        assert_eq!(info.total_count, 1);
    }

    #[test]
    fn api_response_error_json_matches_go() {
        let json_str = r#"{
            "success": false,
            "errors": [{"code": 1003, "message": "Invalid tunnel ID"}],
            "messages": []
        }"#;
        let resp: ApiResponse = serde_json::from_str(json_str).expect("deserialize");
        assert!(!resp.success);
        assert_eq!(resp.errors.len(), 1);
        assert_eq!(resp.errors[0].code, 1003);
        assert_eq!(resp.errors[0].message, "Invalid tunnel ID");
        assert!(resp.result.is_none());
        assert!(resp.result_info.is_none());
    }

    #[test]
    fn api_response_roundtrip() {
        let resp = ApiResponse {
            success: true,
            errors: vec![],
            messages: vec!["created".to_string()],
            result: Some(serde_json::json!({"name": "test-tunnel"})),
            result_info: None,
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        let back: ApiResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(resp, back);
    }

    #[test]
    fn pagination_omits_zero_fields() {
        let p = Pagination {
            count: 0,
            page: 0,
            per_page: 0,
            total_count: 0,
        };
        let json = serde_json::to_string(&p).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
        // All zero fields should be omitted by Go's omitempty
        assert!(parsed.get("count").is_none());
        assert!(parsed.get("page").is_none());
    }

    #[test]
    fn pagination_json_keys_match_go() {
        let p = Pagination {
            count: 5,
            page: 2,
            per_page: 25,
            total_count: 42,
        };
        let json = serde_json::to_string(&p).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed["count"], 5);
        assert_eq!(parsed["page"], 2);
        assert_eq!(parsed["per_page"], 25);
        assert_eq!(parsed["total_count"], 42);
    }

    // -- Auth constants (CDC-035) -----------------------------------------

    #[test]
    fn auth_constants_match_go() {
        assert_eq!(AUTHORIZATION_BEARER_PREFIX, "Bearer ");
        assert_eq!(API_ACCEPT_HEADER, "application/json;version=1");
        assert_eq!(JSON_CONTENT_TYPE, "application/json");
        assert_eq!(DEFAULT_API_TIMEOUT, Duration::from_secs(15));
    }

    // -- API paths --------------------------------------------------------

    #[test]
    fn api_path_templates_match_go() {
        assert!(ACCOUNT_TUNNEL_PATH.contains("/accounts/"));
        assert!(ACCOUNT_TUNNEL_PATH.contains("cfd_tunnel"));
        assert!(ACCOUNT_ROUTE_PATH.contains("teamnet/routes"));
        assert!(ACCOUNT_VNET_PATH.contains("teamnet/virtual_networks"));
    }

    // -- Client errors ----------------------------------------------------

    #[test]
    fn api_client_error_display() {
        assert_eq!(ApiClientError::Unauthorized.to_string(), "unauthorized");
        assert_eq!(ApiClientError::BadRequest.to_string(), "bad request");
        assert_eq!(ApiClientError::NotFound.to_string(), "not found");
        assert_eq!(
            ApiClientError::TunnelNameConflict.to_string(),
            "tunnel name already in use"
        );
        assert!(
            ApiClientError::Transport("timeout".to_string())
                .to_string()
                .contains("timeout")
        );

        let err = ApiClientError::NoSuccess {
            errors: vec![ApiError {
                code: 1003,
                message: "invalid".to_string(),
            }],
        };
        let msg = err.to_string();
        assert!(msg.contains("1003"));
        assert!(msg.contains("invalid"));
    }

    #[test]
    fn api_error_omits_zero_code_and_empty_message() {
        let e = ApiError {
            code: 0,
            message: String::new(),
        };
        let json = serde_json::to_string(&e).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert!(parsed.get("code").is_none());
        assert!(parsed.get("message").is_none());
    }

    // -- Response parsing helpers (CDC-034) --------------------------------

    #[test]
    fn api_response_parse_result_typed() {
        let resp = ApiResponse {
            success: true,
            errors: vec![],
            messages: vec![],
            result: Some(serde_json::json!({"id": "00000000-0000-0000-0000-000000000000", "name": "test"})),
            result_info: None,
        };
        let tunnel: crate::api_resources::Tunnel = resp.parse_result().expect("parse");
        assert_eq!(tunnel.name, "test");
    }

    #[test]
    fn api_response_check_success() {
        let resp = ApiResponse {
            success: true,
            errors: vec![],
            messages: vec![],
            result: None,
            result_info: None,
        };
        assert!(resp.check().is_ok());
    }

    #[test]
    fn api_response_check_failure() {
        let resp = ApiResponse {
            success: false,
            errors: vec![ApiError {
                code: 1001,
                message: "missing".to_string(),
            }],
            messages: vec![],
            result: None,
            result_info: None,
        };
        let err = resp.check().expect_err("response should fail");
        assert!(matches!(err, ApiClientError::NoSuccess { .. }));
    }

    // -- Client config (CDC-035) ------------------------------------------

    #[test]
    fn api_client_config_builds_urls() {
        let config = ApiClientConfig {
            base_url: "https://api.cloudflare.com/client/v4".to_string(),
            account_tag: "abc123".to_string(),
            zone_tag: "zone456".to_string(),
            auth_token: "token".to_string(),
            user_agent: "cloudflared/test".to_string(),
        };
        assert_eq!(
            config.account_tunnel_url(),
            "https://api.cloudflare.com/client/v4/accounts/abc123/cfd_tunnel"
        );
        assert_eq!(
            config.account_route_url(),
            "https://api.cloudflare.com/client/v4/accounts/abc123/teamnet/routes"
        );
        assert_eq!(
            config.account_vnet_url(),
            "https://api.cloudflare.com/client/v4/accounts/abc123/teamnet/virtual_networks"
        );
        assert_eq!(
            config.zone_tunnel_url(),
            "https://api.cloudflare.com/client/v4/zones/zone456/tunnels"
        );
    }
}
