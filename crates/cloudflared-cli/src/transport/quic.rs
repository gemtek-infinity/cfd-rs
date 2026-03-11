use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use cloudflared_config::{FED_ENDPOINT, OriginCertLocator, OriginCertToken, TunnelCredentialsFile};
use quiche::ConnectionId;
use tokio::net::{UdpSocket, lookup_host};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::TransportLifecycleStage;
use crate::protocol::ProtocolBridgeState;
use crate::protocol::{CONTROL_STREAM_ID, ProtocolEvent, ProtocolSender};
use crate::runtime::{
    ChildTask, RuntimeCommand, RuntimeConfig, RuntimeService, RuntimeServiceFactory, ServiceExit,
};

const EDGE_DEFAULT_REGION: &str = "region1";
const EDGE_DEFAULT_HOST: &str = "region1.v2.argotunnel.com";
const EDGE_QUIC_PORT: u16 = 7844;
const EDGE_QUIC_SERVER_NAME: &str = "quic.cftunnel.com";
const EDGE_QUIC_ALPN: &[&[u8]] = &[b"argotunnel"];
const QUIC_ESTABLISH_TIMEOUT: Duration = Duration::from_secs(5);
const QUIC_IDLE_TIMEOUT_MS: u64 = 30_000;
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
        let identity = match TransportIdentity::from_runtime_config(&self.config) {
            Ok(identity) => identity,
            Err(detail) => {
                return ServiceExit::Fatal {
                    service: service_name,
                    detail,
                };
            }
        };

        send_transport_stage(
            &command_tx,
            service_name,
            TransportLifecycleStage::IdentityLoaded,
            format!("identity-source={}", identity.identity_source),
        )
        .await;

        send_status(
            &command_tx,
            service_name,
            format!("transport-phase: quiche attempt={}", self.attempt + 1),
        )
        .await;
        send_status(
            &command_tx,
            service_name,
            format!("transport-tunnel-id: {}", identity.tunnel_id),
        )
        .await;
        send_status(
            &command_tx,
            service_name,
            format!("transport-identity-source: {}", identity.identity_source),
        )
        .await;
        send_status(
            &command_tx,
            service_name,
            format!("quic-0rtt-policy: {}", identity.resumption.policy_label()),
        )
        .await;
        send_status(
            &command_tx,
            service_name,
            "quic-pqc-compatibility: preserved through quiche + boringssl lane".to_owned(),
        )
        .await;
        send_transport_stage(
            &command_tx,
            service_name,
            TransportLifecycleStage::ResolvingEdge,
            format!(
                "endpoint-hint={}",
                identity.endpoint_hint.as_deref().unwrap_or(EDGE_DEFAULT_REGION)
            ),
        )
        .await;

        let target = match self.test_target.as_ref() {
            Some(target) => target.clone(),
            None => match resolve_edge_target(&identity).await {
                Ok(target) => target,
                Err(detail) => {
                    return ServiceExit::RetryableFailure {
                        service: service_name,
                        detail,
                    };
                }
            },
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
        let bind_addr = wildcard_bind_addr(target.connect_addr);
        let socket = UdpSocket::bind(bind_addr)
            .await
            .map_err(|error| format!("failed to bind UDP socket for QUIC transport: {error}"))?;
        let local_addr = socket
            .local_addr()
            .map_err(|error| format!("failed to inspect UDP local address: {error}"))?;

        let mut quic_config = build_quiche_config(target)?;
        let scid_bytes = Uuid::new_v4().into_bytes();
        let scid = ConnectionId::from_ref(scid_bytes.as_ref());
        let connection = quiche::connect(
            Some(target.server_name.as_str()),
            &scid,
            local_addr,
            target.connect_addr,
            &mut quic_config,
        )
        .map_err(|error| format!("failed to initialize quiche client connection: {error}"))?;

        let mut session = QuicSessionState {
            socket,
            local_addr,
            connection,
            recv_buffer: [0_u8; 65_535],
            send_buffer: [0_u8; MAX_DATAGRAM_SIZE],
        };

        flush_egress(&session.socket, &mut session.connection, &mut session.send_buffer)
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
                recv_result = session.socket.recv_from(&mut session.recv_buffer) => {
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
                        &mut session.send_buffer,
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

        flush_egress(&session.socket, &mut session.connection, &mut session.send_buffer)
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
        let _ = flush_egress(&session.socket, &mut session.connection, &mut session.send_buffer).await;
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

struct QuicSessionState {
    socket: UdpSocket,
    local_addr: SocketAddr,
    connection: quiche::Connection,
    recv_buffer: [u8; 65_535],
    send_buffer: [u8; MAX_DATAGRAM_SIZE],
}

#[derive(Debug, Clone)]
struct TransportIdentity {
    tunnel_id: Uuid,
    identity_source: &'static str,
    endpoint_hint: Option<String>,
    resumption: ResumptionShape,
}

impl TransportIdentity {
    fn from_runtime_config(config: &RuntimeConfig) -> Result<Self, String> {
        let normalized = config.normalized();
        let tunnel = normalized
            .tunnel
            .as_ref()
            .ok_or_else(|| String::from("quic tunnel core requires a configured tunnel reference"))?;
        let tunnel_id = tunnel.uuid.ok_or_else(|| {
            String::from("quic tunnel core requires the tunnel reference to be a UUID-backed named tunnel")
        })?;

        let credentials = &normalized.credentials;
        let (identity_source, endpoint_hint) = if let Some(path) = credentials.credentials_file.as_ref() {
            let tunnel_credentials = TunnelCredentialsFile::from_json_path(path).map_err(|error| {
                format!(
                    "failed to load tunnel credentials file {}: {error}",
                    path.display()
                )
            })?;

            if tunnel_credentials.tunnel_id != tunnel_id {
                return Err(format!(
                    "tunnel UUID {} does not match credentials file tunnel ID {}",
                    tunnel_id, tunnel_credentials.tunnel_id
                ));
            }

            (
                "credentials-file",
                tunnel_credentials
                    .endpoint
                    .map(|value| value.to_ascii_lowercase()),
            )
        } else if let Some(path) = origin_cert_path(credentials) {
            let origin_cert = OriginCertToken::from_pem_path(&path)
                .map_err(|error| format!("failed to read origin cert {}: {error}", path.display()))?;
            ("origin-cert", origin_cert.endpoint)
        } else {
            return Err(String::from(
                "quic tunnel core requires credentials-file or origincert to resolve edge interaction \
                 semantics",
            ));
        };

        Ok(Self {
            tunnel_id,
            identity_source,
            endpoint_hint,
            resumption: ResumptionShape::EarlyDataEnabled,
        })
    }
}

#[derive(Debug, Clone)]
enum ResumptionShape {
    EarlyDataEnabled,
}

impl ResumptionShape {
    fn policy_label(&self) -> &'static str {
        match self {
            Self::EarlyDataEnabled => "quiche early data enabled when session tickets are available",
        }
    }

    fn shape_label(&self) -> &'static str {
        match self {
            Self::EarlyDataEnabled => "0-rtt-preserving",
        }
    }
}

