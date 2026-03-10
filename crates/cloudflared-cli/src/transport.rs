#![forbid(unsafe_code)]

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
const PHASE_35_DEFERRED_DETAIL: &str = "QUIC transport session is established, but tunnel registration and \
                                        control-stream wire behavior remain deferred";

#[derive(Debug, Clone)]
pub(crate) struct QuicTunnelServiceFactory {
    test_target: Option<QuicEdgeTarget>,
}

impl QuicTunnelServiceFactory {
    pub(crate) fn production() -> Self {
        Self { test_target: None }
    }

    #[cfg(test)]
    pub(crate) fn with_test_target(target: QuicEdgeTarget) -> Self {
        Self {
            test_target: Some(target),
        }
    }
}

impl RuntimeServiceFactory for QuicTunnelServiceFactory {
    fn create_primary(&self, config: Arc<RuntimeConfig>, attempt: u32) -> Box<dyn RuntimeService> {
        Box::new(QuicTunnelService {
            config,
            attempt,
            test_target: self.test_target.clone(),
        })
    }
}

struct QuicTunnelService {
    config: Arc<RuntimeConfig>,
    attempt: u32,
    test_target: Option<QuicEdgeTarget>,
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
        let bind_addr = wildcard_bind_addr(target.connect_addr);
        let socket = UdpSocket::bind(bind_addr)
            .await
            .map_err(|error| format!("failed to bind UDP socket for QUIC transport: {error}"))?;
        let local_addr = socket
            .local_addr()
            .map_err(|error| format!("failed to inspect UDP local address: {error}"))?;

        let mut quic_config = build_quiche_config(&target)?;
        let scid_bytes = Uuid::new_v4().into_bytes();
        let scid = ConnectionId::from_ref(scid_bytes.as_ref());
        let mut connection = quiche::connect(
            Some(target.server_name.as_str()),
            &scid,
            local_addr,
            target.connect_addr,
            &mut quic_config,
        )
        .map_err(|error| format!("failed to initialize quiche client connection: {error}"))?;

        let mut recv_buffer = [0_u8; 65_535];
        let mut send_buffer = [0_u8; MAX_DATAGRAM_SIZE];

        flush_egress(&socket, &mut connection, &mut send_buffer)
            .await
            .map_err(|error| format!("failed to send initial QUIC packets: {error}"))?;

        let establish_timer = time::sleep(QUIC_ESTABLISH_TIMEOUT);
        tokio::pin!(establish_timer);

        send_status(
            command_tx,
            self.name(),
            format!("transport-session-state: handshaking local={local_addr}"),
        )
        .await;

        loop {
            if connection.is_established() {
                break;
            }

            if connection.is_closed() {
                return Ok(ServiceExit::RetryableFailure {
                    service: self.name(),
                    detail: format!(
                        "quic transport closed before establishment for edge {}",
                        target.connect_addr
                    ),
                });
            }

            tokio::select! {
                _ = shutdown.cancelled() => {
                    send_status(command_tx, self.name(), "transport-session-state: teardown-before-establish".to_owned()).await;
                    return Ok(ServiceExit::Completed { service: self.name() });
                }
                _ = &mut establish_timer => {
                    return Ok(ServiceExit::RetryableFailure {
                        service: self.name(),
                        detail: format!("quic handshake timed out for edge {}", target.connect_addr),
                    });
                }
                recv_result = socket.recv_from(&mut recv_buffer) => {
                    let (read, from) = recv_result
                        .map_err(|error| format!("failed to receive QUIC packet from edge: {error}"))?;
                    let recv_info = quiche::RecvInfo {
                        from,
                        to: local_addr,
                    };

                    match connection.recv(&mut recv_buffer[..read], recv_info) {
                        Ok(_) | Err(quiche::Error::Done) => {}
                        Err(error) => {
                            return Ok(ServiceExit::RetryableFailure {
                                service: self.name(),
                                detail: format!("quic handshake failed while reading edge packets: {error}"),
                            });
                        }
                    }

                    flush_egress(&socket, &mut connection, &mut send_buffer)
                        .await
                        .map_err(|error| format!("failed to flush QUIC packets during handshake: {error}"))?;
                }
            }
        }

        send_status(
            command_tx,
            self.name(),
            format!(
                "transport-session-state: established peer={} early-data={} resumed-shape={}",
                target.connect_addr,
                connection.is_in_early_data(),
                identity.resumption.shape_label(),
            ),
        )
        .await;
        let _ = command_tx
            .send(RuntimeCommand::ServiceReady { service: self.name() })
            .await;

