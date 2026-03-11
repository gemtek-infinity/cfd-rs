use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time;
use tokio_util::sync::CancellationToken;

use super::TransportLifecycleStage;
use crate::protocol::ProtocolBridgeState;
use crate::protocol::{CONTROL_STREAM_ID, ProtocolEvent, ProtocolSender};
use crate::runtime::{
    ChildTask, RuntimeCommand, RuntimeConfig, RuntimeService, RuntimeServiceFactory, ServiceExit,
};

#[path = "quic/edge.rs"]
mod edge;
#[path = "quic/identity.rs"]
mod identity;
#[path = "quic/session.rs"]
mod session;

#[cfg(test)]
#[path = "quic/tests.rs"]
mod tests;

use self::edge::{EDGE_DEFAULT_REGION, QuicEdgeTarget, resolve_edge_target};
use self::identity::TransportIdentity;
use self::session::{QuicSessionState, flush_egress};

const QUIC_ESTABLISH_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_DATAGRAM_SIZE: usize = 1350;
const WIRE_PROTOCOL_DEFERRED_DETAIL: &str = "wire/protocol boundary crossed (control stream opened, proxy \
                                             notified), but registration RPC and incoming stream handling \
                                             remain deferred";

#[derive(Debug, Clone)]
pub(crate) struct QuicTunnelServiceFactory {
    test_target: Option<QuicEdgeTarget>,
    protocol_sender: ProtocolSender,
}

impl QuicTunnelServiceFactory {
    pub(crate) fn production(protocol_sender: ProtocolSender) -> Self {
        Self {
            test_target: None,
            protocol_sender,
        }
    }

    #[cfg(test)]
    fn with_test_target(protocol_sender: ProtocolSender, target: QuicEdgeTarget) -> Self {
        Self {
            test_target: Some(target),
            protocol_sender,
        }
    }
}

impl RuntimeServiceFactory for QuicTunnelServiceFactory {
    fn create_primary(&self, config: Arc<RuntimeConfig>, attempt: u32) -> Box<dyn RuntimeService> {
        Box::new(QuicTunnelService {
            config,
            attempt,
            test_target: self.test_target.clone(),
            protocol_sender: self.protocol_sender.clone(),
        })
    }
}

struct QuicTunnelService {
    config: Arc<RuntimeConfig>,
    attempt: u32,
    test_target: Option<QuicEdgeTarget>,
    protocol_sender: ProtocolSender,
}

impl RuntimeService for QuicTunnelService {
    fn name(&self) -> &'static str {
        "quic-tunnel-core"
    }

    fn spawn(
        self: Box<Self>,
        command_tx: mpsc::Sender<RuntimeCommand>,
        shutdown: CancellationToken,
        child_tasks: &mut JoinSet<ChildTask>,
    ) {
        child_tasks.spawn(async move {
            let service_name = self.name();
            let exit = self.run(command_tx.clone(), shutdown).await;
            let _ = command_tx.send(RuntimeCommand::ServiceExited(exit)).await;
            ChildTask::Service(service_name)
        });
    }
}

impl QuicTunnelService {
    async fn run(self, command_tx: mpsc::Sender<RuntimeCommand>, shutdown: CancellationToken) -> ServiceExit {
        let service_name = self.name();
        let identity = match self.load_identity() {
            Ok(identity) => identity,
            Err(detail) => {
                return ServiceExit::Fatal {
                    service: service_name,
                    detail,
                };
            }
        };

        self.report_identity_status(&command_tx, service_name, &identity)
            .await;

        let target = match self.resolve_target(&identity).await {
            Ok(target) => target,
            Err(detail) => {
                return ServiceExit::RetryableFailure {
                    service: service_name,
                    detail,
                };
            }
        };

        send_status(
            &command_tx,
            service_name,
            format!(
                "transport-edge-target: host={} addr={} endpoint-hint={}",
                target.host_label,
                target.connect_addr,
                identity.endpoint_hint.as_deref().unwrap_or(EDGE_DEFAULT_REGION)
            ),
        )
        .await;
        send_transport_stage(
            &command_tx,
            service_name,
            TransportLifecycleStage::Dialing,
            format!("edge={}", target.connect_addr),
        )
        .await;
        send_status(
            &command_tx,
            service_name,
            "transport-session-state: dialing".to_owned(),
        )
        .await;

        match self
            .establish_quic_session(identity, target, &command_tx, shutdown)
            .await
        {
            Ok(ServiceExit::Completed { service }) => ServiceExit::Completed { service },
            Ok(ServiceExit::Deferred {
                service,
                phase,
                detail,
            }) => ServiceExit::Deferred {
                service,
                phase,
                detail,
            },
            Ok(ServiceExit::RetryableFailure { service, detail }) => {
                ServiceExit::RetryableFailure { service, detail }
            }
            Ok(ServiceExit::Fatal { service, detail }) => ServiceExit::Fatal { service, detail },
            Err(detail) => ServiceExit::RetryableFailure {
                service: self.name(),
                detail,
            },
        }
    }

    fn load_identity(&self) -> Result<TransportIdentity, String> {
        TransportIdentity::from_runtime_config(&self.config)
    }