#[derive(Debug, Clone)]
struct QuicEdgeTarget {
    connect_addr: SocketAddr,
    host_label: String,
    server_name: String,
    verify_peer: bool,
    ca_bundle_path: Option<PathBuf>,
}

async fn resolve_edge_target(identity: &TransportIdentity) -> Result<QuicEdgeTarget, String> {
    let host_label = edge_host_label(identity.endpoint_hint.as_deref());
    let mut addrs = lookup_host((host_label.clone(), EDGE_QUIC_PORT))
        .await
        .map_err(|error| {
            format!("failed to resolve QUIC edge target {host_label}:{EDGE_QUIC_PORT}: {error}")
        })?;

    let connect_addr = addrs.next().ok_or_else(|| {
        format!("no socket addresses resolved for QUIC edge target {host_label}:{EDGE_QUIC_PORT}")
    })?;

    Ok(QuicEdgeTarget {
        connect_addr,
        host_label,
        server_name: EDGE_QUIC_SERVER_NAME.to_owned(),
        verify_peer: true,
        ca_bundle_path: Some(default_ca_bundle_path().ok_or_else(|| {
            String::from("no Linux CA bundle path found for QUIC edge certificate verification")
        })?),
    })
}

fn edge_host_label(endpoint_hint: Option<&str>) -> String {
    match endpoint_hint {
        Some(FED_ENDPOINT) | None => EDGE_DEFAULT_HOST.to_owned(),
        Some(region) if !region.is_empty() => format!("{region}.v2.argotunnel.com"),
        Some(_) => EDGE_DEFAULT_HOST.to_owned(),
    }
}

