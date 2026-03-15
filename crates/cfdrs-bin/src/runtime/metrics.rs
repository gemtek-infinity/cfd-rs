use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cfdrs_his::metrics_server::{
    self, BuildInfo, ConfigResponse, HEALTHCHECK_RESPONSE, PPROF_DEFERRED, READ_TIMEOUT_SECS,
    ReadinessResponse, WRITE_TIMEOUT_SECS,
};
use serde_json::json;

use super::RuntimeConfig;
use super::state::RuntimeStatus;

/// Maximum HTTP request buffer size for the metrics server.
const HTTP_REQUEST_BUFFER_SIZE: usize = 2048;

#[derive(Debug)]
struct MetricsSnapshot {
    connector_id: uuid::Uuid,
    ready_connections: u32,
    build_info: BuildInfo,
    config_response: ConfigResponse,
    diagnostic_configuration: BTreeMap<String, String>,
}

pub(super) struct RuntimeMetricsHandle {
    actual_address: SocketAddr,
    snapshot: Arc<Mutex<MetricsSnapshot>>,
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl RuntimeMetricsHandle {
    pub(super) fn start(config: &RuntimeConfig) -> Result<Self, String> {
        let listener = bind_metrics_listener(config.metrics_bind_address(), config.is_container_runtime())?;
        let actual_address = listener
            .local_addr()
            .map_err(|error| format!("metrics listener local address unavailable: {error}"))?;

        listener
            .set_nonblocking(true)
            .map_err(|error| format!("metrics listener nonblocking setup failed: {error}"))?;

        let snapshot = Arc::new(Mutex::new(MetricsSnapshot {
            connector_id: config.connector_id(),
            ready_connections: 0,
            build_info: runtime_build_info(),
            config_response: runtime_config_response(config),
            diagnostic_configuration: config.diagnostic_configuration().clone(),
        }));
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_snapshot = Arc::clone(&snapshot);
        let thread_shutdown = Arc::clone(&shutdown);

        let thread = thread::spawn(move || run_metrics_server(listener, thread_snapshot, thread_shutdown));

        Ok(Self {
            actual_address,
            snapshot,
            shutdown,
            thread: Some(thread),
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

        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.ready_connections = ready_connections;
        }
    }

    pub(super) fn stop(mut self) {
        self.shutdown.store(true, Ordering::Relaxed);

        let _ = TcpStream::connect(self.actual_address).and_then(|stream| stream.shutdown(Shutdown::Both));

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn bind_metrics_listener(
    explicit_bind: Option<SocketAddr>,
    is_container: bool,
) -> Result<TcpListener, String> {
    let mut last_error = None;

    for candidate in metrics_candidates(explicit_bind, is_container) {
        match TcpListener::bind(candidate) {
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

fn run_metrics_server(
    listener: TcpListener,
    snapshot: Arc<Mutex<MetricsSnapshot>>,
    shutdown: Arc<AtomicBool>,
) {
    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => handle_connection(stream, &snapshot),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(25));
            }
            Err(_) => break,
        }
    }
}

fn handle_connection(mut stream: TcpStream, snapshot: &Arc<Mutex<MetricsSnapshot>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(READ_TIMEOUT_SECS)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(WRITE_TIMEOUT_SECS)));

    let mut buffer = [0_u8; HTTP_REQUEST_BUFFER_SIZE];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(bytes_read) if bytes_read > 0 => bytes_read,
        _ => return,
    };

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let (method, path) = match parse_request_line(&request) {
        Some(parsed) => parsed,
        None => {
            let _ = write_response(&mut stream, 400, "text/plain", b"bad request\n", false);
            return;
        }
    };

    let head_only = method == "HEAD";
    let response = build_response(path, snapshot);
    let _ = write_response(
        &mut stream,
        response.status,
        response.content_type,
        response.body.as_bytes(),
        head_only,
    );
}

fn parse_request_line(request: &str) -> Option<(&str, &str)> {
    let mut parts = request.lines().next()?.split_whitespace();
    let method = parts.next()?;
    let path = parts.next()?;

    Some((method, path))
}

struct HttpResponse {
    status: u16,
    content_type: &'static str,
    body: String,
}

fn build_response(path: &str, snapshot: &Arc<Mutex<MetricsSnapshot>>) -> HttpResponse {
    let snapshot = match snapshot.lock() {
        Ok(guard) => guard,
        Err(_) => return internal_error("metrics snapshot lock poisoned"),
    };

    match path {
        "/ready" => {
            let response = ReadinessResponse::new(snapshot.connector_id, snapshot.ready_connections);

            match serde_json::to_string(&response) {
                Ok(body) => HttpResponse {
                    status: response.http_status(),
                    content_type: "application/json",
                    body,
                },
                Err(_) => internal_error("readiness response serialization failed"),
            }
        }
        "/healthcheck" => HttpResponse {
            status: 200,
            content_type: "text/plain; charset=utf-8",
            body: HEALTHCHECK_RESPONSE.to_owned(),
        },
        "/metrics" => HttpResponse {
            status: 200,
            content_type: "text/plain; version=0.0.4",
            body: prometheus_body(&snapshot),
        },
        "/config" => match serde_json::to_string(&snapshot.config_response) {
            Ok(body) => HttpResponse {
                status: 200,
                content_type: "application/json",
                body,
            },
            Err(_) => internal_error("config response serialization failed"),
        },
        "/diag/configuration" => match serde_json::to_string(&snapshot.diagnostic_configuration) {
            Ok(body) => HttpResponse {
                status: 200,
                content_type: "application/json",
                body,
            },
            Err(_) => internal_error("diagnostic configuration serialization failed"),
        },
        path if path.starts_with("/debug/pprof") && PPROF_DEFERRED => HttpResponse {
            status: 501,
            content_type: "text/plain; charset=utf-8",
            body: "pprof deferred\n".to_owned(),
        },
        _ => HttpResponse {
            status: 404,
            content_type: "text/plain; charset=utf-8",
            body: "not found\n".to_owned(),
        },
    }
}

