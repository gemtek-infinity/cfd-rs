//! Phase 3.4a–c + 3.5 + 4.1 + 5.1: Pingora proxy-layer seam with lifecycle
//! participation, origin service dispatch, wire/protocol bridge reception,
//! incoming request stream handling, and owner-scoped operability reporting.
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
//! 4.1 admitted: reports proxy admission, observed registration, bridge
//!     closure, and shutdown acknowledgement through the runtime-owned
//!     operability surface.
//! 5.1 admitted: broader origin service dispatch (HelloWorld, Http origin
//!     routing wired, unimplemented services reported honestly), incoming
//!     stream handling via ConnectRequest dispatch through ingress matching.

use cfdrs_cdc::stream::ConnectRequest;
use cfdrs_shared::{IngressRule, IngressService, find_matching_rule};
use pingora_http::{RequestHeader, ResponseHeader};
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::protocol::{
    ProtocolBridgeState, ProtocolEvent, ProtocolReceiver, StreamResponse, StreamResponseSender,
};
use crate::runtime::{ChildTask, RuntimeCommand};

pub(crate) mod origin;

pub(crate) const PROXY_SEAM_NAME: &str = "pingora-proxy-seam";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProxySeamState {
    Admitted,
    RegistrationObserved,
    ShutdownAcknowledged,
}

impl ProxySeamState {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::RegistrationObserved => "registration-observed",
            Self::ShutdownAcknowledged => "shutdown-acknowledged",
        }
    }
}