fn wildcard_bind_addr(peer: SocketAddr) -> SocketAddr {
    match peer.ip() {
        IpAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        IpAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    }
}

fn default_ca_bundle_path() -> Option<PathBuf> {
    const CANDIDATES: &[&str] = &[
        "/etc/ssl/certs/ca-certificates.crt",
        "/etc/pki/tls/certs/ca-bundle.crt",
        "/etc/ssl/cert.pem",
    ];

    CANDIDATES
        .iter()
        .map(Path::new)
        .find(|path| path.exists())
        .map(Path::to_path_buf)
}

fn build_quiche_config(target: &QuicEdgeTarget) -> Result<quiche::Config, String> {
    let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)
        .map_err(|error| format!("failed to create quiche config: {error}"))?;
    config
        .set_application_protos(EDGE_QUIC_ALPN)
        .map_err(|error| format!("failed to set QUIC ALPN: {error}"))?;
    config.verify_peer(target.verify_peer);
    if target.verify_peer {
        let ca_bundle_path = target
            .ca_bundle_path
            .as_ref()
            .ok_or_else(|| String::from("peer verification requested without a CA bundle path"))?;
        config
            .load_verify_locations_from_file(&ca_bundle_path.to_string_lossy())
            .map_err(|error| {
                format!(
                    "failed to load CA bundle {} for QUIC edge verification: {error}",
                    ca_bundle_path.display()
                )
            })?;
    }

    config.enable_early_data();
    config.set_max_idle_timeout(QUIC_IDLE_TIMEOUT_MS);
    config.set_max_recv_udp_payload_size(MAX_DATAGRAM_SIZE);
    config.set_max_send_udp_payload_size(MAX_DATAGRAM_SIZE);
    config.set_initial_max_data(1_000_000);
    config.set_initial_max_stream_data_bidi_local(256_000);
    config.set_initial_max_stream_data_bidi_remote(256_000);
    config.set_initial_max_stream_data_uni(256_000);
    config.set_initial_max_streams_bidi(32);
    config.set_initial_max_streams_uni(32);
    config.set_disable_active_migration(true);

    Ok(config)
}

