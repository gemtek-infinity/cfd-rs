//! Phase 3.4a–c + 3.5: Pingora proxy-layer seam with lifecycle participation,
//! first admitted origin/proxy path, and wire/protocol bridge reception.
//!
//! This module is the owned entry point for Pingora in the production-alpha
//! path. All direct Pingora types and API usage are confined here. The rest
//! of the binary does not depend on Pingora crates directly.
//!
//! ADR-0003 governs Pingora scope: application-layer proxy above the quiche
//! transport lane, not a transport replacement.
//!
//! 3.4a admitted: dependency path and seam location.
//! 3.4b admitted: runtime lifecycle participation (startup/shutdown).
//! 3.4c admitted: first origin/proxy path (http_status ingress routing).
//! 3.5 admitted: receives protocol registration events from the transport
//!     layer through the explicit wire/protocol bridge.

use cloudflared_config::{IngressRule, IngressService, find_matching_rule};
use pingora_http::{RequestHeader, ResponseHeader};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::protocol::{ProtocolEvent, ProtocolReceiver};
use crate::runtime::{ChildTask, RuntimeCommand};

pub(crate) const PROXY_SEAM_NAME: &str = "pingora-proxy-seam";

/// Owned boundary for the Pingora proxy layer.
///
/// Confines the Pingora dependency surface to this module. Holds ingress
/// rules from the runtime handoff and routes HTTP requests to origin
/// services through the admitted path.
///
/// The first admitted origin path is `http_status:NNN`. Other origin
/// service types return 502 until later slices implement real origin
/// connections.
pub(crate) struct PingoraProxySeam {
    ingress: Vec<IngressRule>,
}

impl PingoraProxySeam {
    /// Create the proxy seam with ingress rules from the runtime handoff.
    pub(crate) fn new(ingress: Vec<IngressRule>) -> Self {
        Self { ingress }
    }

    /// Number of ingress rules held by this seam.
    pub(crate) fn ingress_count(&self) -> usize {
        self.ingress.len()
    }

    /// Handle an HTTP request through the ingress-routed origin path.
    ///
    /// Matches the request against ingress rules and dispatches to the
    /// origin service. For `HttpStatus(code)`, returns a response with that
    /// status code. For origin services not yet implemented, returns 502.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn handle_request(&self, request: &RequestHeader) -> ResponseHeader {
        let host = self::extract_request_host(request);
        let path = request.uri.path();

        let matched_rule = find_matching_rule(&self.ingress, host, path).map(|index| &self.ingress[index]);

        match matched_rule {
            Some(rule) => self::dispatch_origin(&rule.service),
            None => self::build_status_response(502),
        }
    }

    /// Spawn the proxy seam as a runtime-owned lifecycle participant.
    ///
    /// Reports the admitted origin/proxy path and ingress rule count at
    /// startup. When a protocol bridge is provided, waits for
    /// registration events from the transport layer before shutdown.
    pub(crate) fn spawn(
        self,
        command_tx: mpsc::Sender<RuntimeCommand>,
        protocol_rx: Option<ProtocolReceiver>,
        shutdown: CancellationToken,
        child_tasks: &mut JoinSet<ChildTask>,
    ) {
        let ingress_count = self.ingress_count();

        child_tasks.spawn(async move {
            let _ = command_tx
                .send(RuntimeCommand::ServiceStatus {
                    service: PROXY_SEAM_NAME,
                    detail: format!(
                        "origin-proxy-admitted: http_status path active, ingress-rules={ingress_count}"
                    ),
                })
                .await;

            if let Some(mut rx) = protocol_rx {
                // Wait for protocol registration from the transport
                // layer, or shutdown, whichever comes first.
                // Biased: prefer processing a protocol event that
                // arrived just before shutdown over missing it.
                loop {
                    tokio::select! {
                        biased;
                        event = rx.recv() => {
                            match event {
                                Some(ProtocolEvent::Registered { peer }) => {
                                    let _ = command_tx
                                        .send(RuntimeCommand::ServiceStatus {
                                            service: PROXY_SEAM_NAME,
                                            detail: format!(
                                                "protocol-bridge: session registered, peer={peer}"
                                            ),
                                        })
                                        .await;
                                }
                                None => break,
                            }
                        }
                        _ = shutdown.cancelled() => break,
                    }
                }
            } else {
                shutdown.cancelled().await;
            }

            let _ = command_tx
                .send(RuntimeCommand::ServiceStatus {
                    service: PROXY_SEAM_NAME,
                    detail: "lifecycle-exit: shutdown acknowledged".to_owned(),
                })
                .await;

            ChildTask::ProxySeam
        });
    }
}

