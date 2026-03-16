use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use cfdrs_his::metrics_server::{
    self, BuildInfo, ConfigResponse, HEALTHCHECK_RESPONSE, PPROF_DEFERRED, ReadinessResponse,
};
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use super::RuntimeConfig;
use super::state::RuntimeStatus;

#[derive(Debug)]
struct MetricsSnapshot {
    connector_id: uuid::Uuid,
    ready_connections: u32,
    config_response: ConfigResponse,
    diagnostic_configuration: BTreeMap<String, String>,
}

struct AppState {
    snapshot: RwLock<MetricsSnapshot>,
    registry: RwLock<Registry>,
    ready_connections_gauge: Gauge,
}

pub(super) struct RuntimeMetricsHandle {
    actual_address: SocketAddr,
    state: Arc<AppState>,
    shutdown_tx: tokio::sync::watch::Sender<()>,
    task: Option<JoinHandle<()>>,
}

impl RuntimeMetricsHandle {
    pub(super) async fn start(config: &RuntimeConfig) -> Result<Self, String> {
        let listener =
            bind_metrics_listener(config.metrics_bind_address(), config.is_container_runtime()).await?;
        let actual_address = listener
            .local_addr()
            .map_err(|error| format!("metrics listener local address unavailable: {error}"))?;

        let ready_connections_gauge = Gauge::<i64, _>::default();
        let mut registry = Registry::default();
        let build_info = runtime_build_info();

        registry.register("build_info", "Build information", {
            let family =
                prometheus_client::metrics::family::Family::<Vec<(String, String)>, Gauge>::default();
            family
                .get_or_create(&vec![
                    ("goversion".to_owned(), build_info.goversion.to_owned()),
                    ("type".to_owned(), build_info.build_type.to_owned()),
                    ("revision".to_owned(), build_info.revision.to_owned()),
                    ("version".to_owned(), build_info.version.to_owned()),
                ])
                .set(1);
            family
        });
        registry.register(
            "cfdrs_ready_connections",
            "Ready tunnel connections",
            ready_connections_gauge.clone(),
        );

        let state = Arc::new(AppState {
            snapshot: RwLock::new(MetricsSnapshot {
                connector_id: config.connector_id(),
                ready_connections: 0,
                config_response: runtime_config_response(config),
                diagnostic_configuration: config.diagnostic_configuration().clone(),
            }),
            registry: RwLock::new(registry),
            ready_connections_gauge,
        });

        let app = build_router(Arc::clone(&state));
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(());

        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.changed().await;
                })
                .await
                .ok();
        });

        Ok(Self {
            actual_address,
            state,
            shutdown_tx,
            task: Some(task),
        })
    }

    pub(super) fn actual_address(&self) -> SocketAddr {
        self.actual_address
    }

    pub(super) fn sync_from_status(&self, status: &RuntimeStatus) {
        // Use active_connections to match Go ConnTracker.CountActiveConns()
        // semantics: report the number of currently registered tunnel
        // connections, not a binary ready/not-ready flag.
        let ready_connections = status.active_connections;

        if let Ok(mut snapshot) = self.state.snapshot.try_write() {
            snapshot.ready_connections = ready_connections;
            self.state
                .ready_connections_gauge
                .set(i64::from(ready_connections));
        }
    }

    pub(super) async fn stop(mut self) {
        let _ = self.shutdown_tx.send(());

        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ready", get(handle_ready))
        .route("/healthcheck", get(handle_healthcheck))
        .route("/metrics", get(handle_metrics))
        .route("/config", get(handle_config))
        .route("/diag/configuration", get(handle_diag_configuration))
        .route("/debug/pprof/{*rest}", get(handle_pprof))
        .fallback(handle_not_found)
        .with_state(state)
}

async fn handle_ready(State(state): State<Arc<AppState>>) -> Response {
    let snapshot = state.snapshot.read().await;
    let response = ReadinessResponse::new(snapshot.connector_id, snapshot.ready_connections);

    match serde_json::to_string(&response) {
        Ok(body) => (
            StatusCode::from_u16(response.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            [(header::CONTENT_TYPE, "application/json")],
            body,
        )
            .into_response(),
        Err(_) => internal_error("readiness response serialization failed"),
    }
}

async fn handle_healthcheck() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        HEALTHCHECK_RESPONSE,
    )
        .into_response()
}

async fn handle_metrics(State(state): State<Arc<AppState>>) -> Response {
    let registry = state.registry.read().await;
    let mut body = String::new();

    if encode(&mut body, &registry).is_err() {
        return internal_error("metrics encoding failed");
    }

    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )],
        body,
    )
        .into_response()
}

async fn handle_config(State(state): State<Arc<AppState>>) -> Response {
    let snapshot = state.snapshot.read().await;

    match serde_json::to_string(&snapshot.config_response) {
        Ok(body) => (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], body).into_response(),
        Err(_) => internal_error("config response serialization failed"),
    }
}

async fn handle_diag_configuration(State(state): State<Arc<AppState>>) -> Response {
    let snapshot = state.snapshot.read().await;

    match serde_json::to_string(&snapshot.diagnostic_configuration) {
        Ok(body) => (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], body).into_response(),
        Err(_) => internal_error("diagnostic configuration serialization failed"),
    }
}

async fn handle_pprof() -> Response {
    if PPROF_DEFERRED {
        return (
            StatusCode::NOT_IMPLEMENTED,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "pprof deferred\n",
        )
            .into_response();
    }
    handle_not_found().await
}

async fn handle_not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        "not found\n",
    )
        .into_response()
}

