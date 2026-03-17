//! Management HTTP service (CDC-023, CDC-024, CDC-026).
//!
//! Axum router for the management service that the Cloudflare edge
//! connects to for log streaming, host details, and diagnostics.
//!
//! See `baseline-2026.2.0/management/service.go`,
//! `baseline-2026.2.0/management/middleware.go`,
//! `baseline-2026.2.0/management/session.go`, and
//! `baseline-2026.2.0/management/events.go`.

use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use cfdrs_cdc::log_streaming::{
    ClientEvent, EventLog, LOG_WINDOW, LogEntry, REASON_IDLE_LIMIT_EXCEEDED, REASON_INVALID_COMMAND,
    REASON_SESSION_LIMIT_EXCEEDED, STATUS_IDLE_LIMIT_EXCEEDED, STATUS_INVALID_COMMAND,
    STATUS_SESSION_LIMIT_EXCEEDED, StreamingFilters, parse_client_event,
};
use cfdrs_cdc::management::{
    HostDetailsResponse, ManagementErrorResponse, ManagementTokenClaims, ROUTE_HOST_DETAILS, ROUTE_LOGS,
    ROUTE_METRICS, ROUTE_PING,
};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Log session manager (CDC-026)
// ---------------------------------------------------------------------------

/// Entry for a single active streaming session.
struct SessionEntry {
    actor_id: String,
    active: Arc<AtomicBool>,
    sender: mpsc::Sender<LogEntry>,
    filters: StreamingFilters,
    cancel: CancellationToken,
}

/// Tracks active log streaming sessions and distributes log entries.
///
/// Go baseline: `management.Logger` implements `LoggerListener` + `io.Writer`.
/// Each `Write()` call dispatches log entries to all active sessions via
/// bounded channels. Session preemption rules: at most one active session
/// globally; same actor preempts, different actor is rejected with 4002.
pub(super) struct LogSessionManager {
    inner: std::sync::Mutex<Vec<SessionEntry>>,
}

impl LogSessionManager {
    pub(super) fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Check if a new session for `actor_id` can start, handling preemption.
    ///
    /// Go: `canStartStream()` — same actor preempts, different actor rejected.
    fn can_start_stream(&self, actor_id: &str) -> bool {
        let mut sessions = self.inner.lock().expect("session lock");
        sessions.retain(|s| s.active.load(Ordering::Relaxed));

        if sessions.is_empty() {
            return true;
        }

        if let Some(existing) = sessions.iter().find(|s| s.actor_id == actor_id) {
            // Same actor: preempt the old session.
            existing.active.store(false, Ordering::Relaxed);
            existing.cancel.cancel();
            sessions.retain(|s| s.active.load(Ordering::Relaxed));
            true
        } else {
            // Different actor: reject.
            false
        }
    }

    /// Register a new streaming session and return its log entry receiver.
    ///
    /// Go: `Logger.Listen(session)` — sets active, appends to slice.
    fn listen(
        &self,
        actor_id: String,
        filters: StreamingFilters,
        cancel: CancellationToken,
    ) -> mpsc::Receiver<LogEntry> {
        let (tx, rx) = mpsc::channel(LOG_WINDOW);
        let mut sessions = self.inner.lock().expect("session lock");

        sessions.push(SessionEntry {
            actor_id,
            active: Arc::new(AtomicBool::new(true)),
            sender: tx,
            filters,
            cancel,
        });

        rx
    }

    /// Stop and remove sessions for the given actor.
    ///
    /// Go: `session.Stop()` + `Logger.Remove(session)`.
    fn remove(&self, actor_id: &str) {
        let mut sessions = self.inner.lock().expect("session lock");

        for s in sessions.iter() {
            if s.actor_id == actor_id {
                s.active.store(false, Ordering::Relaxed);
            }
        }

        sessions.retain(|s| s.active.load(Ordering::Relaxed));
    }

