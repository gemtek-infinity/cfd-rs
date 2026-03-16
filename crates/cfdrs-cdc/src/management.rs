//! Management service contracts (CDC-023, CDC-024, CDC-025, CDC-027, CDC-028).
//!
//! Types and constants for the management WebSocket service that the
//! Cloudflare edge connects to for log streaming, host details, and
//! diagnostics.
//!
//! See `baseline-2026.2.0/management/service.go` and
//! `baseline-2026.2.0/management/middleware.go`.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Token parsing (CDC-024)
// ---------------------------------------------------------------------------

/// Issuer string for FED-issued management tokens.
///
/// Matches `tunnelstoreFEDIssuer` in `management/token.go`.
const TUNNELSTORE_FED_ISSUER: &str = "fed-tunnelstore";

/// Tunnel identity embedded in `tun` claim.
///
/// Matches `tunnel` in `management/token.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct ManagementTunnelClaim {
    pub id: String,
    pub account_tag: String,
}

/// Actor identity embedded in `actor` claim.
///
/// Matches `actor` in `management/token.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct ManagementActorClaim {
    pub id: String,
    #[serde(default)]
    pub support: bool,
}

/// Decoded management token claims.
///
/// Matches `managementTokenClaims` in `management/token.go`.
/// Go uses `UnsafeClaimsWithoutVerification` because the edge already
/// verifies the token before it reaches cloudflared.
#[derive(Debug, Clone, PartialEq)]
#[derive(Serialize, Deserialize)]
pub struct ManagementTokenClaims {
    pub tun: ManagementTunnelClaim,
    pub actor: ManagementActorClaim,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exp: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iat: Option<u64>,
}

impl ManagementTokenClaims {
    /// Verify that required claim fields are non-empty.
    ///
    /// Matches `managementTokenClaims.verify()` in Go.
    fn verify(&self) -> bool {
        !self.tun.id.is_empty() && !self.tun.account_tag.is_empty() && !self.actor.id.is_empty()
    }

    /// Returns true if the token was issued by the FED tunnelstore.
    ///
    /// Matches `managementTokenClaims.IsFed()` in Go.
    pub fn is_fed(&self) -> bool {
        self.iss.as_deref() == Some(TUNNELSTORE_FED_ISSUER)
    }
}

/// Parse a management JWT without signature verification.
///
/// Matches `ParseToken` in `management/token.go`. The Go baseline uses
/// `UnsafeClaimsWithoutVerification` because the edge already verifies
/// the token before forwarding it to cloudflared. We mirror that with
/// `insecure_disable_signature_validation`.
///
/// Returns the decoded claims or an error string.
pub fn parse_management_token(token: &str) -> Result<ManagementTokenClaims, String> {
    use jsonwebtoken::{Algorithm, DecodingKey, Validation};

    let mut validation = Validation::new(Algorithm::ES256);
    validation.insecure_disable_signature_validation();
    // Go does not validate exp/iat/nbf either.
    validation.validate_exp = false;
    validation.required_spec_claims.clear();

    let token_data =
        jsonwebtoken::decode::<ManagementTokenClaims>(token, &DecodingKey::from_secret(b""), &validation)
            .map_err(|e| format!("malformed jwt: {e}"))?;

    if !token_data.claims.verify() {
        return Err("invalid management token format provided".to_string());
    }

    Ok(token_data.claims)
}

// ---------------------------------------------------------------------------
// Route paths (CDC-023)
// ---------------------------------------------------------------------------

/// Management service route for ping/liveness.
pub const ROUTE_PING: &str = "/ping";

/// Management service route for log streaming (WebSocket).
pub const ROUTE_LOGS: &str = "/logs";

/// Management service route for host details.
pub const ROUTE_HOST_DETAILS: &str = "/host_details";

/// Management service route for Prometheus metrics (conditional).
pub const ROUTE_METRICS: &str = "/metrics";

/// Management service route prefix for pprof (conditional).
pub const ROUTE_DEBUG_PPROF: &str = "/debug/pprof/";

// ---------------------------------------------------------------------------
// CORS (CDC-027)
// ---------------------------------------------------------------------------

/// Allowed CORS origin pattern for management service.
///
/// Matches the Go baseline's `cors.Options.AllowedOrigins` in
/// `management/service.go`.
pub const CORS_ALLOWED_ORIGIN: &str = "https://*.cloudflare.com";

/// CORS max-age in seconds.
///
/// Matches the Go baseline's `MaxAge: 300` in `management/service.go`.
pub const CORS_MAX_AGE_SECS: u32 = 300;

/// Whether CORS allows credentials.
pub const CORS_ALLOW_CREDENTIALS: bool = true;

