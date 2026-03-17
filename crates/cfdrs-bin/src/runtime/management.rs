//! Management HTTP service (CDC-023, CDC-024).
//!
//! Axum router for the management service that the Cloudflare edge
//! connects to for log streaming, host details, and diagnostics.
//!
//! See `baseline-2026.2.0/management/service.go` and
//! `baseline-2026.2.0/management/middleware.go`.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use cfdrs_cdc::management::{
    HostDetailsResponse, ManagementErrorResponse, ROUTE_HOST_DETAILS, ROUTE_LOGS, ROUTE_METRICS, ROUTE_PING,
};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Management service state
// ---------------------------------------------------------------------------

struct ManagementState {
    connector_id: uuid::Uuid,
    label: String,
}

// ---------------------------------------------------------------------------
// Router construction (CDC-023)
// ---------------------------------------------------------------------------

/// Build the management service router.
///
/// Mirrors `management.New()` in Go: installs auth middleware on all routes,
/// then registers the default management routes (`/ping`, `/logs`,
/// `/host_details`) and conditionally the diagnostic routes (`/metrics`,
/// `/debug/pprof/{profile}`).
pub(super) fn build_management_router(
    connector_id: uuid::Uuid,
    label: String,
    enable_diag_services: bool,
) -> Router {
    let state = Arc::new(ManagementState { connector_id, label });

    let mut router = Router::new()
        .route(ROUTE_PING, get(handle_ping))
        .route(ROUTE_LOGS, get(handle_logs_stub))
        .route(ROUTE_HOST_DETAILS, get(handle_host_details));

    if enable_diag_services {
        router = router
            .route(ROUTE_METRICS, get(handle_diag_metrics_stub))
            .route("/debug/pprof/{profile}", get(handle_diag_pprof_stub));
    }

    router
        .layer(middleware::from_fn(auth_middleware))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Auth middleware (CDC-024)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct AccessTokenParams {
    access_token: Option<String>,
}

/// Validate `?access_token=<JWT>` on every management request.
///
/// Matches Go `ValidateAccessTokenQueryMiddleware`: extract the token from
/// the query string, parse without signature verification, and reject with
/// a 400 error if missing or malformed.
async fn auth_middleware(
    Query(params): Query<AccessTokenParams>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let token_str = match params.access_token.as_deref() {
        Some(t) if !t.is_empty() => t,
        _ => return missing_access_token_response(),
    };

    match cfdrs_cdc::management::parse_management_token(token_str) {
        Ok(claims) => {
            request.extensions_mut().insert(claims);
            next.run(request).await
        }
        Err(_) => missing_access_token_response(),
    }
}

fn missing_access_token_response() -> Response {
    let body = serde_json::to_string(&ManagementErrorResponse::missing_access_token()).unwrap_or_default();

    (
        StatusCode::BAD_REQUEST,
        [(header::CONTENT_TYPE, "application/json")],
        body,
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET/HEAD `/ping` — liveness probe.
///
/// Matches `ping()` in Go: writes 200 with empty body.
async fn handle_ping() -> StatusCode {
    StatusCode::OK
}

/// GET `/host_details` — connector identity.
///
/// Matches `getHostDetails()` in Go. Returns `HostDetailsResponse` JSON
/// with connector_id, hostname label, and local IP.
/// Full IP detection is CDC-025 scope.
async fn handle_host_details(State(state): State<Arc<ManagementState>>) -> Response {
    let hostname = get_host_label(&state.label);

    let response = HostDetailsResponse {
        connector_id: state.connector_id.to_string(),
        ip: String::new(),
        hostname,
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&response).unwrap_or_default(),
    )
        .into_response()
}

/// GET `/logs` — WebSocket log streaming.
///
/// Stub returning 501 until CDC-026 wires the WebSocket upgrade.
async fn handle_logs_stub() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// GET `/metrics` — Prometheus metrics (conditional diagnostic route).
///
/// Stub returning 501 until wired to the runtime metrics registry.
async fn handle_diag_metrics_stub() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// GET `/debug/pprof/{profile}` — profiling (conditional diagnostic route).
///
/// Stub returning 501 matching the deferred pprof boundary.
async fn handle_diag_pprof_stub() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Derive the hostname label for `/host_details`.
///
/// Matches Go `getLabel()`: returns `"custom:{label}"` if a custom label
/// is set, otherwise falls back to the system hostname.
fn get_host_label(label: &str) -> String {
    if !label.is_empty() {
        return format!("custom:{label}");
    }

    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|_| "unknown".to_owned())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use super::*;

    /// Build a minimal test JWT that `parse_management_token` accepts.
    fn make_test_jwt() -> String {
        use base64::Engine;
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;

        let header = r#"{"typ":"JWT","alg":"ES256","kid":"1"}"#;
        let claims = serde_json::json!({
            "tun": {"id": "test-tunnel-id", "account_tag": "test-account"},
            "actor": {"id": "test-actor", "support": false}
        });
        let sig = engine.encode(b"not-a-real-signature");

        format!(
            "{}.{}.{sig}",
            engine.encode(header.as_bytes()),
            engine.encode(claims.to_string().as_bytes()),
        )
    }

    fn authed_get(path: &str) -> Request<Body> {
        let token = make_test_jwt();
        Request::get(format!("{path}?access_token={token}"))
            .body(Body::empty())
            .expect("request")
    }

    fn unauthed_get(path: &str) -> Request<Body> {
        Request::get(path).body(Body::empty()).expect("request")
    }

    // -- Route inventory (CDC-023) ----------------------------------------

    #[tokio::test]
    async fn ping_route_returns_200() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), false);

        let response = app.oneshot(authed_get("/ping")).await.expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn ping_head_returns_200() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), false);
        let token = make_test_jwt();

        let response = app
            .oneshot(
                Request::head(format!("/ping?access_token={token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn logs_route_returns_stub() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), false);

        let response = app.oneshot(authed_get("/logs")).await.expect("response");

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn host_details_route_returns_json() {
        let id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("uuid");
        let app = build_management_router(id, "my-label".to_owned(), false);

        let response = app.oneshot(authed_get("/host_details")).await.expect("response");

        assert_eq!(response.status(), StatusCode::OK);

        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("content-type");
        assert_eq!(content_type, "application/json");

        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        let parsed: serde_json::Value = serde_json::from_slice(&body).expect("json");

        assert_eq!(parsed["connector_id"], "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(parsed["hostname"], "custom:my-label");
    }

    // -- Conditional diagnostic routes (CDC-028) --------------------------

    #[tokio::test]
    async fn diag_metrics_available_when_enabled() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), true);

        let response = app.oneshot(authed_get("/metrics")).await.expect("response");

        // Route exists and responds (stub returns 501)
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn diag_metrics_absent_when_disabled() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), false);

        let response = app.oneshot(authed_get("/metrics")).await.expect("response");

        // Route not registered → 404
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn diag_pprof_available_when_enabled() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), true);

        let response = app
            .oneshot(authed_get("/debug/pprof/heap"))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn diag_pprof_absent_when_disabled() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), false);

        let response = app
            .oneshot(authed_get("/debug/pprof/heap"))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // -- Auth middleware (CDC-024) -----------------------------------------

    #[tokio::test]
    async fn missing_token_returns_400_with_error_code_1001() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), false);

        let response = app.oneshot(unauthed_get("/ping")).await.expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("content-type");
        assert_eq!(content_type, "application/json");

        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        let parsed: serde_json::Value = serde_json::from_slice(&body).expect("json");
        let errors = parsed["errors"].as_array().expect("errors array");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0]["code"], 1001);
        assert_eq!(errors[0]["message"], "missing access_token query parameter");
    }

    #[tokio::test]
    async fn invalid_token_returns_400() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), false);

        let response = app
            .oneshot(
                Request::get("/ping?access_token=not-a-jwt")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn empty_token_returns_400() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), false);

        let response = app
            .oneshot(
                Request::get("/ping?access_token=")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn valid_token_allows_request_through() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), false);

        let response = app.oneshot(authed_get("/ping")).await.expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    // -- Host label (CDC-025 prep) ----------------------------------------

    #[test]
    fn get_host_label_with_custom_label() {
        assert_eq!(get_host_label("my-connector"), "custom:my-connector");
    }

    #[test]
    fn get_host_label_empty_falls_back_to_system() {
        let label = get_host_label("");

        // On any system, the fallback should produce a non-empty string
        assert!(!label.is_empty());
        assert!(!label.starts_with("custom:"));
    }

    // -- Route completeness audit (CDC-023) -------------------------------

    #[tokio::test]
    async fn all_default_routes_respond_when_authenticated() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), false);

        for path in ["/ping", "/logs", "/host_details"] {
            let response = app
                .clone()
                .oneshot(authed_get(path))
                .await
                .unwrap_or_else(|_| panic!("{path} should respond"));

            assert_ne!(
                response.status(),
                StatusCode::NOT_FOUND,
                "{path} should be registered"
            );
        }
    }

    #[tokio::test]
    async fn all_diag_routes_respond_when_enabled() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), true);

        let diag_paths = ["/metrics", "/debug/pprof/heap", "/debug/pprof/goroutine"];

        for path in diag_paths {
            let response = app
                .clone()
                .oneshot(authed_get(path))
                .await
                .unwrap_or_else(|_| panic!("{path} should respond"));

            assert_ne!(
                response.status(),
                StatusCode::NOT_FOUND,
                "{path} should be registered when diag is enabled"
            );
        }
    }

    #[tokio::test]
    async fn auth_required_on_all_routes_including_diag() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), true);

        let all_paths = ["/ping", "/logs", "/host_details", "/metrics", "/debug/pprof/heap"];

        for path in all_paths {
            let response = app
                .clone()
                .oneshot(unauthed_get(path))
                .await
                .unwrap_or_else(|_| panic!("{path} should respond"));

            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "{path} should require auth"
            );
        }
    }
}
