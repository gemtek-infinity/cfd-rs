use tokio::sync::mpsc;
use tokio::time;
use tokio_util::sync::CancellationToken;

use super::edge::QuicEdgeTarget;
use super::identity::TransportIdentity;
use super::session::{QuicSessionState, flush_egress};
use super::{
    QUIC_ESTABLISH_TIMEOUT, QuicTunnelService, TransportLifecycleStage, WIRE_PROTOCOL_DEFERRED_DETAIL,
};
use crate::protocol::{CONTROL_STREAM_ID, ProtocolBridgeState, ProtocolEvent};
use crate::runtime::{RuntimeCommand, RuntimeService, ServiceExit};

impl QuicTunnelService {
    pub(super) async fn establish_quic_session(
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

        super::send_status(
            command_tx,
            self.name(),
            format!(
                "transport-session-state: handshaking local={}",
                session.local_addr
            ),
        )
        .await;
        super::send_transport_stage(
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
                    super::send_status(command_tx, self.name(), "transport-session-state: teardown-before-establish".to_owned()).await;
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
                super::send_status(
                    command_tx,
                    self.name(),
                    format!("protocol-boundary: control-stream-{CONTROL_STREAM_ID} opened"),
                )
                .await;
                super::send_transport_stage(
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

        super::send_status(
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
        super::send_transport_stage(
            command_tx,
            self.name(),
            TransportLifecycleStage::Teardown,
            format!("peer={}", target.connect_addr),
        )
        .await;
        super::send_status(
            command_tx,
            self.name(),
            "transport-session-state: teardown".to_owned(),
        )
        .await;
    }
}