/// Dispatch a request to the matched origin service.
#[cfg_attr(not(test), allow(dead_code))]
fn dispatch_origin(service: &IngressService) -> ResponseHeader {
    match service {
        IngressService::HttpStatus(code) => self::build_status_response(*code),
        _ => self::build_status_response(502),
    }
}

/// Build a response with the given HTTP status code.
///
/// Status codes from config-validated ingress rules are guaranteed to be
/// in 100–999. Hardcoded codes (like 502) are valid by construction.
#[cfg_attr(not(test), allow(dead_code))]
fn build_status_response(code: u16) -> ResponseHeader {
    ResponseHeader::build(code, None)
        .expect("status codes from validated config or hardcoded constants are always valid")
}

/// Extract the request host for ingress matching.
///
/// Checks the URI authority first (for absolute-form requests), then
/// falls back to the Host header.
#[cfg_attr(not(test), allow(dead_code))]
fn extract_request_host(request: &RequestHeader) -> &str {
    if let Some(host) = request.uri.host() {
        return host;
    }

    request
        .headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloudflared_config::{IngressMatch, OriginRequestConfig};

    fn http_status_rule(hostname: Option<&str>, code: u16) -> IngressRule {
        IngressRule {
            matcher: IngressMatch {
                hostname: hostname.map(String::from),
                punycode_hostname: None,
                path: None,
            },
            service: IngressService::HttpStatus(code),
            origin_request: OriginRequestConfig::default(),
        }
    }

    fn catch_all_rule(code: u16) -> IngressRule {
        self::http_status_rule(None, code)
    }

    fn build_request(method: &str, path: &[u8], host: Option<&str>) -> RequestHeader {
        let mut request = RequestHeader::build(method, path, None).expect("test request should build");
        if let Some(host_value) = host {
            request
                .insert_header("host", host_value)
                .expect("test host header should insert");
        }
        request
    }

    // -- Dependency admission (preserved from 3.4a) --

    #[test]
    fn pingora_http_request_type_admitted() {
        let header = RequestHeader::build("GET", b"/", None);
        assert!(
            header.is_ok(),
            "Pingora HTTP request type should build at the admitted seam"
        );
    }

    // -- Origin/proxy path tests (3.4c) --

    #[test]
    fn handle_request_returns_http_status_from_catch_all() {
        let seam = PingoraProxySeam::new(vec![catch_all_rule(503)]);
        let request = build_request("GET", b"/", None);
        let response = seam.handle_request(&request);
        assert_eq!(response.status.as_u16(), 503);
    }

    #[test]
    fn handle_request_matches_hostname_to_origin() {
        let seam = PingoraProxySeam::new(vec![
            http_status_rule(Some("example.com"), 200),
            catch_all_rule(404),
        ]);
        let request = build_request("GET", b"/", Some("example.com"));
        let response = seam.handle_request(&request);
        assert_eq!(response.status.as_u16(), 200);
    }

    #[test]
    fn handle_request_falls_through_to_catch_all() {
        let seam = PingoraProxySeam::new(vec![
            http_status_rule(Some("example.com"), 200),
            catch_all_rule(404),
        ]);
        let request = build_request("GET", b"/", Some("other.com"));
        let response = seam.handle_request(&request);
        assert_eq!(response.status.as_u16(), 404);
    }

    #[test]
    fn handle_request_returns_502_for_empty_ingress() {
        let seam = PingoraProxySeam::new(vec![]);
        let request = build_request("GET", b"/", None);
        let response = seam.handle_request(&request);
        assert_eq!(response.status.as_u16(), 502);
    }

    #[test]
    fn handle_request_returns_502_for_unimplemented_origin() {
        let seam = PingoraProxySeam::new(vec![IngressRule {
            matcher: IngressMatch::default(),
            service: IngressService::HelloWorld,
            origin_request: OriginRequestConfig::default(),
        }]);
        let request = build_request("GET", b"/", None);
        let response = seam.handle_request(&request);
        assert_eq!(response.status.as_u16(), 502);
    }

    #[test]
    fn ingress_count_reflects_handoff_rules() {
        let seam = PingoraProxySeam::new(vec![
            http_status_rule(Some("a.example.com"), 200),
            http_status_rule(Some("b.example.com"), 201),
            catch_all_rule(503),
        ]);
        assert_eq!(seam.ingress_count(), 3);
    }

    // -- Lifecycle participation (evolved from 3.4b) --

    #[tokio::test]
    async fn proxy_seam_reports_origin_path_and_shuts_down() {
        let (command_tx, mut command_rx) = mpsc::channel(16);
        let shutdown = CancellationToken::new();
        let mut child_tasks = JoinSet::new();

        let seam = PingoraProxySeam::new(vec![catch_all_rule(503)]);
        seam.spawn(command_tx, None, shutdown.clone(), &mut child_tasks);

        // Seam should report the admitted origin/proxy path on startup.
        let msg = command_rx.recv().await.expect("should receive origin status");
        match msg {
            RuntimeCommand::ServiceStatus { service, detail } => {
                assert_eq!(service, PROXY_SEAM_NAME);
                assert!(
                    detail.contains("origin-proxy-admitted"),
                    "startup status should report admitted origin path, got: {detail}"
                );
                assert!(
                    detail.contains("ingress-rules=1"),
                    "startup status should report ingress rule count, got: {detail}"
                );
            }
            other => panic!("expected ServiceStatus for origin admission, got: {other:?}"),
        }

        shutdown.cancel();

        let msg = command_rx.recv().await.expect("should receive shutdown status");
        match msg {
            RuntimeCommand::ServiceStatus { service, detail } => {
                assert_eq!(service, PROXY_SEAM_NAME);
                assert!(detail.contains("shutdown acknowledged"));
            }
            other => panic!("expected ServiceStatus for shutdown exit, got: {other:?}"),
        }

        let result = child_tasks.join_next().await;
        assert!(result.is_some(), "proxy seam child task should complete");
        match result
            .expect("join should succeed")
            .expect("task should not panic")
        {
            ChildTask::ProxySeam => {}
            other => panic!("expected ChildTask::ProxySeam, got: {other:?}"),
        }
    }

    // -- Wire/protocol bridge (3.5) --

    #[tokio::test]
    async fn proxy_seam_receives_protocol_registration() {
        let (command_tx, mut command_rx) = mpsc::channel(16);
        let (protocol_sender, protocol_receiver) = crate::protocol::protocol_bridge();
        let shutdown = CancellationToken::new();
        let mut child_tasks = JoinSet::new();

        let seam = PingoraProxySeam::new(vec![catch_all_rule(503)]);
        seam.spawn(
            command_tx,
            Some(protocol_receiver),
            shutdown.clone(),
            &mut child_tasks,
        );

        // Startup status.
        let msg = command_rx.recv().await.expect("should receive startup status");
        assert!(matches!(msg, RuntimeCommand::ServiceStatus { .. }));

        // Simulate transport sending registration event.
        protocol_sender
            .send(ProtocolEvent::Registered {
                peer: "127.0.0.1:7844".to_owned(),
            })
            .await;

        // Proxy should report the protocol bridge registration.
        let msg = command_rx
            .recv()
            .await
            .expect("should receive protocol bridge status");
        match msg {
            RuntimeCommand::ServiceStatus { service, detail } => {
                assert_eq!(service, PROXY_SEAM_NAME);
                assert!(
                    detail.contains("protocol-bridge: session registered"),
                    "expected protocol bridge registration, got: {detail}"
                );
                assert!(
                    detail.contains("peer=127.0.0.1:7844"),
                    "expected peer address, got: {detail}"
                );
            }
            other => panic!("expected ServiceStatus for protocol bridge, got: {other:?}"),
        }

        shutdown.cancel();

        let msg = command_rx.recv().await.expect("should receive shutdown status");
        match msg {
            RuntimeCommand::ServiceStatus { service, detail } => {
                assert_eq!(service, PROXY_SEAM_NAME);
                assert!(detail.contains("shutdown acknowledged"));
            }
            other => panic!("expected ServiceStatus for shutdown, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn proxy_seam_handles_bridge_closure_without_registration() {
        let (command_tx, mut command_rx) = mpsc::channel(16);
        let (protocol_sender, protocol_receiver) = crate::protocol::protocol_bridge();
        let shutdown = CancellationToken::new();
        let mut child_tasks = JoinSet::new();

        let seam = PingoraProxySeam::new(vec![catch_all_rule(503)]);
        seam.spawn(
            command_tx,
            Some(protocol_receiver),
            shutdown.clone(),
            &mut child_tasks,
        );

        // Startup status.
        let _ = command_rx.recv().await;

        // Drop sender without sending registration — simulates
        // transport failure before reaching the protocol boundary.
        drop(protocol_sender);

        // Proxy should still exit cleanly after bridge closure.
        let msg = command_rx
            .recv()
            .await
            .expect("should receive shutdown status after bridge closure");
        match msg {
            RuntimeCommand::ServiceStatus { service, detail } => {
                assert_eq!(service, PROXY_SEAM_NAME);
                assert!(
                    detail.contains("shutdown acknowledged"),
                    "expected shutdown ack after bridge closure, got: {detail}"
                );
            }
            other => panic!("expected ServiceStatus for shutdown, got: {other:?}"),
        }
    }
}
