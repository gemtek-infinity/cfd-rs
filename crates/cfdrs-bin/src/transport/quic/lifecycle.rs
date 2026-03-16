use tokio::sync::mpsc;
use tokio::time;
use tokio_util::sync::CancellationToken;

use super::datagram::{DatagramSessionManager, dispatch_datagram};
use super::edge::QuicEdgeTarget;
use super::identity::TransportIdentity;
use super::session::{QuicSessionState, flush_egress};
use super::{QUIC_ESTABLISH_TIMEOUT, QuicTunnelService, TransportLifecycleStage};
use crate::protocol::{CONTROL_STREAM_ID, ProtocolBridgeState, ProtocolEvent};
use crate::runtime::{RuntimeCommand, ServiceExit};
use cfdrs_cdc::registration::{ConnectionOptions, ConnectionResponse, RegisterConnectionRequest, TunnelAuth};
use cfdrs_cdc::stream::ConnectRequest;

const REGISTRATION_RESPONSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

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
            .await_handshake(&mut session, &target, command_tx, shutdown.clone())
            .await?
        {
            return Ok(exit);
        }

        self.report_established(&session, &identity, &target, command_tx)
            .await;

        if let Some(exit) = self
            .cross_protocol_boundary(&mut session, &identity, &target, command_tx, shutdown.clone())
            .await?
        {
            return Ok(exit);
        }

        // Phase 5.1: Enter the stream-serving loop. Accept incoming QUIC
        // data streams from the edge, parse ConnectRequest metadata, and
        // forward them to the proxy layer through the protocol bridge.
        // CDC-040/041: Also handle V3 QUIC datagrams for UDP session proxying.
        let datagram_manager = DatagramSessionManager::new();
        let exit = self
            .serve_streams(&mut session, &target, command_tx, shutdown, &datagram_manager)
            .await;

        // CDC-019: Signal Unregistering on graceful shutdown before teardown.
        // Matches Go's waitForUnregister() → GracefulShutdown() flow in
        // connection/control.go.
        if matches!(exit, ServiceExit::Completed { .. }) {
            let _ = self
                .protocol_sender
                .send(ProtocolEvent::Unregistering {
                    conn_index: u8::try_from(self.attempt).unwrap_or(u8::MAX),
                })
                .await;

            // CDC-007: Send UnregisterConnection on the control stream.
            // Matches Go's `registrationClient.GracefulShutdown(ctx, gracePeriod)`
            // which calls `client.UnregisterConnection(ctx)` in
            // connection/control.go:waitForUnregister().
            self.send_unregister_connection(&mut session, command_tx).await;
        }

        self.teardown_session(&mut session, &target, command_tx).await;

        Ok(exit)
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

                    if let Some(exit) = process_handshake_packet(session, from, read, self.name()) {
                        return Ok(Some(exit));
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
        identity: &TransportIdentity,
        target: &QuicEdgeTarget,
        command_tx: &mpsc::Sender<RuntimeCommand>,
        shutdown: CancellationToken,
    ) -> Result<Option<ServiceExit>, String> {
        // Phase 3.5 + 4.1: Cross the wire/protocol boundary and report
        // the transport-owned stage transition explicitly.
        // Open the control stream on the established QUIC session.
        // This proves wire-level protocol behavior exists beyond
        // transport establishment. Registration RPC content and
        // incoming request stream handling remain deferred.
        let registration_request =
            build_registration_request(identity, target, self.attempt, session.local_addr);
        let control_payload = registration_request
            .as_ref()
            .map(serialize_registration_request)
            .unwrap_or_default();
        // CDC-007: Keep the control stream open for subsequent operations
        // (sendLocalConfiguration, unregisterConnection). The write side
        // is closed only when the final unregister message is sent during
        // graceful shutdown.
        let control_fin = false;

        match session
            .connection
            .stream_send(CONTROL_STREAM_ID, &control_payload, control_fin)
        {
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

                if registration_request.is_some() {
                    super::send_status(
                        command_tx,
                        self.name(),
                        "protocol-boundary: bounded registration request sent over control stream".to_owned(),
                    )
                    .await;
                } else {
                    super::send_status(
                        command_tx,
                        self.name(),
                        "protocol-boundary: registration content deferred for origin-cert identity"
                            .to_owned(),
                    )
                    .await;
                }
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
                peer: target.connect_addr,
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

        if registration_request.is_some() {
            self.await_registration_response(session, target, command_tx, shutdown)
                .await?;
        }

        Ok(None)
    }

    async fn await_registration_response(
        &self,
        session: &mut QuicSessionState,
        target: &QuicEdgeTarget,
        command_tx: &mpsc::Sender<RuntimeCommand>,
        shutdown: CancellationToken,
    ) -> Result<(), String> {
        let mut response_buf = vec![0_u8; 4096];
        let response_timer = time::sleep(REGISTRATION_RESPONSE_TIMEOUT);
        tokio::pin!(response_timer);

        loop {
            match session
                .connection
                .stream_recv(CONTROL_STREAM_ID, &mut response_buf)
            {
                Ok((read, _fin)) => {
                    let Some(response) = parse_registration_response(&response_buf[..read]) else {
                        super::send_status(
                            command_tx,
                            self.name(),
                            "protocol-boundary: registration response was unreadable; continuing with \
                             explicit defer"
                                .to_owned(),
                        )
                        .await;
                        return Ok(());
                    };

                    match response {
                        ConnectionResponse::Success(details) => {
                            self.protocol_sender
                                .send(ProtocolEvent::RegistrationComplete {
                                    conn_uuid: details.uuid,
                                    location: details.location.clone(),
                                })
                                .await
                                .map_err(|detail| {
                                    format!(
                                        "failed to report registration completion across protocol bridge: \
                                         {detail}"
                                    )
                                })?;

                            // CDC-019/CDC-008: On conn_index 0 when not remotely
                            // managed, send local configuration to the edge.
                            // Matches Go baseline connection/control.go:
                            //   if connIndex == 0 && !TunnelIsRemotelyManaged {
                            //       SendLocalConfiguration(ctx, tunnelConfig)
                            //   }
                            if self.attempt == 0 && !details.is_remotely_managed {
                                self.send_local_configuration(session, command_tx).await;
                            }

                            super::send_status(
                                command_tx,
                                self.name(),
                                format!(
                                    "protocol-boundary: registration response received uuid={} location={}",
                                    details.uuid, details.location
                                ),
                            )
                            .await;

                            return Ok(());
                        }

                        ConnectionResponse::Error(conn_err) => {
                            super::send_status(
                                command_tx,
                                self.name(),
                                format!(
                                    "protocol-boundary: registration response reported error={} retry={} \
                                     continuing",
                                    conn_err.cause, conn_err.should_retry
                                ),
                            )
                            .await;

                            return Ok(());
                        }
                    }
                }
                Err(quiche::Error::Done) => {}
                Err(error) => {
                    return Err(format!(
                        "failed to read registration response from control stream for peer {}: {error}",
                        target.connect_addr
                    ));
                }
            }

            tokio::select! {
                biased;
                _ = shutdown.cancelled() => return Ok(()),
                _ = &mut response_timer => {
                    super::send_status(
                        command_tx,
                        self.name(),
                        "protocol-boundary: registration response deferred after bounded wait"
                            .to_owned(),
                    )
                    .await;

                    return Ok(());
                }
                recv_result = session.socket.recv_from(&mut *session.recv_buffer) => {
                    let (read, from) = recv_result
                        .map_err(|error| format!("failed to receive registration response packet from edge: {error}"))?;

                    let recv_info = quiche::RecvInfo {
                        from,
                        to: session.local_addr,
                    };
                    let _ = session
                        .connection
                        .recv(&mut session.recv_buffer[..read], recv_info);

                    flush_egress(
                        &session.socket,
                        &mut session.connection,
                        &mut *session.send_buffer,
                    )
                    .await
                    .map_err(|error| format!("failed to flush registration response packets: {error}"))?;
                }
            }
        }
    }

    /// Serve incoming QUIC data streams until shutdown or connection close.
    ///
    /// Enters the stream-serving phase: receives UDP packets, processes
    /// readable QUIC streams, parses ConnectRequest metadata from edge-
    /// initiated data streams, and forwards them to the proxy layer
    /// through the protocol bridge.
    async fn serve_streams(
        &self,
        session: &mut QuicSessionState,
        target: &QuicEdgeTarget,
        command_tx: &mpsc::Sender<RuntimeCommand>,
        shutdown: CancellationToken,
        datagram_manager: &DatagramSessionManager,
    ) -> ServiceExit {
        super::send_transport_stage(
            command_tx,
            self.name(),
            TransportLifecycleStage::ServingStreams,
            format!("peer={}", target.connect_addr),
        )
        .await;
        super::send_status(
            command_tx,
            self.name(),
            "transport-session-state: serving-streams".to_owned(),
        )
        .await;

        let mut stream_buf = vec![0u8; 65_535];
        let mut streams_accepted: u64 = 0;

        loop {
            if session.connection.is_closed() {
                return ServiceExit::RetryableFailure {
                    service: self.name(),
                    detail: format!(
                        "quic connection closed during stream serving for peer {}",
                        target.connect_addr,
                    ),
                };
            }

            streams_accepted += self
                .process_readable_streams(session, &mut stream_buf, streams_accepted, command_tx)
                .await;

            // CDC-040/041: Drain pending QUIC datagrams and dispatch them
            // through the session manager. Matches Go's
            // `datagramConn.Serve()` receive loop in `quic/v3/muxer.go`.
            let conn_index = u8::try_from(self.attempt).unwrap_or(u8::MAX);
            self.drain_datagrams(session, conn_index, datagram_manager);

            self.drain_pending_responses(session);

            let _ = flush_egress(
                &session.socket,
                &mut session.connection,
                &mut *session.send_buffer,
            )
            .await;

            if let Some(exit) = self.await_next_packet(session, &shutdown).await {
                return exit;
            }
        }
    }

    /// Process all readable QUIC data streams, forwarding parsed requests
    /// to the proxy layer. Returns the number of newly accepted streams.
    async fn process_readable_streams(
        &self,
        session: &mut QuicSessionState,
        stream_buf: &mut [u8],
        streams_accepted: u64,
        command_tx: &mpsc::Sender<RuntimeCommand>,
    ) -> u64 {
        let mut accepted = 0;
        let readable: Vec<u64> = session.connection.readable().collect();

        for stream_id in readable {
            if stream_id == CONTROL_STREAM_ID {
                continue;
            }

            // Only accept server-initiated bidi streams (edge-initiated).
            // Per QUIC: bit 0 = initiator (0=client, 1=server), bit 1 = type (0=bidi,
            // 1=uni).
            if stream_id % 4 != 1 {
                continue;
            }

            accepted += self
                .try_accept_stream(
                    session,
                    stream_id,
                    stream_buf,
                    streams_accepted + accepted,
                    command_tx,
                )
                .await;
        }

        accepted
    }

    /// Drain all pending QUIC datagrams from the connection and dispatch
    /// them through the V3 session manager.
    ///
    /// Matches Go's `datagramConn.Serve()` receive → dispatch loop in
    /// `quic/v3/muxer.go`. Response datagrams (session registration acks)
    /// are sent back through the same QUIC connection.
    fn drain_datagrams(
        &self,
        session: &mut QuicSessionState,
        conn_index: u8,
        datagram_manager: &DatagramSessionManager,
    ) {
        let mut dgram_buf = [0u8; cfdrs_cdc::datagram::MAX_DATAGRAM_PAYLOAD_LEN];

        loop {
            match session.connection.dgram_recv(&mut dgram_buf) {
                Ok(len) => {
                    if let Some(response) = dispatch_datagram(&dgram_buf[..len], conn_index, datagram_manager)
                    {
                        // Best-effort: if the send fails the response is lost,
                        // matching Go's behavior where send errors are logged
                        // but do not stop the muxer loop.
                        let _ = session.connection.dgram_send(&response);
                    }
                }
                Err(quiche::Error::Done) => break,
                Err(error) => {
                    tracing::warn!(%error, "error receiving QUIC datagram");
                    break;
                }
            }
        }
    }

    /// Try to read and parse a ConnectRequest from a single QUIC stream.
    /// Returns 1 if a request was successfully forwarded, 0 otherwise.
    async fn try_accept_stream(
        &self,
        session: &mut QuicSessionState,
        stream_id: u64,
        stream_buf: &mut [u8],
        total_before: u64,
        command_tx: &mpsc::Sender<RuntimeCommand>,
    ) -> u64 {
        match session.connection.stream_recv(stream_id, stream_buf) {
            Ok((read, _fin)) => {
                let Some(request) = parse_connect_request(&stream_buf[..read]) else {
                    return 0;
                };

                let _ = self
                    .protocol_sender
                    .send(ProtocolEvent::IncomingStream { stream_id, request })
                    .await;

                super::send_status(
                    command_tx,
                    self.name(),
                    format!("stream-accepted: stream={stream_id} total={}", total_before + 1),
                )
                .await;

                1
            }
            Err(quiche::Error::Done) => 0,
            Err(error) => {
                super::send_status(
                    command_tx,
                    self.name(),
                    format!("stream-error: stream={stream_id} error={error}"),
                )
                .await;
                0
            }
        }
    }

    /// Wait for the next inbound UDP packet or a shutdown signal.
    /// Returns `Some(ServiceExit)` if the loop should terminate.
    async fn await_next_packet(
        &self,
        session: &mut QuicSessionState,
        shutdown: &CancellationToken,
    ) -> Option<ServiceExit> {
        tokio::select! {
            biased;
            _ = shutdown.cancelled() => {
                Some(ServiceExit::Completed {
                    service: self.name(),
                })
            }
            recv_result = session.socket.recv_from(&mut *session.recv_buffer) => {
                match recv_result {
                    Ok((read, from)) => {
                        let recv_info = quiche::RecvInfo {
                            from,
                            to: session.local_addr,
                        };
                        let _ = session
                            .connection
                            .recv(&mut session.recv_buffer[..read], recv_info);
                        None
                    }
                    Err(error) => {
                        Some(ServiceExit::RetryableFailure {
                            service: self.name(),
                            detail: format!(
                                "UDP recv failed during stream serving: {error}"
                            ),
                        })
                    }
                }
            }
        }
    }

    /// Drain pending stream responses from the proxy and write them to
    /// QUIC data streams so they are included in the next egress flush.
    fn drain_pending_responses(&self, session: &mut QuicSessionState) {
        let Ok(mut rx) = self.stream_response_rx.lock() else {
            return;
        };

        while let Ok(response) = rx.try_recv() {
            let _ = session
                .connection
                .stream_send(response.stream_id, &response.data, true);
        }
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

        // CDC-019: Signal Disconnected after the connection is closed.
        // Matches Go's Disconnected status in connection/event.go.
        let _ = self
            .protocol_sender
            .send(ProtocolEvent::Disconnected {
                conn_index: u8::try_from(self.attempt).unwrap_or(u8::MAX),
            })
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

    /// CDC-008/CDC-019: Send local configuration to the edge on conn_index 0.
    ///
    /// Matches Go baseline `connection/control.go`:
    ///   `registrationClient.SendLocalConfiguration(ctx, tunnelConfig)`
    ///
    /// Serialises the running ingress/warp-routing/originRequest config as
    /// JSON, writes it to the control stream, and emits `ConfigPushed`.
    /// Errors are logged but do not abort the lifecycle (matching Go).
    async fn send_local_configuration(
        &self,
        session: &mut QuicSessionState,
        command_tx: &mpsc::Sender<RuntimeCommand>,
    ) {
        let config_json = match serde_json::to_vec(&serde_json::json!({
            "ingress": self.config.normalized().ingress,
            "warp-routing": self.config.normalized().warp_routing,
            "originRequest": self.config.normalized().origin_request,
        })) {
            Ok(json) => json,
            Err(error) => {
                super::send_status(
                    command_tx,
                    self.name(),
                    format!("config-push: failed to serialize local configuration: {error}"),
                )
                .await;
                return;
            }
        };

        let request = cfdrs_cdc::registration::UpdateLocalConfigurationRequest { config: config_json };
        let payload = request.to_capnp_bytes();

        match session.connection.stream_send(CONTROL_STREAM_ID, &payload, false) {
            Ok(_) => {
                let _ = self
                    .protocol_sender
                    .send(ProtocolEvent::ConfigPushed { conn_index: 0 })
                    .await;

                super::send_status(
                    command_tx,
                    self.name(),
                    format!("config-push: sent local configuration ({} bytes)", payload.len()),
                )
                .await;
            }
            Err(error) => {
                super::send_status(
                    command_tx,
                    self.name(),
                    format!("config-push: failed to write to control stream: {error}"),
                )
                .await;
            }
        }
    }

    /// CDC-007: Send `UnregisterConnection` on the control stream during
    /// graceful shutdown.
    ///
    /// Matches Go baseline `connection/control.go:waitForUnregister()` →
    /// `registrationClient.GracefulShutdown(ctx, gracePeriod)` →
    /// `client.UnregisterConnection(ctx)`.
    ///
    /// Best-effort: errors are logged but do not abort teardown, matching
    /// Go where `GracefulShutdown` ignores the error from
    /// `UnregisterConnection`.
    async fn send_unregister_connection(
        &self,
        session: &mut QuicSessionState,
        command_tx: &mpsc::Sender<RuntimeCommand>,
    ) {
        let payload = cfdrs_cdc::registration_codec::encode_unregister_request();

        // Send with fin=true — this is the last message on the control stream.
        match session.connection.stream_send(CONTROL_STREAM_ID, &payload, true) {
            Ok(_) => {
                let _ = flush_egress(
                    &session.socket,
                    &mut session.connection,
                    &mut *session.send_buffer,
                )
                .await;

                super::send_status(
                    command_tx,
                    self.name(),
                    format!(
                        "unregister: sent UnregisterConnection on control stream ({} bytes)",
                        payload.len()
                    ),
                )
                .await;
            }
            Err(error) => {
                super::send_status(
                    command_tx,
                    self.name(),
                    format!("unregister: failed to write to control stream: {error}"),
                )
                .await;
            }
        }
    }
}

/// Process a single handshake packet, returning a `ServiceExit` if the
/// connection encountered a fatal recv error.
fn process_handshake_packet(
    session: &mut QuicSessionState,
    from: std::net::SocketAddr,
    read: usize,
    service_name: &'static str,
) -> Option<ServiceExit> {
    let recv_info = quiche::RecvInfo {
        from,
        to: session.local_addr,
    };

    match session
        .connection
        .recv(&mut session.recv_buffer[..read], recv_info)
    {
        Ok(_) | Err(quiche::Error::Done) => None,
        Err(error) => Some(ServiceExit::RetryableFailure {
            service: service_name,
            detail: format!("quic handshake failed while reading edge packets: {error}"),
        }),
    }
}

/// Parse a ConnectRequest from raw stream data.
///
/// Decode a `ConnectRequest` from the wire format.
///
/// Delegates to the CDC-owned codec in `cfdrs_cdc::stream_codec`.
fn parse_connect_request(data: &[u8]) -> Option<ConnectRequest> {
    cfdrs_cdc::stream_codec::decode_connect_request(data)
}

fn build_registration_request(
    identity: &TransportIdentity,
    _target: &QuicEdgeTarget,
    attempt: u32,
    local_addr: std::net::SocketAddr,
) -> Option<RegisterConnectionRequest> {
    let auth = identity.registration_auth.as_ref()?;
    let mut options =
        ConnectionOptions::for_current_platform(identity.tunnel_id, u8::try_from(attempt).unwrap_or(u8::MAX));
    options.origin_local_ip = Some(local_addr.ip());
    options.client.features = cfdrs_cdc::features::build_feature_list(true, false);

    Some(RegisterConnectionRequest {
        auth: TunnelAuth {
            account_tag: auth.account_tag.clone(),
            tunnel_secret: auth.tunnel_secret.as_bytes().to_vec(),
        },
        tunnel_id: identity.tunnel_id,
        conn_index: u8::try_from(attempt).unwrap_or(u8::MAX),
        options,
    })
}

fn serialize_registration_request(request: &RegisterConnectionRequest) -> Vec<u8> {
    cfdrs_cdc::registration_codec::encode_registration_request(request)
}

fn parse_registration_response(data: &[u8]) -> Option<ConnectionResponse> {
    cfdrs_cdc::registration_codec::decode_registration_response(data)
}

/// Encode a `ConnectRequest` into the wire format for testing.
///
/// Delegates to the CDC-owned codec in `cfdrs_cdc::stream_codec`.
#[cfg(test)]
pub(super) fn serialize_connect_request(request: &ConnectRequest) -> Vec<u8> {
    cfdrs_cdc::stream_codec::encode_connect_request(request)
}

#[cfg(test)]
pub(super) fn serialize_registration_response(response: &ConnectionResponse) -> Vec<u8> {
    cfdrs_cdc::registration_codec::encode_registration_response(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cfdrs_cdc::registration::ConnectionDetails;
    use cfdrs_cdc::stream::{ConnectionType, Metadata};
    use uuid::Uuid;

    #[test]
    fn connect_request_roundtrip() {
        let original = ConnectRequest {
            dest: "http://example.com/api".to_owned(),
            connection_type: ConnectionType::Http,
            metadata: vec![
                Metadata::new("HttpMethod", "GET"),
                Metadata::new("HttpHost", "example.com"),
            ],
        };

        let wire = serialize_connect_request(&original);
        let parsed = parse_connect_request(&wire).expect("roundtrip parse should succeed");

        assert_eq!(parsed.dest, original.dest);
        assert_eq!(parsed.connection_type, original.connection_type);
        assert_eq!(parsed.metadata.len(), original.metadata.len());

        for (a, b) in parsed.metadata.iter().zip(&original.metadata) {
            assert_eq!(a.key, b.key);
            assert_eq!(a.val, b.val);
        }
    }

    #[test]
    fn parse_empty_data_returns_none() {
        assert!(parse_connect_request(&[]).is_none());
    }

    #[test]
    fn parse_truncated_data_returns_none() {
        // Only 2 bytes — enough for connection type but nothing else.
        assert!(parse_connect_request(&[0, 0]).is_none());
    }

    #[test]
    fn registration_request_builds_for_credentials_identity() {
        let identity = TransportIdentity {
            tunnel_id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid should parse"),
            identity_source: super::super::identity::IdentitySource::CredentialsFile,
            endpoint_hint: Some("us".to_owned()),
            registration_auth: Some(super::super::identity::RegistrationAuth {
                account_tag: "account".to_owned(),
                tunnel_secret: cfdrs_shared::TunnelSecret::from_bytes(b"secret".to_vec()),
            }),
            resumption: super::super::identity::ResumptionShape::EarlyDataEnabled,
        };
        let target = QuicEdgeTarget {
            connect_addr: "127.0.0.1:7844".parse().expect("target should parse"),
            host_label: "region1.v2.argotunnel.com".to_owned(),
            server_name: "localhost".to_owned(),
            verification: super::super::edge::PeerVerification::Unverified,
        };

        let request = build_registration_request(
            &identity,
            &target,
            2,
            "127.0.0.1:40000".parse().expect("local addr should parse"),
        )
        .expect("credentials-file identity should build a request");

        assert_eq!(request.auth.account_tag, "account");
        assert_eq!(request.auth.tunnel_secret, b"secret");
        assert_eq!(request.tunnel_id, identity.tunnel_id);
        assert_eq!(request.conn_index, 2);
        assert_eq!(request.options.num_previous_attempts, 2);
        assert_eq!(
            request.options.origin_local_ip,
            Some("127.0.0.1".parse().expect("ip should parse"))
        );
    }

    #[test]
    fn registration_response_roundtrip() {
        let response = ConnectionResponse::success(ConnectionDetails {
            uuid: Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid should parse"),
            location: "SFO".to_owned(),
            is_remotely_managed: false,
        });

        let wire = serialize_registration_response(&response);
        let parsed = parse_registration_response(&wire).expect("registration response should parse");

        assert_eq!(parsed, response);
    }

    #[test]
    fn registration_request_wire_roundtrip() {
        let identity = TransportIdentity {
            tunnel_id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid should parse"),
            identity_source: super::super::identity::IdentitySource::CredentialsFile,
            endpoint_hint: Some("us".to_owned()),
            registration_auth: Some(super::super::identity::RegistrationAuth {
                account_tag: "acct-wire".to_owned(),
                tunnel_secret: cfdrs_shared::TunnelSecret::from_bytes(b"wire-secret".to_vec()),
            }),
            resumption: super::super::identity::ResumptionShape::EarlyDataEnabled,
        };
        let target = QuicEdgeTarget {
            connect_addr: "127.0.0.1:7844".parse().expect("target should parse"),
            host_label: "region1.v2.argotunnel.com".to_owned(),
            server_name: "localhost".to_owned(),
            verification: super::super::edge::PeerVerification::Unverified,
        };

        let request = build_registration_request(
            &identity,
            &target,
            1,
            "10.0.0.1:50000".parse().expect("local addr should parse"),
        )
        .expect("credentials-file identity should build a request");

        let encoded = serialize_registration_request(&request);
        let decoded = cfdrs_cdc::registration_codec::decode_registration_request(&encoded)
            .expect("wire roundtrip should produce a valid request");

        assert_eq!(decoded.auth, request.auth);
        assert_eq!(decoded.tunnel_id, request.tunnel_id);
        assert_eq!(decoded.conn_index, request.conn_index);
        assert_eq!(decoded.options.client.features, request.options.client.features);
        assert_eq!(decoded.options.origin_local_ip, request.options.origin_local_ip);
    }

    /// CDC-007: the unregister request encodes to a non-empty capnp message
    /// and the response decoder accepts it (both sides are empty structs).
    #[test]
    fn unregister_request_codec_roundtrip() {
        let encoded = cfdrs_cdc::registration_codec::encode_unregister_request();

        assert!(
            !encoded.is_empty(),
            "even an empty-params message needs the capnp segment header"
        );

        assert!(
            cfdrs_cdc::registration_codec::decode_unregister_response(&encoded),
            "an empty capnp struct should parse as a valid unregister response"
        );
    }
}