fn internal_error(message: &str) -> HttpResponse {
    HttpResponse {
        status: 500,
        content_type: "text/plain; charset=utf-8",
        body: format!("{message}\n"),
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => "OK",
    };

    let headers = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: \
         close\r\n\r\n",
        body.len()
    );

    stream.write_all(headers.as_bytes())?;

    if !head_only {
        stream.write_all(body)?;
    }

    stream.flush()
}

fn prometheus_body(snapshot: &MetricsSnapshot) -> String {
    format!(
        "# HELP build_info Build information.\n# TYPE build_info \
         gauge\nbuild_info{{goversion=\"{}\",type=\"{}\",revision=\"{}\",version=\"{}\"}} 1\n# HELP \
         cfdrs_ready_connections Ready tunnel connections.\n# TYPE cfdrs_ready_connections \
         gauge\ncfdrs_ready_connections {}\n",
        snapshot.build_info.goversion,
        snapshot.build_info.build_type,
        snapshot.build_info.revision,
        snapshot.build_info.version,
        snapshot.ready_connections
    )
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
    use super::*;

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

    #[test]
    fn readiness_endpoint_reflects_snapshot() {
        let snapshot = Arc::new(Mutex::new(MetricsSnapshot {
            connector_id: uuid::Uuid::nil(),
            ready_connections: 1,
            build_info: runtime_build_info(),
            config_response: ConfigResponse {
                version: 1,
                config: json!({}),
            },
            diagnostic_configuration: BTreeMap::new(),
        }));

        let response = build_response("/ready", &snapshot);

        assert_eq!(response.status, 200);
        assert!(response.body.contains("\"readyConnections\":1"));
    }

    #[test]
    fn metrics_endpoint_emits_build_info_metric() {
        let snapshot = Arc::new(Mutex::new(MetricsSnapshot {
            connector_id: uuid::Uuid::nil(),
            ready_connections: 0,
            build_info: runtime_build_info(),
            config_response: ConfigResponse {
                version: 1,
                config: json!({}),
            },
            diagnostic_configuration: BTreeMap::new(),
        }));

        let response = build_response("/metrics", &snapshot);

        assert_eq!(response.status, 200);
        assert!(response.body.contains("build_info{"));
        assert!(response.body.contains("cfdrs_ready_connections 0"));
    }

    #[test]
    fn config_endpoint_serializes_runtime_shape() {
        let snapshot = Arc::new(Mutex::new(MetricsSnapshot {
            connector_id: uuid::Uuid::nil(),
            ready_connections: 0,
            build_info: runtime_build_info(),
            config_response: ConfigResponse {
                version: 7,
                config: json!({"ingress": [], "warp-routing": {}, "originRequest": {}}),
            },
            diagnostic_configuration: BTreeMap::new(),
        }));

        let response = build_response("/config", &snapshot);

        assert_eq!(response.status, 200);
        assert!(response.body.contains("\"version\":7"));
        assert!(response.body.contains("\"originRequest\""));
    }

    #[test]
    fn diagnostic_configuration_endpoint_serializes_uid_and_logs() {
        let snapshot = Arc::new(Mutex::new(MetricsSnapshot {
            connector_id: uuid::Uuid::nil(),
            ready_connections: 0,
            build_info: runtime_build_info(),
            config_response: ConfigResponse {
                version: 1,
                config: json!({}),
            },
            diagnostic_configuration: BTreeMap::from([
                ("uid".to_owned(), "1000".to_owned()),
                ("log_directory".to_owned(), "/var/log/cloudflared".to_owned()),
            ]),
        }));

        let response = build_response("/diag/configuration", &snapshot);

        assert_eq!(response.status, 200);
        assert!(response.body.contains("\"uid\":\"1000\""));
        assert!(
            response
                .body
                .contains("\"log_directory\":\"/var/log/cloudflared\"")
        );
    }

    #[test]
    fn healthcheck_endpoint_returns_ok_text_plain() {
        let snapshot = Arc::new(Mutex::new(MetricsSnapshot {
            connector_id: uuid::Uuid::nil(),
            ready_connections: 0,
            build_info: runtime_build_info(),
            config_response: ConfigResponse {
                version: 1,
                config: json!({}),
            },
            diagnostic_configuration: BTreeMap::new(),
        }));

        let response = build_response("/healthcheck", &snapshot);

        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "text/plain; charset=utf-8");
        assert_eq!(response.body, "OK\n");
    }

    #[test]
    fn pprof_routes_report_deferred_boundary() {
        let snapshot = Arc::new(Mutex::new(MetricsSnapshot {
            connector_id: uuid::Uuid::nil(),
            ready_connections: 0,
            build_info: runtime_build_info(),
            config_response: ConfigResponse {
                version: 1,
                config: json!({}),
            },
            diagnostic_configuration: BTreeMap::new(),
        }));

        let response = build_response("/debug/pprof/goroutine", &snapshot);

        assert_eq!(response.status, 501);
        assert_eq!(response.body, "pprof deferred\n");
    }
}
