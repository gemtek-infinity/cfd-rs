use super::QuicTunnelServiceFactory;
use super::edge::{PeerVerification, QuicEdgeTarget, edge_host_label};
use super::identity::TransportIdentity;
use super::session::build_quiche_config;
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

fn build_test_quiche_server_config(cert_path: &Path, key_path: &Path) -> quiche::Config {
    let mut config =
        quiche::Config::new(quiche::PROTOCOL_VERSION).expect("test quiche server config should be created");
    config
        .load_cert_chain_from_pem_file(&cert_path.to_string_lossy())
        .expect("test certificate chain should load");
    config
        .load_priv_key_from_pem_file(&key_path.to_string_lossy())
        .expect("test private key should load");
    config
        .set_application_protos(&[b"argotunnel"])
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
    config
}

fn run_test_quic_server(socket: UdpSocket, mut config: quiche::Config) {
    let mut recv_buf = [0_u8; 65_535];
    let mut send_buf = [0_u8; 1350];
    let local_addr = socket
        .local_addr()
        .expect("server address should remain available");
    let mut connection = None;

    loop {
        let (read, from) = match socket.recv_from(&mut recv_buf) {
            Ok(result) => result,
            Err(error) if error.kind() == ErrorKind::WouldBlock || error.kind() == ErrorKind::TimedOut => {
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

        flush_test_server_egress(conn, &socket, &mut send_buf);

        if conn.is_closed() {
            break;
        }
    }
}

fn flush_test_server_egress(conn: &mut quiche::Connection, socket: &UdpSocket, send_buf: &mut [u8]) {
    loop {
        match conn.send(send_buf) {
            Ok((written, send_info)) => {
                socket
                    .send_to(&send_buf[..written], send_info.to)
                    .expect("test server UDP packet should send");
            }
            Err(quiche::Error::Done) => break,
            Err(error) => panic!("unexpected test server send error: {error}"),
        }
    }
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

    let config = build_test_quiche_server_config(&cert_path, &key_path);
    thread::spawn(move || run_test_quic_server(socket, config));

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
        verification: PeerVerification::Unverified,
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
                verification: PeerVerification::Unverified,
            },
        ),
        crate::runtime::HarnessBuilder::for_tests().build(),
        Some(protocol_receiver),
    );

    assert!(
        matches!(
            execution.exit,
            RuntimeExit::Deferred {
                phase: "later runtime/protocol slices",
                ..
            }
        ),
        "unexpected runtime exit: {:?}",
        execution.exit
    );
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