// ---------------------------------------------------------------------------
// Auth middleware (CDC-024)
// ---------------------------------------------------------------------------

/// Query parameter name for the management access token.
pub const ACCESS_TOKEN_QUERY_PARAM: &str = "access_token";

/// Error code for missing access token.
///
/// Matches `errMissingAccessToken.Code` in `management/middleware.go`.
pub const ERR_MISSING_ACCESS_TOKEN_CODE: u32 = 1001;

/// Error message for missing access token.
///
/// Matches `errMissingAccessToken.Message` in `management/middleware.go`.
pub const ERR_MISSING_ACCESS_TOKEN_MSG: &str = "missing access_token query parameter";

/// A management API error entry.
///
/// Matches `managementError` in `management/middleware.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct ManagementError {
    #[serde(skip_serializing_if = "is_zero_u32")]
    pub code: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
}

/// A management API error response envelope.
///
/// Matches `managementErrorResponse` in `management/middleware.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct ManagementErrorResponse {
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub success: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ManagementError>,
}

impl ManagementErrorResponse {
    /// Build the standard missing-access-token error response.
    ///
    /// Produces the exact JSON shape returned by Go's
    /// `ValidateAccessTokenQueryMiddleware`.
    pub fn missing_access_token() -> Self {
        Self {
            success: false,
            errors: vec![ManagementError {
                code: ERR_MISSING_ACCESS_TOKEN_CODE,
                message: ERR_MISSING_ACCESS_TOKEN_MSG.to_string(),
            }],
        }
    }
}

fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

// ---------------------------------------------------------------------------
// Host details (CDC-025)
// ---------------------------------------------------------------------------

/// Response from the `/host_details` management endpoint.
///
/// Matches `getHostDetailsResponse` in `management/service.go`:
/// ```text
/// type getHostDetailsResponse struct {
///     ClientID string `json:"connector_id"`
///     IP       string `json:"ip,omitempty"`
///     HostName string `json:"hostname,omitempty"`
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct HostDetailsResponse {
    pub connector_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ip: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hostname: String,
}

// ---------------------------------------------------------------------------
// Diagnostics conditional (CDC-028)
// ---------------------------------------------------------------------------