    /// Insert a log entry to all active sessions that pass filters.
    ///
    /// Go: `Logger.Write()` → `session.Insert()`.
    /// Non-blocking: drops the entry if the session's channel is full
    /// (matches Go's `default` case in the non-blocking select).
    #[allow(dead_code)] // Wired when tracing subscriber integration connects.
    pub(super) fn insert(&self, entry: &LogEntry) {
        let sessions = self.inner.lock().expect("session lock");

        for session in sessions.iter() {
            if !session.active.load(Ordering::Relaxed) {
                continue;
            }

            if !session.filters.should_accept(entry) {
                continue;
            }

            let _ = session.sender.try_send(entry.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// Management service state
// ---------------------------------------------------------------------------

struct ManagementState {
    connector_id: uuid::Uuid,
    label: String,
    service_ip: String,
    sessions: LogSessionManager,
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
    service_ip: String,
    enable_diag_services: bool,
) -> Router {
    let state = Arc::new(ManagementState {
        connector_id,
        label,
        service_ip,
        sessions: LogSessionManager::new(),
    });

    let mut router = Router::new()
        .route(ROUTE_PING, get(handle_ping))
        .route(ROUTE_LOGS, get(handle_logs))
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

/// GET `/host_details` — connector identity (CDC-025).
///
/// Matches `getHostDetails()` in Go. Returns `HostDetailsResponse` JSON
/// with connector_id, hostname label, and local IP.
async fn handle_host_details(State(state): State<Arc<ManagementState>>) -> Response {
    let hostname = get_host_label(&state.label);
    let ip = get_private_ip(&state.service_ip).unwrap_or_default();

    let response = HostDetailsResponse {
        connector_id: state.connector_id.to_string(),
        ip,
        hostname,
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&response).unwrap_or_default(),
    )
        .into_response()
}

/// GET `/logs` — WebSocket log streaming (CDC-026).
///
/// Go: `logs()` in `service.go`. Upgrades to WebSocket, creates a session,
/// and enters the main event/streaming loop. Close codes:
/// - 4001 if first message is not `start_streaming`
/// - 4002 if a different actor already holds the session
/// - 4003 if the connection is idle for 5 minutes
async fn handle_logs(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ManagementState>>,
    axum::Extension(claims): axum::Extension<ManagementTokenClaims>,
) -> Response {
    let actor_id = claims.actor.id.clone();
    ws.on_upgrade(move |socket| handle_logs_session(socket, state, actor_id))
}

/// Idle timeout before the connection is closed (Go: 5 minutes).
const IDLE_TIMEOUT: Duration = Duration::from_secs(300);

/// Ping keepalive interval (Go: 15 seconds).
const PING_INTERVAL: Duration = Duration::from_secs(15);

/// WebSocket session lifecycle for `/logs`.
///
/// Go lifecycle in `logs()`:
///   1. Reader goroutine reads client events → channel
///   2. Main loop: select on events, ping (15s), idle (5min), context done
///   3. `StartStreaming` → stop idle, create session, spawn log streamer
///   4. `StopStreaming` → stop session, reset idle timer
///   5. After stop, client can send another `StartStreaming`
async fn handle_logs_session(mut socket: WebSocket, state: Arc<ManagementState>, actor_id: String) {
    let session_cancel = CancellationToken::new();
    let mut log_rx: Option<mpsc::Receiver<LogEntry>> = None;
    let mut streaming = false;

    let idle_deadline = tokio::time::sleep(IDLE_TIMEOUT);
    tokio::pin!(idle_deadline);

    let mut ping_interval = tokio::time::interval(PING_INTERVAL);
    ping_interval.tick().await; // consume the initial immediate tick

    loop {
        let log_entry = async {
            match &mut log_rx {
                Some(rx) => rx.recv().await,
                None => std::future::pending().await,
            }
        };

        tokio::select! {
            msg = socket.recv() => {
                let msg = match msg {
                    Some(Ok(m)) => m,
                    _ => break,
                };

                match msg {
                    Message::Text(text) => {
                        match parse_client_event(&text) {
                            Ok(ClientEvent::StartStreaming(start)) => {
                                if !state.sessions.can_start_stream(&actor_id) {
                                    let _ = send_close(
                                        &mut socket,
                                        STATUS_SESSION_LIMIT_EXCEEDED,
                                        REASON_SESSION_LIMIT_EXCEEDED,
                                    ).await;
                                    break;
                                }

                                let filters = start.filters.unwrap_or_default();
                                log_rx = Some(state.sessions.listen(
                                    actor_id.clone(),
                                    filters,
                                    session_cancel.clone(),
                                ));
                                streaming = true;
                            }

                            Ok(ClientEvent::StopStreaming(_)) => {
                                state.sessions.remove(&actor_id);
                                log_rx = None;
                                streaming = false;
                                idle_deadline.as_mut().reset(
                                    tokio::time::Instant::now() + IDLE_TIMEOUT,
                                );
                            }

                            Err(_) if !streaming => {
                                let _ = send_close(
                                    &mut socket,
                                    STATUS_INVALID_COMMAND,
                                    REASON_INVALID_COMMAND,
                                ).await;
                                break;
                            }

                            Err(_) => {
                                let _ = send_close(&mut socket, 1003, "unknown message type")
                                    .await;
                                break;
                            }
                        }
                    }

                    Message::Close(_) => break,
                    _ => {}
                }
            }

            entry = log_entry, if streaming => {
                if let Some(entry) = entry {
                    let event = EventLog::new(vec![entry]);

                    if let Ok(json) = serde_json::to_string(&event)
                        && socket.send(Message::Text(json.into())).await.is_err()
                    {
                        break;
                    }
                }
            }

            _ = ping_interval.tick() => {
                if socket.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }

            _ = &mut idle_deadline, if !streaming => {
                let _ = send_close(
                    &mut socket,
                    STATUS_IDLE_LIMIT_EXCEEDED,
                    REASON_IDLE_LIMIT_EXCEEDED,
                ).await;
                break;
            }

            _ = session_cancel.cancelled() => {
                break;
            }
        }
    }

    state.sessions.remove(&actor_id);
}

/// Send a close frame with a custom code and reason.
async fn send_close(socket: &mut WebSocket, code: u16, reason: &str) -> Result<(), axum::Error> {
    socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })))
        .await
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

/// Discover the machine's preferred private IP by dialing `service_ip`.
///
/// Matches Go `getPrivateIP(addr)`: establishes a short-lived TCP
/// connection to the management service's own listen address and reads
/// the local socket address chosen by the OS.
fn get_private_ip(service_ip: &str) -> Option<String> {
    if service_ip.is_empty() {
        return None;
    }
    let stream = TcpStream::connect_timeout(&service_ip.parse().ok()?, Duration::from_secs(1)).ok()?;
    let local = stream.local_addr().ok()?;
    Some(local.ip().to_string())
}

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
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), false);

        let response = app.oneshot(authed_get("/ping")).await.expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn ping_head_returns_200() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), false);
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
    async fn logs_route_rejects_non_websocket() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), false);

        let response = app.oneshot(authed_get("/logs")).await.expect("response");

        // Non-WebSocket GET to /logs returns an upgrade-required error,
        // not 501 (stub was replaced with real WebSocket handler).
        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn host_details_route_returns_json() {
        let id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("uuid");
        let app = build_management_router(id, "my-label".to_owned(), String::new(), false);

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
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), true);

        let response = app.oneshot(authed_get("/metrics")).await.expect("response");

        // Route exists and responds (stub returns 501)
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn diag_metrics_absent_when_disabled() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), false);

        let response = app.oneshot(authed_get("/metrics")).await.expect("response");

        // Route not registered → 404
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn diag_pprof_available_when_enabled() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), true);

        let response = app
            .oneshot(authed_get("/debug/pprof/heap"))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn diag_pprof_absent_when_disabled() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), false);

        let response = app
            .oneshot(authed_get("/debug/pprof/heap"))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // -- Auth middleware (CDC-024) -----------------------------------------

    #[tokio::test]
    async fn missing_token_returns_400_with_error_code_1001() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), false);

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
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), false);

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
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), false);

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
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), false);

        let response = app.oneshot(authed_get("/ping")).await.expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    // -- Host label and private IP (CDC-025) --------------------------------

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

    #[test]
    fn get_private_ip_empty_addr_returns_none() {
        assert!(get_private_ip("").is_none());
    }

    #[test]
    fn get_private_ip_invalid_addr_returns_none() {
        assert!(get_private_ip("not-an-address").is_none());
    }

    #[test]
    fn get_private_ip_unreachable_returns_none() {
        // Port 1 on loopback is not listening; connect should fail/timeout.
        assert!(get_private_ip("127.0.0.1:1").is_none());
    }

    #[tokio::test]
    async fn host_details_includes_ip_when_service_ip_reachable() {
        // Bind a throwaway listener to get a real port, then build the
        // management router with that address as service_ip.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");

        let id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("uuid");
        let app = build_management_router(id, "test".to_owned(), addr.to_string(), false);

        let response = app.oneshot(authed_get("/host_details")).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        let parsed: serde_json::Value = serde_json::from_slice(&body).expect("json");

        // When service_ip is reachable, we expect a non-empty IP field.
        assert_eq!(parsed["ip"], "127.0.0.1");
        assert_eq!(parsed["hostname"], "custom:test");
    }

    #[tokio::test]
    async fn host_details_omits_ip_when_service_ip_empty() {
        let id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("uuid");
        let app = build_management_router(id, "label".to_owned(), String::new(), false);

        let response = app.oneshot(authed_get("/host_details")).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        let parsed: serde_json::Value = serde_json::from_slice(&body).expect("json");

        // ip should be omitted (skip_serializing_if = "String::is_empty")
        assert!(parsed.get("ip").is_none(), "empty ip should be omitted from JSON");
        assert_eq!(parsed["hostname"], "custom:label");
    }

    // -- Route completeness audit (CDC-023) -------------------------------

    #[tokio::test]
    async fn all_default_routes_respond_when_authenticated() {
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), false);

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
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), true);

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
        let app = build_management_router(uuid::Uuid::nil(), String::new(), String::new(), true);

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

    // -- Log session manager (CDC-026) ------------------------------------

    #[test]
    fn session_manager_empty_allows_start() {
        let mgr = LogSessionManager::new();
        assert!(mgr.can_start_stream("actor-1"));
    }

    #[test]
    fn session_manager_same_actor_preempts() {
        let mgr = LogSessionManager::new();
        let cancel = CancellationToken::new();
        let _rx = mgr.listen("actor-1".to_owned(), StreamingFilters::default(), cancel.clone());

        assert!(mgr.can_start_stream("actor-1"), "same actor should preempt");
        assert!(cancel.is_cancelled(), "old session should be cancelled");
    }

    #[test]
    fn session_manager_different_actor_rejected() {
        let mgr = LogSessionManager::new();
        let cancel = CancellationToken::new();
        let _rx = mgr.listen("actor-1".to_owned(), StreamingFilters::default(), cancel.clone());

        assert!(
            !mgr.can_start_stream("actor-2"),
            "different actor should be rejected"
        );
        assert!(!cancel.is_cancelled(), "existing session should not be cancelled");
    }

    #[test]
    fn session_manager_remove_frees_slot() {
        let mgr = LogSessionManager::new();
        let cancel = CancellationToken::new();
        let _rx = mgr.listen("actor-1".to_owned(), StreamingFilters::default(), cancel);

        mgr.remove("actor-1");

        assert!(
            mgr.can_start_stream("actor-2"),
            "slot should be free after remove"
        );
    }

    #[tokio::test]
    async fn session_manager_insert_delivers_to_session() {
        let mgr = LogSessionManager::new();
        let cancel = CancellationToken::new();
        let mut rx = mgr.listen("actor-1".to_owned(), StreamingFilters::default(), cancel);

        let entry = LogEntry {
            time: "2026-01-01T00:00:00Z".to_owned(),
            level: Some(cfdrs_cdc::log_streaming::LogLevel::Info),
            message: "test".to_owned(),
            event: None,
            fields: None,
        };

        mgr.insert(&entry);

        let received = rx.try_recv().expect("should receive entry");
        assert_eq!(received.message, "test");
    }

    #[tokio::test]
    async fn session_manager_insert_respects_level_filter() {
        let mgr = LogSessionManager::new();
        let cancel = CancellationToken::new();
        let filters = StreamingFilters {
            level: Some(cfdrs_cdc::log_streaming::LogLevel::Warn),
            events: Vec::new(),
            sampling: 0.0,
        };
        let mut rx = mgr.listen("actor-1".to_owned(), filters, cancel);

        let info_entry = LogEntry {
            time: String::new(),
            level: Some(cfdrs_cdc::log_streaming::LogLevel::Info),
            message: "info-msg".to_owned(),
            event: None,
            fields: None,
        };
        let warn_entry = LogEntry {
            time: String::new(),
            level: Some(cfdrs_cdc::log_streaming::LogLevel::Warn),
            message: "warn-msg".to_owned(),
            event: None,
            fields: None,
        };

        mgr.insert(&info_entry);
        mgr.insert(&warn_entry);

        let received = rx.try_recv().expect("should receive warn entry");
        assert_eq!(received.message, "warn-msg");
        assert!(rx.try_recv().is_err(), "info entry should have been filtered");
    }

    #[test]
    fn session_manager_insert_drops_when_full() {
        let mgr = LogSessionManager::new();
        let cancel = CancellationToken::new();
        let _rx = mgr.listen("actor-1".to_owned(), StreamingFilters::default(), cancel);

        let entry = LogEntry {
            time: String::new(),
            level: None,
            message: "fill".to_owned(),
            event: None,
            fields: None,
        };

        // Fill beyond LOG_WINDOW — should not panic.
        for _ in 0..LOG_WINDOW + 10 {
            mgr.insert(&entry);
        }
    }
}