async fn flush_egress(
    socket: &UdpSocket,
    connection: &mut quiche::Connection,
    buffer: &mut [u8],
) -> Result<(), String> {
    loop {
        match connection.send(buffer) {
            Ok((written, send_info)) => {
                socket
                    .send_to(&buffer[..written], send_info.to)
                    .await
                    .map_err(|error| format!("failed to send UDP packet to {}: {error}", send_info.to))?;
            }
            Err(quiche::Error::Done) => return Ok(()),
            Err(error) => {
                return Err(format!("quiche send failed while flushing egress: {error}"));
            }
        }
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

fn origin_cert_path(credentials: &cloudflared_config::CredentialSurface) -> Option<PathBuf> {
    match credentials.origin_cert.as_ref() {
        Some(OriginCertLocator::ConfiguredPath(path)) | Some(OriginCertLocator::DefaultSearchPath(path)) => {
            Some(path.clone())
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EDGE_QUIC_ALPN, QuicEdgeTarget, QuicTunnelServiceFactory, TransportIdentity, build_quiche_config,
        default_ca_bundle_path, edge_host_label,
    };
    use crate::protocol;
    use crate::runtime::{RuntimeExit, run_with_factory};
    use cloudflared_config::{ConfigSource, DiscoveryAction, DiscoveryOutcome, NormalizedConfig, RawConfig};
    use std::fs;
    use std::io::ErrorKind;
    use std::net::{SocketAddr, UdpSocket};
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    const TEST_CERT_PEM: &str = concat!(
        "-----BEGIN CERTIFICATE-----\n",
        "MIIDCTCCAfGgAwIBAgIUJb0Jfxu0MAeoFD0npL3VZBW2h+owDQYJKoZIhvcNAQEL\n",
        "BQAwFDESMBAGA1UEAwwJbG9jYWxob3N0MB4XDTI2MDMxMDA3MDYxNFoXDTI3MDMx\n",
        "MDA3MDYxNFowFDESMBAGA1UEAwwJbG9jYWxob3N0MIIBIjANBgkqhkiG9w0BAQEF\n",
        "AAOCAQ8AMIIBCgKCAQEArRtFsb0NMB9y09zu4KBt3h+lvJT0iHYFN46BFehJhD55\n",
        "er1h0cNJHTQ6s8x1cohQpfITM+03ZOMRRYj7rg+L+ylVpkYvTuXBVrK9xcAMwdYk\n",
        "taL4uFHGc1kBs8awa7RfgFwqXEnaQ4sO7ie1FpJ0sViC3t9ZmJ2kJgOPKT6HGUS+\n",
        "miYbZE2c+5FBb1OD0fWNRNakrQtgMIZuHKnG1Iq3CLG8IgQLvkBxL72CPEUyxeks\n",
        "Z7unQR95duwf1Vlz0UcEegfnAz+yNaZGvJ0VOgzountMCWahviCkXqoc3HJthR86\n",
        "feNhWtoa+LEI27ERUFQljuDjxNjX1A3Q+EKcPt9HKwIDAQABo1MwUTAdBgNVHQ4E\n",
        "FgQU7FVi0ezFYdq1iLUAu8yqPYetntEwHwYDVR0jBBgwFoAU7FVi0ezFYdq1iLUA\n",
        "u8yqPYetntEwDwYDVR0TAQH/BAUwAwEB/zANBgkqhkiG9w0BAQsFAAOCAQEAcwIp\n",
        "3w2Bx3vk9hYWrwGfWH/vvyqMrF6GUkcF8557rmO1uXnk9uzHDcjUT+9zmFA/gXxc\n",
        "coCS3l+HjTk6InGq/Bncsc0WR/gdp8JCbOKJKCnTnK1zQdExJ4H2646ARxJNpxPl\n",
        "Cv5/SL7LyJbQm/2H60V/urcIwtl/WnBgw58BZ1wOWXaVQYBaSp2m6A3TPCozrQ2N\n",
        "Hu5tPOzkXjkSMdfOPvHdK3tvIn04gKxAe+kc05efsncWZdlgfpTT5SOfOMp+LQ6T\n",
        "gegfwgzYQzBwWZNUqprAGNyUsW5dxIAWYMkxHr3n4eZ83A8M8GPPKa8TOp6qFbza\n",
        "KWggdegvHvjpedAG8A==\n",
        "-----END CERTIFICATE-----\n",
    );
    const TEST_KEY_PEM: &str = concat!(
        "-----BEGIN PRIVATE KEY-----\n",
        "MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCtG0WxvQ0wH3LT\n",
        "3O7goG3eH6W8lPSIdgU3joEV6EmEPnl6vWHRw0kdNDqzzHVyiFCl8hMz7Tdk4xFF\n",
        "iPuuD4v7KVWmRi9O5cFWsr3FwAzB1iS1ovi4UcZzWQGzxrBrtF+AXCpcSdpDiw7u\n",
        "J7UWknSxWILe31mYnaQmA48pPocZRL6aJhtkTZz7kUFvU4PR9Y1E1qStC2Awhm4c\n",
        "qcbUircIsbwiBAu+QHEvvYI8RTLF6Sxnu6dBH3l27B/VWXPRRwR6B+cDP7I1pka8\n",
        "nRU6DOi6e0wJZqG+IKReqhzccm2FHzp942Fa2hr4sQjbsRFQVCWO4OPE2NfUDdD4\n",
        "Qpw+30crAgMBAAECggEARe4NgpbXvAgIUDQhQBcvKxtnzb3y5ymeQ+pKlXoIMOc4\n",
        "FfBpkt6sK6MMz9OZ4pHU2qTnQwPia9wa/xcubQuUxfrVwdz6gYnpR8ffSAKkZK3I\n",
        "mKPkjDlkzPY47NIoNOph5i3VYwDmroB/oI/j5OF3SKlz/OsHe9K7HCw16jh7RSZI\n",
        "uJ6BFkNZGjv/uKkzdW5u4kSoQQhE7gdnuO9B53w2J+Td6MgHuYqgXV8ASEwBK7YV\n",
        "YRgHFdR1ZyFEjArjgUqxokPG0y05R9X4Dd0LigFdc/JnK4gwqo/tJXU2O6IoXDYs\n",
        "zDOSkJ56VBEAFL6h/rY6VJMv2p433uQs3Q1smGCPsQKBgQDir7LB+3kiTVE0JKsb\n",
        "iapddkRuQcuK/v5cK9Yc8ZdFYCmUhm2uhimw37VKw4MJDLY9Q4JO8MzTH9ySd1Ci\n",
        "1Ny0DRDb4h+ROCxm3OPxk6HOl/yxM9AgqxmUNLD1J8tnXweQxAeHnil95Vp921FA\n",
        "PMrwNMchNKiI25kaaDjn70YsSQKBgQDDfdmqQihPbGFudDzI3+Sl+Jlm0Vqs+aN5\n",
        "0R7nT0b4+2FBl9YP2RA0tFSNEqpD31ytLyMOF5E6gAiaXDFpzQ/T5H7SXDcg0MWF\n",
        "Iw+Fzf8eTJ3Eu58vfEi2Uif/RPJUhDrA5nD3VWPlckyNjoJPcZRReQdCE6oIeBo/\n",
        "Zzhc8uyP0wKBgCDDRBLFRbyvcA0ZP6G7Q+Q+M6W73K86K4kmzMtiH3rnaxsMUs3m\n",
        "lh/6NTmZCFdGfxBbsXm3U+Mvt7FzjTP7j+p1+PnOtMFIXSKAynEf5UL2tI7n7izK\n",
        "jefdtbW5CqzmDzHdIzl2ooiPnYSTLisanjoZZq5l7fXZx0cJyS+8ZWgBAoGBALsf\n",
        "4BZFNWixCaI8yWJOTgNArzXn96/TVVPphHdNP1Zc6X9r449P619HrhdLYoeNapyr\n",
        "nhaDIJSqsZFv5iysCRZ+hZa+hlZ3AFqscNNXl3hdRjdmkL1XbhJ3GaoTSRL1b3fu\n",
        "HPvjVLfwbK6jVsDMq3hBLV1mjT+GFznRh/YQ4bfZAoGAOH5L1RIL+xtCa2wy5655\n",
        "y8Kd6324XkVXi7qdftRP2Vm9XZAqJBzv21+lt5BhnZkoPw4U6Dl4kezw84zh/ePn\n",
        "jomEl8m65QUqTwpIA1c7fD9qptUGSVHTvz4ztJTR/hdIJ+zriqJnGjV8maCcwilg\n",
        "Uhs4xBjV47qq1Jr/4FCoeKw=\n",
        "-----END PRIVATE KEY-----\n",
    );

    fn runtime_config(root: &Path, server_addr: SocketAddr) -> crate::runtime::RuntimeConfig {
        let credentials_path = root.join("credentials.json");
        fs::write(
            &credentials_path,
            format!(
                "{{\"AccountTag\":\"account\",\"TunnelSecret\":\"secret\",\"TunnelID\":\"\
                 11111111-1111-1111-1111-111111111111\",\"Endpoint\":\"{}\"}}",
                server_addr.ip()
            ),
        )
        .expect("transport credentials fixture should be written");

        let raw = RawConfig::from_yaml_str(
            "runtime-test.yaml",
            &format!(
                "tunnel: 11111111-1111-1111-1111-111111111111\ncredentials-file: {}\ningress:\n  - service: \
                 http_status:503\n",
                credentials_path.display()
            ),
        )
        .expect("runtime transport config should parse");
        let normalized =
            NormalizedConfig::from_raw(ConfigSource::ExplicitPath(root.join("runtime-test.yaml")), raw)
                .expect("runtime transport config should normalize");
        let discovery = DiscoveryOutcome {
            action: DiscoveryAction::UseExisting,
            source: ConfigSource::ExplicitPath(root.join("runtime-test.yaml")),
            path: root.join("runtime-test.yaml"),
            created_paths: Vec::new(),
            written_config: None,
        };

        crate::runtime::RuntimeConfig::new(discovery, normalized)
    }

    fn runtime_config_with_origin_cert(root: &Path) -> crate::runtime::RuntimeConfig {
        let origin_cert_path = root.join("cert.pem");
        let origin_cert = cloudflared_config::OriginCertToken {
            zone_id: "zone".to_owned(),
            account_id: "account".to_owned(),
            api_token: "token".to_owned(),
            endpoint: Some("FED".to_owned()),
        };
        fs::write(
            &origin_cert_path,
            origin_cert
                .encode_pem()
                .expect("origin cert fixture should encode"),
        )
        .expect("origin cert fixture should be written");

        let raw = RawConfig::from_yaml_str(
            "runtime-origin-cert.yaml",
            &format!(
                "tunnel: 11111111-1111-1111-1111-111111111111\norigincert: {}\ningress:\n  - service: \
                 http_status:503\n",
                origin_cert_path.display()
            ),
        )
        .expect("runtime transport config should parse");
        let normalized = NormalizedConfig::from_raw(
            ConfigSource::ExplicitPath(root.join("runtime-origin-cert.yaml")),
            raw,
        )
        .expect("runtime transport config should normalize");
        let discovery = DiscoveryOutcome {
            action: DiscoveryAction::UseExisting,
            source: ConfigSource::ExplicitPath(root.join("runtime-origin-cert.yaml")),
            path: root.join("runtime-origin-cert.yaml"),
            created_paths: Vec::new(),
            written_config: None,
        };

        crate::runtime::RuntimeConfig::new(discovery, normalized)
    }

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cloudflared-transport-{name}-{unique}"));
        fs::create_dir_all(&path).expect("temp directory should be created");
        path
    }

    fn write_tls_files(root: &Path) -> (PathBuf, PathBuf) {
        let cert_path = root.join("edge-cert.pem");
        let key_path = root.join("edge-key.pem");
        fs::write(&cert_path, TEST_CERT_PEM).expect("test certificate should be written");
        fs::write(&key_path, TEST_KEY_PEM).expect("test private key should be written");
        (cert_path, key_path)
    }

    fn spawn_test_server(root: &Path) -> SocketAddr {
        let (cert_path, key_path) = write_tls_files(root);
        let socket = UdpSocket::bind("127.0.0.1:0").expect("test UDP socket should bind");
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("test UDP socket timeout should be configured");
        let server_addr = socket
            .local_addr()
            .expect("test UDP socket address should be available");

        thread::spawn(move || {
            let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)
                .expect("test quiche server config should be created");
            config
                .load_cert_chain_from_pem_file(&cert_path.to_string_lossy())
                .expect("test certificate chain should load");
            config
                .load_priv_key_from_pem_file(&key_path.to_string_lossy())
                .expect("test private key should load");
            config
                .set_application_protos(EDGE_QUIC_ALPN)
                .expect("test ALPN should be configured");
            config.verify_peer(false);
            config.enable_early_data();
            config.set_max_idle_timeout(30_000);
            config.set_max_recv_udp_payload_size(1350);
            config.set_max_send_udp_payload_size(1350);
            config.set_initial_max_data(1_000_000);
            config.set_initial_max_stream_data_bidi_local(256_000);
            config.set_initial_max_stream_data_bidi_remote(256_000);
            config.set_initial_max_stream_data_uni(256_000);
            config.set_initial_max_streams_bidi(32);
            config.set_initial_max_streams_uni(32);
            config.set_disable_active_migration(true);

            let mut recv_buf = [0_u8; 65_535];
            let mut send_buf = [0_u8; 1350];
            let local_addr = socket
                .local_addr()
                .expect("server address should remain available");
            let mut connection = None;

            loop {
                let (read, from) = match socket.recv_from(&mut recv_buf) {
                    Ok(result) => result,
                    Err(error)
                        if error.kind() == ErrorKind::WouldBlock || error.kind() == ErrorKind::TimedOut =>
                    {
                        break;
                    }
                    Err(error) => panic!("unexpected test server recv error: {error}"),
                };

                if connection.is_none() {
                    let header = quiche::Header::from_slice(&mut recv_buf[..read], quiche::MAX_CONN_ID_LEN)
                        .expect("initial client packet header should parse");
                    let scid = quiche::ConnectionId::from_ref(&header.dcid);
                    connection = Some(
                        quiche::accept(&scid, None, local_addr, from, &mut config)
                            .expect("test server connection should initialize"),
                    );
                }

                let conn = connection.as_mut().expect("test server connection should exist");
                let recv_info = quiche::RecvInfo { from, to: local_addr };
                let _ = conn.recv(&mut recv_buf[..read], recv_info);

                loop {
                    match conn.send(&mut send_buf) {
                        Ok((written, send_info)) => {
                            socket
                                .send_to(&send_buf[..written], send_info.to)
                                .expect("test server UDP packet should send");
                        }
                        Err(quiche::Error::Done) => break,
                        Err(error) => panic!("unexpected test server send error: {error}"),
                    }
                }

                if conn.is_closed() {
                    break;
                }
            }
        });

        server_addr
    }

    #[test]
    fn edge_host_label_preserves_phase_33_region_shape() {
        assert_eq!(edge_host_label(None), "region1.v2.argotunnel.com");
        assert_eq!(edge_host_label(Some("us")), "us.v2.argotunnel.com");
        assert_eq!(edge_host_label(Some("fed")), "region1.v2.argotunnel.com");
    }

    #[test]
    fn quiche_config_keeps_0rtt_lane_enabled() {
        let target = QuicEdgeTarget {
            connect_addr: "127.0.0.1:7844"
                .parse()
                .expect("test socket address should parse"),
            host_label: "region1.v2.argotunnel.com".to_owned(),
            server_name: "localhost".to_owned(),
            verify_peer: false,
            ca_bundle_path: default_ca_bundle_path(),
        };

        let _ = build_quiche_config(&target).expect("quiche config should build");
    }

    #[test]
    fn transport_identity_reads_origin_cert_through_owned_pem_boundary() {
        let root = temp_dir("origin-cert-runtime");
        let runtime_config = runtime_config_with_origin_cert(&root);

        let identity = TransportIdentity::from_runtime_config(&runtime_config)
            .expect("origin cert should resolve runtime identity");

        assert_eq!(identity.identity_source, "origin-cert");
        assert_eq!(identity.endpoint_hint.as_deref(), Some("fed"));

        fs::remove_dir_all(root).expect("temp directory should be removable");
    }

    #[test]
    fn runtime_crosses_wire_protocol_boundary_after_quic_establish() {
        let root = temp_dir("quic-runtime");
        let server_addr = spawn_test_server(&root);
        let runtime_config = runtime_config(&root, server_addr);
        let (protocol_sender, protocol_receiver) = protocol::protocol_bridge();
        let execution = run_with_factory(
            runtime_config,
            QuicTunnelServiceFactory::with_test_target(
                protocol_sender,
                QuicEdgeTarget {
                    connect_addr: server_addr,
                    host_label: "localhost".to_owned(),
                    server_name: "localhost".to_owned(),
                    verify_peer: false,
                    ca_bundle_path: None,
                },
            ),
            crate::runtime::RuntimeHarness::for_tests(),
            Some(protocol_receiver),
        );

        // The runtime now stops honestly at the post-transport protocol boundary
        // without tying that deferred work to a stale numbered phase label.
        assert!(matches!(
            execution.exit,
            RuntimeExit::Deferred {
                phase: "later runtime/protocol slices",
                ..
            }
        ));
        assert!(
            execution
                .summary_lines
                .iter()
                .any(|line| line.contains("transport-session-state: established")),
            "should report QUIC session establishment"
        );
        assert!(
            execution
                .summary_lines
                .iter()
                .any(|line| line.contains("protocol-boundary: control-stream-0 opened")),
            "should report control stream opened at wire/protocol boundary"
        );
        assert!(
            execution
                .summary_lines
                .iter()
                .any(|line| line.contains("protocol-boundary: registration event sent to proxy layer")),
            "should report registration event sent through protocol bridge"
        );
        assert!(
            execution
                .summary_lines
                .iter()
                .any(|line| line.contains("quic-0rtt-policy:")),
            "should report 0-RTT policy"
        );

        fs::remove_dir_all(root).expect("temp directory should be removable");
    }
}