/// Diagnostic routes (`/metrics`, `/debug/pprof/`) are only registered on the
/// management service when this parameter is true.
///
/// Go baseline: `enableDiagServices` parameter in `management/service.go`
/// `New()` constructor controls whether metrics and pprof handlers are added.
pub const DIAG_ROUTES: &[&str] = &[ROUTE_METRICS, ROUTE_DEBUG_PPROF];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Route paths (CDC-023) --------------------------------------------

    #[test]
    fn management_route_paths_match_go() {
        assert_eq!(ROUTE_PING, "/ping");
        assert_eq!(ROUTE_LOGS, "/logs");
        assert_eq!(ROUTE_HOST_DETAILS, "/host_details");
        assert_eq!(ROUTE_METRICS, "/metrics");
        assert_eq!(ROUTE_DEBUG_PPROF, "/debug/pprof/");
    }

    // -- CORS (CDC-027) ---------------------------------------------------

    #[test]
    fn cors_constants_match_go() {
        assert_eq!(CORS_ALLOWED_ORIGIN, "https://*.cloudflare.com");
        assert_eq!(CORS_MAX_AGE_SECS, 300);
        const { assert!(CORS_ALLOW_CREDENTIALS) }
    }

    // -- Auth error (CDC-024) ---------------------------------------------

    #[test]
    fn missing_access_token_error_code_matches_go() {
        assert_eq!(ERR_MISSING_ACCESS_TOKEN_CODE, 1001);
        assert_eq!(
            ERR_MISSING_ACCESS_TOKEN_MSG,
            "missing access_token query parameter"
        );
    }

    #[test]
    fn missing_access_token_response_json_matches_go() {
        let resp = ManagementErrorResponse::missing_access_token();
        let json = serde_json::to_string(&resp).expect("serialize");

        // Go baseline produces exactly this shape:
        // {"errors":[{"code":1001,"message":"missing access_token query parameter"}]}
        // Note: success=false is omitted by Go's `omitempty` on bool.
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
        let errors = parsed["errors"].as_array().expect("errors array");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0]["code"], 1001);
        assert_eq!(errors[0]["message"], "missing access_token query parameter");
    }

    #[test]
    fn access_token_query_param_name() {
        assert_eq!(ACCESS_TOKEN_QUERY_PARAM, "access_token");
    }

    // -- Host details (CDC-025) -------------------------------------------

    #[test]
    fn host_details_json_keys_match_go() {
        let resp = HostDetailsResponse {
            connector_id: "test-uuid".to_string(),
            ip: "10.0.0.4".to_string(),
            hostname: "custom:label".to_string(),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");

        // Go uses `connector_id`, `ip`, `hostname` — all snake_case
        assert!(parsed.get("connector_id").is_some());
        assert!(parsed.get("ip").is_some());
        assert!(parsed.get("hostname").is_some());
        assert_eq!(parsed["connector_id"], "test-uuid");
        assert_eq!(parsed["ip"], "10.0.0.4");
        assert_eq!(parsed["hostname"], "custom:label");
    }

    #[test]
    fn host_details_omits_empty_fields() {
        let resp = HostDetailsResponse {
            connector_id: "uuid".to_string(),
            ip: String::new(),
            hostname: String::new(),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");

        // Go's omitempty means empty strings are not serialized
        assert!(parsed.get("connector_id").is_some());
        assert!(parsed.get("ip").is_none());
        assert!(parsed.get("hostname").is_none());
    }

    #[test]
    fn host_details_deserialize_go_json() {
        let go_json = r#"{"connector_id":"abc-123","ip":"10.0.0.4","hostname":"custom:label"}"#;
        let resp: HostDetailsResponse = serde_json::from_str(go_json).expect("deserialize");
        assert_eq!(resp.connector_id, "abc-123");
        assert_eq!(resp.ip, "10.0.0.4");
        assert_eq!(resp.hostname, "custom:label");
    }

    // -- Diagnostics conditional (CDC-028) --------------------------------

    #[test]
    fn diag_routes_match_go_conditional_set() {
        // Go only registers /metrics and /debug/pprof/ when enableDiagServices=true
        assert!(DIAG_ROUTES.contains(&ROUTE_METRICS));
        assert!(DIAG_ROUTES.contains(&ROUTE_DEBUG_PPROF));
        assert!(!DIAG_ROUTES.contains(&ROUTE_PING));
        assert!(!DIAG_ROUTES.contains(&ROUTE_LOGS));
        assert!(!DIAG_ROUTES.contains(&ROUTE_HOST_DETAILS));
    }

    /// CDC-024: Go omits zero code and empty message via `omitempty`.
    #[test]
    fn management_error_omits_zero_code_and_empty_message() {
        let err = ManagementError {
            code: 0,
            message: String::new(),
        };
        let json = serde_json::to_string(&err).expect("serialize");
        // Both fields skipped → empty object
        assert_eq!(json, "{}");
    }

    /// CDC-024: Exact byte-level match with Go's missing-access-token
    /// response.
    #[test]
    fn missing_access_token_response_exact_go_json_bytes() {
        let resp = ManagementErrorResponse::missing_access_token();
        let json = serde_json::to_string(&resp).expect("serialize");
        // Go produces: {"errors":[{"code":1001,"message":"missing access_token query
        // parameter"}]} success=false is omitted by Go's omitempty on bool.
        assert_eq!(
            json,
            r#"{"errors":[{"code":1001,"message":"missing access_token query parameter"}]}"#
        );
    }

    /// CDC-025: `connector_id` field uses UUID hyphenated string format
    /// matching Go's `uuid.String()` output.
    #[test]
    fn host_details_connector_id_is_uuid_string_format() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let resp = HostDetailsResponse {
            connector_id: uuid_str.to_string(),
            ip: "10.0.0.4".to_string(),
            hostname: String::new(),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
        // UUID must be hyphenated, not raw hex
        assert_eq!(parsed["connector_id"], uuid_str);
        assert!(parsed["connector_id"].as_str().expect("str").contains('-'));
    }

    // -- Token parsing (CDC-024) ------------------------------------------

    /// Helper: build a minimal JWT from claims without a real signature.
    ///
    /// Since `parse_management_token` disables signature verification
    /// (matching Go's `UnsafeClaimsWithoutVerification`), we only need a
    /// structurally valid three-part JWT.
    fn make_test_jwt(claims: &impl Serialize) -> String {
        use base64::Engine;
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;

        let header = r#"{"typ":"JWT","alg":"ES256","kid":"1"}"#;
        let header_b64 = engine.encode(header.as_bytes());
        let claims_json = serde_json::to_string(claims).expect("serialize");
        let claims_b64 = engine.encode(claims_json.as_bytes());
        // Dummy signature — not verified.
        let sig_b64 = engine.encode(b"not-a-real-signature");

        format!("{header_b64}.{claims_b64}.{sig_b64}")
    }

    fn valid_claims() -> ManagementTokenClaims {
        ManagementTokenClaims {
            tun: ManagementTunnelClaim {
                id: "7b098149-51fe-4ee5-a687-3e374466efc7".to_string(),
                account_tag: "cd391e9c0626a8f76cb1f670f6591b05".to_string(),
            },
            actor: ManagementActorClaim {
                id: "dcarr@cloudflare.com".to_string(),
                support: false,
            },
            iss: Some("tunnelstore".to_string()),
            exp: None,
            iat: None,
        }
    }

    #[test]
    fn parse_management_token_valid() {
        let jwt = make_test_jwt(&valid_claims());
        let claims = parse_management_token(&jwt).expect("parse valid token");
        assert_eq!(claims.tun.id, "7b098149-51fe-4ee5-a687-3e374466efc7");
        assert_eq!(claims.tun.account_tag, "cd391e9c0626a8f76cb1f670f6591b05");
        assert_eq!(claims.actor.id, "dcarr@cloudflare.com");
        assert!(!claims.actor.support);
    }

    #[test]
    fn parse_management_token_malformed() {
        let err = parse_management_token("not-a-jwt").expect_err("should fail");
        assert!(err.contains("malformed jwt"), "got: {err}");
    }

    #[test]
    fn parse_management_token_missing_tunnel_id() {
        let claims = ManagementTokenClaims {
            tun: ManagementTunnelClaim {
                id: String::new(),
                account_tag: "acct".to_string(),
            },
            actor: ManagementActorClaim {
                id: "actor".to_string(),
                support: false,
            },
            iss: None,
            exp: None,
            iat: None,
        };
        let jwt = make_test_jwt(&claims);
        let err = parse_management_token(&jwt).expect_err("should fail");
        assert_eq!(err, "invalid management token format provided");
    }

    #[test]
    fn parse_management_token_missing_account_tag() {
        let claims = ManagementTokenClaims {
            tun: ManagementTunnelClaim {
                id: "tid".to_string(),
                account_tag: String::new(),
            },
            actor: ManagementActorClaim {
                id: "actor".to_string(),
                support: false,
            },
            iss: None,
            exp: None,
            iat: None,
        };
        let jwt = make_test_jwt(&claims);
        let err = parse_management_token(&jwt).expect_err("should fail");
        assert_eq!(err, "invalid management token format provided");
    }

    #[test]
    fn parse_management_token_missing_actor_id() {
        let claims = ManagementTokenClaims {
            tun: ManagementTunnelClaim {
                id: "tid".to_string(),
                account_tag: "acct".to_string(),
            },
            actor: ManagementActorClaim {
                id: String::new(),
                support: false,
            },
            iss: None,
            exp: None,
            iat: None,
        };
        let jwt = make_test_jwt(&claims);
        let err = parse_management_token(&jwt).expect_err("should fail");
        assert_eq!(err, "invalid management token format provided");
    }

    #[test]
    fn parse_management_token_is_fed() {
        let mut claims = valid_claims();
        claims.iss = Some("fed-tunnelstore".to_string());
        let jwt = make_test_jwt(&claims);
        let parsed = parse_management_token(&jwt).expect("parse");
        assert!(parsed.is_fed());
    }

    #[test]
    fn parse_management_token_not_fed() {
        let claims = valid_claims();
        let jwt = make_test_jwt(&claims);
        let parsed = parse_management_token(&jwt).expect("parse");
        assert!(!parsed.is_fed());
    }

    /// Go's hardcoded valid token from token_test.go should parse
    /// successfully.
    #[test]
    fn parse_go_baseline_valid_token() {
        let go_token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzI1NiIsImtpZCI6IjEifQ.\
            eyJ0dW4iOnsiaWQiOiI3YjA5ODE0OS01MWZlLTRlZTUtYTY4Ny0zZTM3NDQ2NmVm\
            YzciLCJhY2NvdW50X3RhZyI6ImNkMzkxZTljMDYyNmE4Zjc2Y2IxZjY3MGY2NTkx\
            YjA1In0sImFjdG9yIjp7ImlkIjoiZGNhcnJAY2xvdWRmbGFyZS5jb20iLCJzdXBw\
            b3J0IjpmYWxzZX0sInJlcyI6WyJsb2dzIl0sImV4cCI6MTY3NzExNzY5NiwiaWF0\
            IjoxNjc3MTE0MDk2LCJpc3MiOiJ0dW5uZWxzdG9yZSJ9.\
            mKenOdOy3Xi4O-grldFnAAemdlE9WajEpTDC_FwezXQTstWiRTLwU65P5jt4vNsI\
            iZA4OJRq7bH-QYID9wf9NA";
        let claims = parse_management_token(go_token).expect("parse go token");
        assert_eq!(claims.tun.id, "7b098149-51fe-4ee5-a687-3e374466efc7");
        assert_eq!(claims.tun.account_tag, "cd391e9c0626a8f76cb1f670f6591b05");
        assert_eq!(claims.actor.id, "dcarr@cloudflare.com");
        assert!(!claims.is_fed());
    }
}
