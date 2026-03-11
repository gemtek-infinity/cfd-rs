use std::net::SocketAddr;

use quiche::ConnectionId;
use tokio::net::UdpSocket;
use uuid::Uuid;

use super::MAX_DATAGRAM_SIZE;
use super::edge::{QuicEdgeTarget, wildcard_bind_addr};

const EDGE_QUIC_ALPN: &[&[u8]] = &[b"argotunnel"];
const QUIC_IDLE_TIMEOUT_MS: u64 = 30_000;

pub(super) struct QuicSessionState {
    pub(super) socket: UdpSocket,
    pub(super) local_addr: SocketAddr,
    pub(super) connection: quiche::Connection,
    pub(super) recv_buffer: Box<[u8; 65_535]>,
    pub(super) send_buffer: Box<[u8; MAX_DATAGRAM_SIZE]>,
}

impl QuicSessionState {
    pub(super) async fn initialize(target: &QuicEdgeTarget) -> Result<Self, String> {
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

        Ok(Self {
            socket,
            local_addr,
            connection,
            recv_buffer: Box::new([0_u8; 65_535]),
            send_buffer: Box::new([0_u8; MAX_DATAGRAM_SIZE]),
        })
    }
}

pub(super) fn build_quiche_config(target: &QuicEdgeTarget) -> Result<quiche::Config, String> {
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

pub(super) async fn flush_egress(
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