    async fn report_identity_status(
        &self,
        command_tx: &mpsc::Sender<RuntimeCommand>,
        service_name: &'static str,
        identity: &TransportIdentity,
    ) {
        let endpoint_hint = identity.endpoint_hint.as_deref().unwrap_or(EDGE_DEFAULT_REGION);

        send_transport_stage(
            command_tx,
            service_name,
            TransportLifecycleStage::IdentityLoaded,
            format!("identity-source={}", identity.identity_source),
        )
        .await;

        send_status(
            command_tx,
            service_name,
            format!("transport-phase: quiche attempt={}", self.attempt + 1),
        )
        .await;
        send_status(
            command_tx,
            service_name,
            format!("transport-tunnel-id: {}", identity.tunnel_id),
        )
        .await;
        send_status(
            command_tx,
            service_name,
            format!("transport-identity-source: {}", identity.identity_source),
        )
        .await;
        send_status(
            command_tx,
            service_name,
            format!("quic-0rtt-policy: {}", identity.resumption.policy_label()),
        )
        .await;
        send_status(
            command_tx,
            service_name,
            "quic-pqc-compatibility: preserved through quiche + boringssl lane".to_owned(),
        )
        .await;
        send_transport_stage(
            command_tx,
            service_name,
            TransportLifecycleStage::ResolvingEdge,
            format!("endpoint-hint={endpoint_hint}"),
        )
        .await;
    }

    async fn resolve_target(&self, identity: &TransportIdentity) -> Result<QuicEdgeTarget, String> {
        match self.test_target.as_ref() {
            Some(target) => Ok(target.clone()),
            None => resolve_edge_target(identity).await,
        }
    }

    async fn establish_quic_session(
        &self,
        identity: TransportIdentity,
        target: QuicEdgeTarget,
        command_tx: &mpsc::Sender<RuntimeCommand>,
        shutdown: CancellationToken,
    ) -> Result<ServiceExit, String> {
        let mut session = self.initialize_quic_session(&target).await?;

        if let Some(exit) = self
            .await_handshake(&mut session, &target, command_tx, shutdown)
            .await?
        {
            return Ok(exit);
        }

        self.report_established(&session, &identity, &target, command_tx)
            .await;

        if let Some(exit) = self
            .cross_protocol_boundary(&mut session, &target, command_tx)
            .await?
        {
            return Ok(exit);
        }

        self.teardown_session(&mut session, &target, command_tx).await;

        Ok(ServiceExit::Deferred {
            service: self.name(),
            phase: "later runtime/protocol slices",
            detail: format!(
                "{} for tunnel {} against {}",
                WIRE_PROTOCOL_DEFERRED_DETAIL, identity.tunnel_id, target.connect_addr
            ),
        })
    }

    async fn initialize_quic_session(&self, target: &QuicEdgeTarget) -> Result<QuicSessionState, String> {
        let mut session = QuicSessionState::initialize(target).await?;

        flush_egress(
            &session.socket,
            &mut session.connection,
            &mut *session.send_buffer,
        )
        .await
        .map_err(|error| format!("failed to send initial QUIC packets: {error}"))?;

        Ok(session)
    }

    async fn await_handshake(
        &self,
        session: &mut QuicSessionState,
        target: &QuicEdgeTarget,
        command_tx: &mpsc::Sender<RuntimeCommand>,
        shutdown: CancellationToken,
    ) -> Result<Option<ServiceExit>, String> {
        let establish_timer = time::sleep(QUIC_ESTABLISH_TIMEOUT);
        tokio::pin!(establish_timer);

        send_status(
            command_tx,
            self.name(),
            format!(
                "transport-session-state: handshaking local={}",
                session.local_addr
            ),
        )
        .await;
        send_transport_stage(
            command_tx,
            self.name(),
            TransportLifecycleStage::Handshaking,
            format!("local={} remote={}", session.local_addr, target.connect_addr),
        )
        .await;

        loop {
            if session.connection.is_established() {
                return Ok(None);
            }

            if session.connection.is_closed() {
                return Ok(Some(ServiceExit::RetryableFailure {
                    service: self.name(),
                    detail: format!(
                        "quic transport closed before establishment for edge {}",
                        target.connect_addr
                    ),
                }));
            }

            tokio::select! {
                _ = shutdown.cancelled() => {
                    send_status(command_tx, self.name(), "transport-session-state: teardown-before-establish".to_owned()).await;
                    return Ok(Some(ServiceExit::Completed { service: self.name() }));
                }
                _ = &mut establish_timer => {
                    return Ok(Some(ServiceExit::RetryableFailure {
                        service: self.name(),
                        detail: format!("quic handshake timed out for edge {}", target.connect_addr),
                    }));
                }
                recv_result = session.socket.recv_from(&mut *session.recv_buffer) => {
                    let (read, from) = recv_result
                        .map_err(|error| format!("failed to receive QUIC packet from edge: {error}"))?;
                    let recv_info = quiche::RecvInfo {
                        from,
                        to: session.local_addr,
                    };

                    match session.connection.recv(&mut session.recv_buffer[..read], recv_info) {
                        Ok(_) | Err(quiche::Error::Done) => {}
                        Err(error) => {
                            return Ok(Some(ServiceExit::RetryableFailure {
                                service: self.name(),
                                detail: format!("quic handshake failed while reading edge packets: {error}"),
                            }));
                        }
                    }

                    flush_egress(
                        &session.socket,
                        &mut session.connection,
                        &mut *session.send_buffer,
                    )
                    .await
                    .map_err(|error| format!("failed to flush QUIC packets during handshake: {error}"))?;
                }
            }
        }
    }