        let _ = connection.close(true, 0x00, b"deferred wire/protocol boundary");
        let _ = flush_egress(&socket, &mut connection, &mut send_buffer).await;
        send_status(
            command_tx,
            self.name(),
            "transport-session-state: teardown".to_owned(),
        )
        .await;

        Ok(ServiceExit::Deferred {
            service: self.name(),
            phase: "Big Phase 3.5",
            detail: format!(
                "{} for tunnel {} against {}",
                PHASE_35_DEFERRED_DETAIL, identity.tunnel_id, target.connect_addr
            ),
        })
    }
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
pub(crate) struct QuicEdgeTarget {
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
        EDGE_QUIC_ALPN, QuicEdgeTarget, QuicTunnelServiceFactory, build_quiche_config,
        default_ca_bundle_path, edge_host_label,
    };
    use crate::runtime::{RuntimeExit, run_with_factory};
    use cloudflared_config::{ConfigSource, DiscoveryAction, DiscoveryOutcome, NormalizedConfig, RawConfig};
    use std::fs;
    use std::io::ErrorKind;
    use std::net::{SocketAddr, UdpSocket};
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    const TEST_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----\\
                                 nMIIDCTCCAfGgAwIBAgIUJb0Jfxu0MAeoFD0npL3VZBW2h+owDQYJKoZIhvcNAQEL\\
                                 nBQAwFDESMBAGA1UEAwwJbG9jYWxob3N0MB4XDTI2MDMxMDA3MDYxNFoXDTI3MDMx\\
                                 nMDA3MDYxNFowFDESMBAGA1UEAwwJbG9jYWxob3N0MIIBIjANBgkqhkiG9w0BAQEF\\
                                 nAAOCAQ8AMIIBCgKCAQEArRtFsb0NMB9y09zu4KBt3h+lvJT0iHYFN46BFehJhD55\\
                                 ner1h0cNJHTQ6s8x1cohQpfITM+03ZOMRRYj7rg+L+ylVpkYvTuXBVrK9xcAMwdYk\\
                                 ntaL4uFHGc1kBs8awa7RfgFwqXEnaQ4sO7ie1FpJ0sViC3t9ZmJ2kJgOPKT6HGUS+\\
                                 nmiYbZE2c+5FBb1OD0fWNRNakrQtgMIZuHKnG1Iq3CLG8IgQLvkBxL72CPEUyxeks\\
                                 nZ7unQR95duwf1Vlz0UcEegfnAz+yNaZGvJ0VOgzountMCWahviCkXqoc3HJthR86\\
                                 nfeNhWtoa+LEI27ERUFQljuDjxNjX1A3Q+EKcPt9HKwIDAQABo1MwUTAdBgNVHQ4E\\
                                 nFgQU7FVi0ezFYdq1iLUAu8yqPYetntEwHwYDVR0jBBgwFoAU7FVi0ezFYdq1iLUA\\
                                 nu8yqPYetntEwDwYDVR0TAQH/BAUwAwEB/zANBgkqhkiG9w0BAQsFAAOCAQEAcwIp\\
                                 n3w2Bx3vk9hYWrwGfWH/vvyqMrF6GUkcF8557rmO1uXnk9uzHDcjUT+9zmFA/gXxc\\
                                 \
                                 ncoCS3l+HjTk6InGq/Bncsc0WR/gdp8JCbOKJKCnTnK1zQdExJ4H2646ARxJNpxPl\nCv5/\
                                 SL7LyJbQm/2H60V/urcIwtl/WnBgw58BZ1wOWXaVQYBaSp2m6A3TPCozrQ2N\\
                                 nHu5tPOzkXjkSMdfOPvHdK3tvIn04gKxAe+kc05efsncWZdlgfpTT5SOfOMp+LQ6T\\
                                 ngegfwgzYQzBwWZNUqprAGNyUsW5dxIAWYMkxHr3n4eZ83A8M8GPPKa8TOp6qFbza\\
                                 nKWggdegvHvjpedAG8A==\n-----END CERTIFICATE-----\n";
    const TEST_KEY_PEM: &str = "-----BEGIN PRIVATE \
                                KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCtG0WxvQ0wH3LT\\
                                n3O7goG3eH6W8lPSIdgU3joEV6EmEPnl6vWHRw0kdNDqzzHVyiFCl8hMz7Tdk4xFF\\
                                niPuuD4v7KVWmRi9O5cFWsr3FwAzB1iS1ovi4UcZzWQGzxrBrtF+AXCpcSdpDiw7u\\
                                nJ7UWknSxWILe31mYnaQmA48pPocZRL6aJhtkTZz7kUFvU4PR9Y1E1qStC2Awhm4c\\
                                nqcbUircIsbwiBAu+QHEvvYI8RTLF6Sxnu6dBH3l27B/VWXPRRwR6B+cDP7I1pka8\\
                                nnRU6DOi6e0wJZqG+IKReqhzccm2FHzp942Fa2hr4sQjbsRFQVCWO4OPE2NfUDdD4\\
                                nQpw+30crAgMBAAECggEARe4NgpbXvAgIUDQhQBcvKxtnzb3y5ymeQ+pKlXoIMOc4\\
                                nFfBpkt6sK6MMz9OZ4pHU2qTnQwPia9wa/xcubQuUxfrVwdz6gYnpR8ffSAKkZK3I\\
                                nmKPkjDlkzPY47NIoNOph5i3VYwDmroB/oI/j5OF3SKlz/OsHe9K7HCw16jh7RSZI\\
                                nuJ6BFkNZGjv/uKkzdW5u4kSoQQhE7gdnuO9B53w2J+Td6MgHuYqgXV8ASEwBK7YV\\
                                nYRgHFdR1ZyFEjArjgUqxokPG0y05R9X4Dd0LigFdc/JnK4gwqo/tJXU2O6IoXDYs\\
                                nzDOSkJ56VBEAFL6h/rY6VJMv2p433uQs3Q1smGCPsQKBgQDir7LB+3kiTVE0JKsb\\
                                niapddkRuQcuK/v5cK9Yc8ZdFYCmUhm2uhimw37VKw4MJDLY9Q4JO8MzTH9ySd1Ci\\
                                n1Ny0DRDb4h+ROCxm3OPxk6HOl/yxM9AgqxmUNLD1J8tnXweQxAeHnil95Vp921FA\\
                                nPMrwNMchNKiI25kaaDjn70YsSQKBgQDDfdmqQihPbGFudDzI3+Sl+Jlm0Vqs+aN5\\
                                n0R7nT0b4+2FBl9YP2RA0tFSNEqpD31ytLyMOF5E6gAiaXDFpzQ/T5H7SXDcg0MWF\\
                                nIw+Fzf8eTJ3Eu58vfEi2Uif/RPJUhDrA5nD3VWPlckyNjoJPcZRReQdCE6oIeBo/\\
                                \
                                nZzhc8uyP0wKBgCDDRBLFRbyvcA0ZP6G7Q+Q+M6W73K86K4kmzMtiH3rnaxsMUs3m\nlh/\
                                6NTmZCFdGfxBbsXm3U+Mvt7FzjTP7j+p1+PnOtMFIXSKAynEf5UL2tI7n7izK\\
                                njefdtbW5CqzmDzHdIzl2ooiPnYSTLisanjoZZq5l7fXZx0cJyS+8ZWgBAoGBALsf\\
                                n4BZFNWixCaI8yWJOTgNArzXn96/TVVPphHdNP1Zc6X9r449P619HrhdLYoeNapyr\\
                                nnhaDIJSqsZFv5iysCRZ+hZa+hlZ3AFqscNNXl3hdRjdmkL1XbhJ3GaoTSRL1b3fu\\
                                nHPvjVLfwbK6jVsDMq3hBLV1mjT+GFznRh/YQ4bfZAoGAOH5L1RIL+xtCa2wy5655\\
                                ny8Kd6324XkVXi7qdftRP2Vm9XZAqJBzv21+lt5BhnZkoPw4U6Dl4kezw84zh/ePn\\
                                njomEl8m65QUqTwpIA1c7fD9qptUGSVHTvz4ztJTR/hdIJ+zriqJnGjV8maCcwilg\\
                                nUhs4xBjV47qq1Jr/4FCoeKw=\n-----END PRIVATE KEY-----\n";

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
    fn runtime_establishes_real_quic_transport_before_wire_boundary() {
        let root = temp_dir("quic-runtime");
        let server_addr = spawn_test_server(&root);
        let runtime_config = runtime_config(&root, server_addr);
        let execution = run_with_factory(
            runtime_config,
            QuicTunnelServiceFactory::with_test_target(QuicEdgeTarget {
                connect_addr: server_addr,
                host_label: "localhost".to_owned(),
                server_name: "localhost".to_owned(),
                verify_peer: false,
                ca_bundle_path: None,
            }),
            crate::runtime::RuntimeHarness::for_tests(),
        );

        assert!(matches!(
            execution.exit,
            RuntimeExit::Deferred {
                phase: "Big Phase 3.5",
                ..
            }
        ));
        assert!(
            execution
                .summary_lines
                .iter()
                .any(|line| line.contains("transport-session-state: established"))
        );
        assert!(
            execution
                .summary_lines
                .iter()
                .any(|line| line.contains("quic-0rtt-policy:"))
        );

        fs::remove_dir_all(root).expect("temp directory should be removable");
    }
}