fn internal_error(message: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        format!("{message}\n"),
    )
        .into_response()
}

async fn bind_metrics_listener(
    explicit_bind: Option<SocketAddr>,
    is_container: bool,
) -> Result<TcpListener, String> {
    let mut last_error = None;

    for candidate in metrics_candidates(explicit_bind, is_container) {
        match TcpListener::bind(candidate).await {
            Ok(listener) => return Ok(listener),
            Err(error) => last_error = Some(format!("{candidate}: {error}")),
        }
    }

    Err(format!(
        "error opening metrics server listener{}",
        last_error.map(|detail| format!(": {detail}")).unwrap_or_default()
    ))
}

fn metrics_candidates(explicit_bind: Option<SocketAddr>, is_container: bool) -> Vec<SocketAddr> {
    if let Some(address) = explicit_bind {
        return vec![address];
    }

    let mut candidates = Vec::with_capacity(1 + metrics_server::KNOWN_METRICS_PORTS.len());
    let default_address =
        metrics_server::parse_metrics_address(metrics_server::default_metrics_address(is_container))
            .expect("default metrics address should parse");

    candidates.push(default_address);
    candidates.extend(metrics_server::known_metrics_addresses(is_container));
    candidates
}

fn runtime_build_info() -> BuildInfo {
    BuildInfo {
        goversion: "rust",
        version: env!("CARGO_PKG_VERSION"),
        revision: option_env!("GITHUB_SHA").unwrap_or("unknown"),
        build_type: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
    }
}

fn runtime_config_response(config: &RuntimeConfig) -> ConfigResponse {
    ConfigResponse {
        version: 1,
        config: json!({
            "ingress": config.normalized().ingress,
            "warp-routing": config.normalized().warp_routing,
            "originRequest": config.normalized().origin_request,
        }),
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use super::*;

    fn test_state() -> Arc<AppState> {
        let ready_connections_gauge = Gauge::<i64, _>::default();
        let mut registry = Registry::default();
        registry.register(
            "cfdrs_ready_connections",
            "Ready tunnel connections",
            ready_connections_gauge.clone(),
        );

        Arc::new(AppState {
            snapshot: RwLock::new(MetricsSnapshot {
                connector_id: uuid::Uuid::nil(),
                ready_connections: 0,
                config_response: ConfigResponse {
                    version: 1,
                    config: json!({}),
                },
                diagnostic_configuration: BTreeMap::new(),
            }),
            registry: RwLock::new(registry),
            ready_connections_gauge,
        })
    }

    #[test]
    fn default_metrics_candidates_include_known_ports() {
        let candidates = metrics_candidates(None, false);
        assert!(candidates.iter().any(|candidate| candidate.port() == 0));
        assert!(candidates.iter().any(|candidate| candidate.port() == 20241));
    }

    #[test]
    fn container_metrics_candidates_bind_to_all_interfaces() {
        let candidates = metrics_candidates(None, true);

        assert!(candidates.iter().any(|candidate| candidate.port() == 0));

        for candidate in &candidates {
            assert!(
                candidate.ip().is_unspecified(),
                "container mode should bind to 0.0.0.0, got {candidate}"
            );
        }
    }

    #[tokio::test]
    async fn readiness_endpoint_reflects_snapshot() {
        let state = test_state();
        state.snapshot.write().await.ready_connections = 1;

        let app = build_router(state);
        let response = app
            .oneshot(Request::get("/ready").body(Body::empty()).expect("request"))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        assert!(String::from_utf8_lossy(&body).contains("\"readyConnections\":1"));
    }

    #[tokio::test]
    async fn metrics_endpoint_emits_prometheus_output() {
        let state = test_state();
        let app = build_router(state);

        let response = app
            .oneshot(Request::get("/metrics").body(Body::empty()).expect("request"))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 8192)
            .await
            .expect("body");
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("cfdrs_ready_connections"));
    }

    #[tokio::test]
    async fn config_endpoint_serializes_runtime_shape() {
        let state = test_state();
        state.snapshot.write().await.config_response = ConfigResponse {
            version: 7,
            config: json!({"ingress": [], "warp-routing": {}, "originRequest": {}}),
        };

        let app = build_router(state);
        let response = app
            .oneshot(Request::get("/config").body(Body::empty()).expect("request"))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("\"version\":7"));
        assert!(text.contains("\"originRequest\""));
    }

    #[tokio::test]
    async fn diagnostic_configuration_endpoint_serializes_uid_and_logs() {
        let state = test_state();
        {
            let mut snapshot = state.snapshot.write().await;
            snapshot.diagnostic_configuration = BTreeMap::from([
                ("uid".to_owned(), "1000".to_owned()),
                ("log_directory".to_owned(), "/var/log/cloudflared".to_owned()),
            ]);
        }

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::get("/diag/configuration")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("\"uid\":\"1000\""));
        assert!(text.contains("\"log_directory\":\"/var/log/cloudflared\""));
    }

    #[tokio::test]
    async fn healthcheck_endpoint_returns_ok_text_plain() {
        let state = test_state();
        let app = build_router(state);

        let response = app
            .oneshot(Request::get("/healthcheck").body(Body::empty()).expect("request"))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("content-type header");
        assert_eq!(content_type, "text/plain; charset=utf-8");
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        assert_eq!(&body[..], b"OK\n");
    }

    #[tokio::test]
    async fn pprof_routes_report_deferred_boundary() {
        let state = test_state();
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::get("/debug/pprof/goroutine")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        assert_eq!(&body[..], b"pprof deferred\n");
    }
}