    async fn report_established(
        &self,
        session: &QuicSessionState,
        identity: &TransportIdentity,
        target: &QuicEdgeTarget,
        command_tx: &mpsc::Sender<RuntimeCommand>,
    ) {
        send_status(
            command_tx,
            self.name(),
            format!(
                "transport-session-state: established peer={} early-data={} resumed-shape={}",
                target.connect_addr,
                session.connection.is_in_early_data(),
                identity.resumption.shape_label(),
            ),
        )
        .await;
        send_transport_stage(
            command_tx,
            self.name(),
            TransportLifecycleStage::Established,
            format!(
                "peer={} resumed-shape={}",
                target.connect_addr,
                identity.resumption.shape_label()
            ),
        )
        .await;
        let _ = command_tx
            .send(RuntimeCommand::ServiceReady { service: self.name() })
            .await;
    }

    async fn cross_protocol_boundary(
        &self,
        session: &mut QuicSessionState,
        target: &QuicEdgeTarget,
        command_tx: &mpsc::Sender<RuntimeCommand>,
    ) -> Result<Option<ServiceExit>, String> {
        // Phase 3.5 + 4.1: Cross the wire/protocol boundary and report
        // the transport-owned stage transition explicitly.
        // Open the control stream on the established QUIC session.
        // This proves wire-level protocol behavior exists beyond
        // transport establishment. Registration RPC content and
        // incoming request stream handling remain deferred.
        match session.connection.stream_send(CONTROL_STREAM_ID, &[], false) {
            Ok(_) | Err(quiche::Error::Done) => {
                send_status(
                    command_tx,
                    self.name(),
                    format!("protocol-boundary: control-stream-{CONTROL_STREAM_ID} opened"),
                )
                .await;
                send_transport_stage(
                    command_tx,
                    self.name(),
                    TransportLifecycleStage::ControlStreamOpened,
                    format!("stream-id={CONTROL_STREAM_ID}"),
                )
                .await;
            }
            Err(error) => {
                return Ok(Some(ServiceExit::RetryableFailure {
                    service: self.name(),
                    detail: format!("failed to open control stream at wire/protocol boundary: {error}"),
                }));
            }
        }

        flush_egress(
            &session.socket,
            &mut session.connection,
            &mut *session.send_buffer,
        )
        .await
        .map_err(|error| format!("failed to flush control stream at wire/protocol boundary: {error}"))?;

        // Notify the proxy layer through the explicit protocol bridge.
        // The runtime consumes the resulting owner-scoped updates to
        // derive its narrow readiness and failure-visibility surface.
        self.protocol_sender
            .send(ProtocolEvent::Registered {
                peer: target.connect_addr.to_string(),
            })
            .await
            .map_err(|detail| {
                format!("failed to report transport registration across protocol bridge: {detail}")
            })?;

        let _ = command_tx
            .send(RuntimeCommand::ProtocolState {
                state: ProtocolBridgeState::RegistrationSent,
                detail: format!(
                    "transport sent registration event for peer {}",
                    target.connect_addr
                ),
            })
            .await;

        send_status(
            command_tx,
            self.name(),
            "protocol-boundary: registration event sent to proxy layer".to_owned(),
        )
        .await;

        Ok(None)
    }

    async fn teardown_session(
        &self,
        session: &mut QuicSessionState,
        target: &QuicEdgeTarget,
        command_tx: &mpsc::Sender<RuntimeCommand>,
    ) {
        // Graceful close — the wire/protocol boundary has been crossed.
        let _ = session.connection.close(true, 0x00, b"protocol boundary crossed");
        let _ = flush_egress(
            &session.socket,
            &mut session.connection,
            &mut *session.send_buffer,
        )
        .await;
        send_transport_stage(
            command_tx,
            self.name(),
            TransportLifecycleStage::Teardown,
            format!("peer={}", target.connect_addr),
        )
        .await;
        send_status(
            command_tx,
            self.name(),
            "transport-session-state: teardown".to_owned(),
        )
        .await;
    }
}

async fn send_status(command_tx: &mpsc::Sender<RuntimeCommand>, service: &'static str, detail: String) {
    let _ = command_tx
        .send(RuntimeCommand::ServiceStatus { service, detail })
        .await;
}

async fn send_transport_stage(
    command_tx: &mpsc::Sender<RuntimeCommand>,
    service: &'static str,
    stage: TransportLifecycleStage,
    detail: String,
) {
    let _ = command_tx
        .send(RuntimeCommand::TransportStage {
            service,
            stage,
            detail,
        })
        .await;
}