impl std::fmt::Display for ProxySeamState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

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

    /// Handle an incoming ConnectRequest through ingress-routed dispatch.
    ///
    /// This is the broader Phase 5.1 entry point from the stream handler
    /// into the proxy. Routes the request through ingress matching and
    /// dispatches to the matched origin service.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn handle_connect_request(&self, request: &ConnectRequest) -> origin::OriginResponse {
        origin::proxy_connect_request(&self.ingress, request)
    }

    /// Spawn the proxy seam as a runtime-owned lifecycle participant.
    ///
    /// Reports the admitted origin/proxy path and ingress rule count at
    /// startup. When a protocol bridge is provided, waits for
    /// registration events and incoming stream requests from the
    /// transport layer, dispatches them through ingress matching, and
    /// reports owned proxy/protocol visibility before shutdown.
    pub(crate) fn spawn(
        self,
        command_tx: mpsc::Sender<RuntimeCommand>,
        protocol_rx: Option<ProtocolReceiver>,
        stream_response_tx: Option<StreamResponseSender>,
        shutdown: CancellationToken,
        child_tasks: &mut JoinSet<ChildTask>,
    ) {
        let ingress_count = self.ingress_count();

        child_tasks.spawn(async move {
            let _ = command_tx
                .send(RuntimeCommand::ProxyState {
                    state: ProxySeamState::Admitted,
                    detail: format!("ingress-rules={ingress_count}"),
                })
                .await;
            let _ = command_tx
                .send(RuntimeCommand::ServiceStatus {
                    service: PROXY_SEAM_NAME,
                    detail: format!(
                        "origin-proxy-admitted: broader dispatch active, ingress-rules={ingress_count}"
                    ),
                })
                .await;

            if let Some(rx) = protocol_rx {
                handle_protocol_bridge(&self, rx, stream_response_tx.as_ref(), &command_tx, &shutdown).await;
            } else {
                shutdown.cancelled().await;
            }

            let _ = command_tx
                .send(RuntimeCommand::ProxyState {
                    state: ProxySeamState::ShutdownAcknowledged,
                    detail: String::from("proxy seam acknowledged runtime shutdown"),
                })
                .await;
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

/// Handle protocol bridge events until shutdown or bridge closure.
async fn handle_protocol_bridge(
    seam: &PingoraProxySeam,
    mut rx: ProtocolReceiver,
    stream_response_tx: Option<&StreamResponseSender>,
    command_tx: &mpsc::Sender<RuntimeCommand>,
    shutdown: &CancellationToken,
) {
    let mut registration_observed = false;
    let mut streams_dispatched: u64 = 0;

    loop {
        tokio::select! {
            biased;
            event = rx.recv() => {
                match event {
                    Some(ProtocolEvent::Registered { peer }) => {
                        registration_observed = true;
                        send_registration_observed(&peer, command_tx).await;
                    }
                    Some(ProtocolEvent::IncomingStream { stream_id, request }) => {
                        let response = seam.handle_connect_request(&request);

                        if let Some(tx) = stream_response_tx {
                            let connect_response = origin::to_connect_response(&response);
                            let data =
                                cfdrs_cdc::stream_codec::encode_connect_response(
                                    &connect_response,
                                );
                            tx.send(StreamResponse { stream_id, data });
                        }

                        streams_dispatched += 1;
                        send_stream_dispatched(
                            stream_id,
                            &request,
                            &response,
                            streams_dispatched,
                            command_tx,
                        ).await;
                    }
                    Some(ProtocolEvent::RegistrationComplete { conn_uuid, location }) => {
                        let _ = command_tx
                            .send(RuntimeCommand::ServiceStatus {
                                service: PROXY_SEAM_NAME,
                                detail: format!(
                                    "registration-complete: uuid={conn_uuid} location={location}"
                                ),
                            })
                            .await;
                    }
                    Some(ProtocolEvent::Unregistering { conn_index }) => {
                        let _ = command_tx
                            .send(RuntimeCommand::ServiceStatus {
                                service: PROXY_SEAM_NAME,
                                detail: format!("unregistering: conn_index={conn_index}"),
                            })
                            .await;
                    }
                    Some(ProtocolEvent::Disconnected { conn_index }) => {
                        let _ = command_tx
                            .send(RuntimeCommand::ServiceStatus {
                                service: PROXY_SEAM_NAME,
                                detail: format!("disconnected: conn_index={conn_index}"),
                            })
                            .await;
                    }
                    Some(ProtocolEvent::ConfigPushed { conn_index }) => {
                        let _ = command_tx
                            .send(RuntimeCommand::ServiceStatus {
                                service: PROXY_SEAM_NAME,
                                detail: format!("config-pushed: conn_index={conn_index}"),
                            })
                            .await;
                    }
                    None => {
                        send_bridge_closed(registration_observed, command_tx).await;
                        break;
                    }
                }
            }
            _ = shutdown.cancelled() => break,
        }
    }
}

async fn send_registration_observed(peer: &SocketAddr, command_tx: &mpsc::Sender<RuntimeCommand>) {
    let _ = command_tx
        .send(RuntimeCommand::ProtocolState {
            state: ProtocolBridgeState::RegistrationObserved,
            detail: format!("proxy observed transport registration from {peer}"),
        })
        .await;
    let _ = command_tx
        .send(RuntimeCommand::ProxyState {
            state: ProxySeamState::RegistrationObserved,
            detail: format!("peer={peer}"),
        })
        .await;
    let _ = command_tx
        .send(RuntimeCommand::ServiceStatus {
            service: PROXY_SEAM_NAME,
            detail: format!("protocol-bridge: session registered, peer={peer}"),
        })
        .await;
}

async fn send_bridge_closed(registration_observed: bool, command_tx: &mpsc::Sender<RuntimeCommand>) {
    let detail = if registration_observed {
        String::from("proxy bridge closed after transport registration")
    } else {
        String::from("proxy bridge closed before transport registration")
    };
    let _ = command_tx
        .send(RuntimeCommand::ProtocolState {
            state: ProtocolBridgeState::BridgeClosed,
            detail: detail.clone(),
        })
        .await;
    let _ = command_tx
        .send(RuntimeCommand::ServiceStatus {
            service: PROXY_SEAM_NAME,
            detail: format!("protocol-bridge: {detail}"),
        })
        .await;
}

async fn send_stream_dispatched(
    stream_id: u64,
    request: &ConnectRequest,
    response: &origin::OriginResponse,
    total_dispatched: u64,
    command_tx: &mpsc::Sender<RuntimeCommand>,
) {
    let response_label = match response {
        origin::OriginResponse::Http(header) => {
            format!("http-{}", header.status.as_u16())
        }
        origin::OriginResponse::StreamEstablished => "stream-established".to_owned(),
        origin::OriginResponse::Unimplemented { service_label } => {
            format!("unimplemented:{service_label}")
        }
    };

    let _ = command_tx
        .send(RuntimeCommand::ServiceStatus {
            service: PROXY_SEAM_NAME,
            detail: format!(
                "stream-dispatch: stream={stream_id} type={} dest={} result={response_label} \
                 total={total_dispatched}",
                request.connection_type, request.dest,
            ),
        })
        .await;
}

/// Dispatch a request to the matched origin service.
///
/// Phase 3.4c path — retained for the `RequestHeader`-based entry point.
/// The Phase 5.1 `ConnectRequest`-based path uses `origin::dispatch_to_origin`
/// directly.
#[cfg_attr(not(test), allow(dead_code))]
fn dispatch_origin(service: &IngressService) -> ResponseHeader {
    match service {
        IngressService::HttpStatus(code) => origin::build_status_response(*code),
        IngressService::HelloWorld => {
            let mut header = ResponseHeader::build(200, None).expect("200 is always a valid status code");
            let _ = header.insert_header("Content-Type", "text/html; charset=utf-8");
            header
        }
        _ => origin::build_status_response(502),
    }
}

/// Build a response with the given HTTP status code.
///
/// Delegates to `origin::build_status_response` for consistency.
#[cfg_attr(not(test), allow(dead_code))]
fn build_status_response(code: u16) -> ResponseHeader {
    origin::build_status_response(code)
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
mod tests;
