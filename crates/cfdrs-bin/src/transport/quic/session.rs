use std::net::SocketAddr;

use quiche::ConnectionId;
use tokio::net::UdpSocket;
use uuid::Uuid;

use super::MAX_DATAGRAM_SIZE;
use super::edge::{PeerVerification, QuicEdgeTarget, wildcard_bind_addr};

const EDGE_QUIC_ALPN: &[&[u8]] = &[cfdrs_cdc::protocol::EDGE_QUIC_ALPN.as_bytes()];
const QUIC_IDLE_TIMEOUT_MS: u64 = 30_000;

/// Datagram receive queue capacity.
///
/// Matches Go's `demuxChanCapacity` (16) in `quic/v3/muxer.go`.
const DGRAM_RECV_QUEUE_LEN: usize = 16;

/// Datagram send queue capacity.
const DGRAM_SEND_QUEUE_LEN: usize = 16;

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

    match &target.verification {
        PeerVerification::Verified { ca_bundle_path } => {
            config.verify_peer(true);
            config
                .load_verify_locations_from_file(&ca_bundle_path.to_string_lossy())
                .map_err(|error| {
                    format!(
                        "failed to load CA bundle {} for QUIC edge verification: {error}",
                        ca_bundle_path.display()
                    )
                })?;
        }
        PeerVerification::Unverified => {
            config.verify_peer(false);
        }
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
    config.enable_dgram(true, DGRAM_RECV_QUEUE_LEN, DGRAM_SEND_QUEUE_LEN);

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
                send_packet(socket, &buffer[..written], send_info.to).await?;
            }
            Err(quiche::Error::Done) => return Ok(()),
            Err(error) => {
                return Err(format!("quiche send failed while flushing egress: {error}"));
            }
        }
    }
}

async fn send_packet(socket: &UdpSocket, data: &[u8], to: SocketAddr) -> Result<(), String> {
    socket
        .send_to(data, to)
        .await
        .map_err(|error| format!("failed to send UDP packet to {to}: {error}"))?;
    Ok(())
}
