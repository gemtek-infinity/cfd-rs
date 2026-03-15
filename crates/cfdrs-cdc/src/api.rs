//! Cloudflare REST API client contracts (CDC-034, CDC-035).
//!
//! Response envelope, pagination, auth header constants, and error
//! types matching `baseline-2026.2.0/cfapi/base_client.go` and
//! `baseline-2026.2.0/cfapi/client.go`.

use serde::{Deserialize, Serialize};
use std::time::Duration;

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
}

impl std::fmt::Display for ApiClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::BadRequest => write!(f, "bad request"),
            Self::NotFound => write!(f, "not found"),
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
}
