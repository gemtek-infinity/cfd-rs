//! Cap'n Proto codec for registration RPC types.
//!
//! Bridges between the Rust domain types in `registration` and the
//! generated Cap'n Proto bindings from `tunnelrpc.capnp`.
//!
//! Baseline truth:
//! - Schema: `baseline-2026.2.0/tunnelrpc/proto/tunnelrpc.capnp`
//! - Go codec: `baseline-2026.2.0/tunnelrpc/pogs/registration_server.go`

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use uuid::Uuid;

use crate::registration::{
    ClientInfo, ConnectionDetails, ConnectionError, ConnectionOptions, ConnectionResponse,
    RegisterUdpSessionRequest, RegisterUdpSessionResponse, TunnelAuth, UpdateConfigurationRequest,
    UpdateConfigurationResponse, UpdateLocalConfigurationRequest,
};
use crate::tunnelrpc_capnp;

/// Read a Cap'n Proto text field as an owned String.
///
/// Cap'n Proto text may contain invalid UTF-8; this converts the
/// `Utf8Error` into a `capnp::Error` for uniform error propagation.
pub(crate) fn read_capnp_text(text: ::capnp::text::Reader<'_>) -> ::capnp::Result<String> {
    text.to_string()
        .map_err(|e| ::capnp::Error::failed(format!("invalid UTF-8 in capnp text: {e}")))
}

// ---------------------------------------------------------------------------
// TunnelAuth — @0x9496331ab9cd463f
// ---------------------------------------------------------------------------

impl TunnelAuth {
    /// Write auth fields into a Cap'n Proto `TunnelAuth` builder.
    pub fn marshal_capnp(&self, mut builder: tunnelrpc_capnp::tunnel_auth::Builder<'_>) {
        builder.set_account_tag(&self.account_tag);
        builder.set_tunnel_secret(&self.tunnel_secret);
    }

    /// Read auth from a Cap'n Proto `TunnelAuth` reader.
    pub fn unmarshal_capnp(reader: tunnelrpc_capnp::tunnel_auth::Reader<'_>) -> ::capnp::Result<Self> {
        Ok(Self {
            account_tag: read_capnp_text(reader.get_account_tag()?)?,
            tunnel_secret: reader.get_tunnel_secret()?.to_vec(),
        })
    }
}

// ---------------------------------------------------------------------------
// ClientInfo — @0x83ced0145b2f114b
// ---------------------------------------------------------------------------

impl ClientInfo {
    /// Write client info into a Cap'n Proto `ClientInfo` builder.
    pub fn marshal_capnp(&self, mut builder: tunnelrpc_capnp::client_info::Builder<'_>) {
        builder.set_client_id(&self.client_id);

        {
            let mut features = builder.reborrow().init_features(self.features.len() as u32);

            for (i, feature) in self.features.iter().enumerate() {
                features.set(i as u32, feature);
            }
        }

        builder.set_version(&self.version);
        builder.set_arch(&self.arch);
    }

    /// Read client info from a Cap'n Proto `ClientInfo` reader.
    pub fn unmarshal_capnp(reader: tunnelrpc_capnp::client_info::Reader<'_>) -> ::capnp::Result<Self> {
        let features_reader = reader.get_features()?;
        let mut features = Vec::with_capacity(features_reader.len() as usize);

        for i in 0..features_reader.len() {
            features.push(read_capnp_text(features_reader.get(i)?)?);
        }

        Ok(Self {
            client_id: reader.get_client_id()?.to_vec(),
            features,
            version: read_capnp_text(reader.get_version()?)?,
            arch: read_capnp_text(reader.get_arch()?)?,
        })
    }
}

// ---------------------------------------------------------------------------
// ConnectionOptions — @0xb4bf9861fe035d04
// ---------------------------------------------------------------------------

/// Parse an IP address from raw bytes as stored in Cap'n Proto Data fields.
///
/// Go's `net.IP` is `[]byte` and is stored directly as Cap'n Proto Data.
/// 4 bytes → IPv4, 16 bytes → IPv6, empty or other → None.
fn parse_ip_from_bytes(bytes: &[u8]) -> Option<IpAddr> {
    match bytes.len() {
        4 => {
            let octets: [u8; 4] = bytes.try_into().expect("length already checked");
            Some(IpAddr::V4(Ipv4Addr::from(octets)))
        }
        16 => {
            let octets: [u8; 16] = bytes.try_into().expect("length already checked");
            Some(IpAddr::V6(Ipv6Addr::from(octets)))
        }
        _ => None,
    }
}

impl ConnectionOptions {
    /// Write options into a Cap'n Proto `ConnectionOptions` builder.
    pub fn marshal_capnp(&self, mut builder: tunnelrpc_capnp::connection_options::Builder<'_>) {
        self.client.marshal_capnp(builder.reborrow().init_client());

        if let Some(ip) = &self.origin_local_ip {
            match ip {
                IpAddr::V4(v4) => builder.set_origin_local_ip(&v4.octets()),
                IpAddr::V6(v6) => builder.set_origin_local_ip(&v6.octets()),
            }
        }

        builder.set_replace_existing(self.replace_existing);
        builder.set_compression_quality(self.compression_quality);
        builder.set_num_previous_attempts(self.num_previous_attempts);
    }

    /// Read options from a Cap'n Proto `ConnectionOptions` reader.
    pub fn unmarshal_capnp(reader: tunnelrpc_capnp::connection_options::Reader<'_>) -> ::capnp::Result<Self> {
        let client = ClientInfo::unmarshal_capnp(reader.get_client()?)?;

        let origin_local_ip = if reader.has_origin_local_ip() {
            parse_ip_from_bytes(reader.get_origin_local_ip()?)
        } else {
            None
        };

        Ok(Self {
            client,
            origin_local_ip,
            replace_existing: reader.get_replace_existing(),
            compression_quality: reader.get_compression_quality(),
            num_previous_attempts: reader.get_num_previous_attempts(),
        })
    }
}

// ---------------------------------------------------------------------------
// ConnectionDetails — @0xb5f39f082b9ac18a
// ---------------------------------------------------------------------------

impl ConnectionDetails {
    /// Write details into a Cap'n Proto `ConnectionDetails` builder.
    pub fn marshal_capnp(&self, mut builder: tunnelrpc_capnp::connection_details::Builder<'_>) {
        builder.set_uuid(self.uuid.as_bytes());
        builder.set_location_name(&self.location);
        builder.set_tunnel_is_remotely_managed(self.is_remotely_managed);
    }

    /// Read details from a Cap'n Proto `ConnectionDetails` reader.
    pub fn unmarshal_capnp(reader: tunnelrpc_capnp::connection_details::Reader<'_>) -> ::capnp::Result<Self> {
        let uuid_bytes = reader.get_uuid()?;
        let uuid = Uuid::from_slice(uuid_bytes)
            .map_err(|e| ::capnp::Error::failed(format!("invalid UUID bytes: {e}")))?;

        Ok(Self {
            uuid,
            location: read_capnp_text(reader.get_location_name()?)?,
            is_remotely_managed: reader.get_tunnel_is_remotely_managed(),
        })
    }
}

// ---------------------------------------------------------------------------
// ConnectionError — @0xf5f383d2785edb86
// ---------------------------------------------------------------------------

impl ConnectionError {
    /// Write error into a Cap'n Proto `ConnectionError` builder.
    pub fn marshal_capnp(&self, mut builder: tunnelrpc_capnp::connection_error::Builder<'_>) {
        builder.set_cause(&self.cause);
        builder.set_retry_after(self.retry_after_ns);
        builder.set_should_retry(self.should_retry);
    }

    /// Read error from a Cap'n Proto `ConnectionError` reader.
    pub fn unmarshal_capnp(reader: tunnelrpc_capnp::connection_error::Reader<'_>) -> ::capnp::Result<Self> {
        Ok(Self {
            cause: read_capnp_text(reader.get_cause()?)?,
            retry_after_ns: reader.get_retry_after(),
            should_retry: reader.get_should_retry(),
        })
    }
}

// ---------------------------------------------------------------------------
// ConnectionResponse — @0xdbaa9d03d52b62dc (union)
// ---------------------------------------------------------------------------

impl ConnectionResponse {
    /// Write response into a Cap'n Proto `ConnectionResponse` builder.
    pub fn marshal_capnp(&self, builder: tunnelrpc_capnp::connection_response::Builder<'_>) {
        let result_builder = builder.init_result();

        match self {
            Self::Error(err) => {
                err.marshal_capnp(result_builder.init_error());
            }
            Self::Success(details) => {
                details.marshal_capnp(result_builder.init_connection_details());
            }
        }
    }

    /// Read response from a Cap'n Proto `ConnectionResponse` reader.
    pub fn unmarshal_capnp(
        reader: tunnelrpc_capnp::connection_response::Reader<'_>,
    ) -> ::capnp::Result<Self> {
        use crate::tunnelrpc_capnp::connection_response::result::Which;

        match reader.get_result().which()? {
            Which::Error(error_reader) => {
                let error = ConnectionError::unmarshal_capnp(error_reader?)?;
                Ok(Self::Error(error))
            }
            Which::ConnectionDetails(details_reader) => {
                let details = ConnectionDetails::unmarshal_capnp(details_reader?)?;
                Ok(Self::Success(details))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// UpdateLocalConfigurationRequest — CDC-008
// (RegistrationServer.updateLocalConfiguration takes `config: Data` inline,
//  not a struct wrapper.  We provide marshal/unmarshal that write into the
//  RPC params builder and read from the params reader as a convenience.)
// ---------------------------------------------------------------------------

impl UpdateLocalConfigurationRequest {
    /// Encode the config payload into Cap'n Proto bytes suitable for passing
    /// as the `config: Data` parameter of `updateLocalConfiguration`.
    pub fn to_capnp_bytes(&self) -> Vec<u8> {
        let mut msg = ::capnp::message::Builder::new_default();
        {
            let mut root = msg.init_root::<tunnelrpc_capnp::register_udp_session_response::Builder<'_>>();
            // Re-use a top-level struct just as an envelope — we only need
            // the raw bytes written to a capnp message for transport.
            // Instead, write a standalone Data segment.
            let _ = root.reborrow();
        }
        // For an RPC `Data` parameter, the raw bytes are passed directly.
        // This helper just wraps the payload for consistency.
        self.config.clone()
    }

    /// Build from raw config bytes received from the RPC parameter.
    pub fn from_config_bytes(config: &[u8]) -> Self {
        Self {
            config: config.to_vec(),
        }
    }
}

// ---------------------------------------------------------------------------
// RegisterUdpSessionResponse — @0xab6d5210c1f26687; CDC-009
// ---------------------------------------------------------------------------

impl RegisterUdpSessionResponse {
    /// Write into a Cap'n Proto `RegisterUdpSessionResponse` builder.
    pub fn marshal_capnp(&self, mut builder: tunnelrpc_capnp::register_udp_session_response::Builder<'_>) {
        builder.set_err(&self.err);
        builder.set_spans(&self.spans);
    }

    /// Read from a Cap'n Proto `RegisterUdpSessionResponse` reader.
    pub fn unmarshal_capnp(
        reader: tunnelrpc_capnp::register_udp_session_response::Reader<'_>,
    ) -> ::capnp::Result<Self> {
        Ok(Self {
            err: read_capnp_text(reader.get_err()?)?,
            spans: reader.get_spans()?.to_vec(),
        })
    }
}

// ---------------------------------------------------------------------------
// RegisterUdpSessionRequest — CDC-009 (RPC params, not a standalone struct)
// The RPC signature is:
//   registerUdpSession(sessionId: Data, dstIp: Data, dstPort: UInt16,
//                      closeAfterIdleHint: Int64, traceContext: Text)
// We provide encode/decode over a raw capnp message for testing and
// downstream codec reuse when capnp-rpc is admitted.
// ---------------------------------------------------------------------------

impl RegisterUdpSessionRequest {
    /// Encode the request fields into a vector of bytes for downstream
    /// codec or testing use.  Since the RPC params are inline (not a named
    /// struct), we store them as simple field accessors.
    pub fn session_id_bytes(&self) -> Vec<u8> {
        self.session_id.as_bytes().to_vec()
    }

    /// Build from raw RPC-parameter values.
    pub fn from_rpc_params(
        session_id: &[u8],
        dst_ip: &[u8],
        dst_port: u16,
        close_after_idle_hint_ns: i64,
        trace_context: &str,
    ) -> Option<Self> {
        let uuid = Uuid::from_slice(session_id).ok()?;
        Some(Self {
            session_id: uuid,
            dst_ip: dst_ip.to_vec(),
            dst_port,
            close_after_idle_hint_ns,
            trace_context: trace_context.to_owned(),
        })
    }
}

// ---------------------------------------------------------------------------
// UpdateConfigurationResponse — @0xdb58ff694ba05cf9; CDC-010
// ---------------------------------------------------------------------------

impl UpdateConfigurationResponse {
    /// Write into a Cap'n Proto `UpdateConfigurationResponse` builder.
    pub fn marshal_capnp(&self, mut builder: tunnelrpc_capnp::update_configuration_response::Builder<'_>) {
        builder.set_latest_applied_version(self.latest_applied_version);
        builder.set_err(&self.err);
    }

    /// Read from a Cap'n Proto `UpdateConfigurationResponse` reader.
    pub fn unmarshal_capnp(
        reader: tunnelrpc_capnp::update_configuration_response::Reader<'_>,
    ) -> ::capnp::Result<Self> {
        Ok(Self {
            latest_applied_version: reader.get_latest_applied_version(),
            err: read_capnp_text(reader.get_err()?)?,
        })
    }
}

// ---------------------------------------------------------------------------
// UpdateConfigurationRequest — CDC-010 (RPC params: version: Int32, config:
// Data)
// ---------------------------------------------------------------------------

impl UpdateConfigurationRequest {
    /// Build from raw RPC-parameter values.
    pub fn from_rpc_params(version: i32, config: &[u8]) -> Self {
        Self {
            version,
            config: config.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- TunnelAuth -------------------------------------------------------

    fn round_trip_tunnel_auth(auth: &TunnelAuth) -> TunnelAuth {
        let mut msg = ::capnp::message::Builder::new_default();
        auth.marshal_capnp(msg.init_root());
        let reader = msg
            .get_root_as_reader::<tunnelrpc_capnp::tunnel_auth::Reader<'_>>()
            .expect("get_root_as_reader");
        TunnelAuth::unmarshal_capnp(reader).expect("unmarshal")
    }

    #[test]
    fn tunnel_auth_round_trip() {
        let auth = TunnelAuth {
            account_tag: "my-account".into(),
            tunnel_secret: vec![0xde, 0xad, 0xbe, 0xef],
        };

        assert_eq!(auth, round_trip_tunnel_auth(&auth));
    }

    #[test]
    fn tunnel_auth_empty_fields() {
        let auth = TunnelAuth {
            account_tag: String::new(),
            tunnel_secret: Vec::new(),
        };

        assert_eq!(auth, round_trip_tunnel_auth(&auth));
    }

    // -- ClientInfo -------------------------------------------------------

    fn round_trip_client_info(info: &ClientInfo) -> ClientInfo {
        let mut msg = ::capnp::message::Builder::new_default();
        info.marshal_capnp(msg.init_root());
        let reader = msg
            .get_root_as_reader::<tunnelrpc_capnp::client_info::Reader<'_>>()
            .expect("get_root_as_reader");
        ClientInfo::unmarshal_capnp(reader).expect("unmarshal")
    }

    #[test]
    fn client_info_round_trip() {
        let id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid");
        let info = ClientInfo::for_current_platform(id);

        assert_eq!(info, round_trip_client_info(&info));
    }

    #[test]
    fn client_info_empty_features() {
        let info = ClientInfo {
            client_id: vec![0u8; 16],
            features: Vec::new(),
            version: "1.0.0".into(),
            arch: "linux_amd64".into(),
        };

        assert_eq!(info, round_trip_client_info(&info));
    }

    // -- ConnectionOptions ------------------------------------------------

    fn round_trip_options(opts: &ConnectionOptions) -> ConnectionOptions {
        let mut msg = ::capnp::message::Builder::new_default();
        opts.marshal_capnp(msg.init_root());
        let reader = msg
            .get_root_as_reader::<tunnelrpc_capnp::connection_options::Reader<'_>>()
            .expect("get_root_as_reader");
        ConnectionOptions::unmarshal_capnp(reader).expect("unmarshal")
    }

    #[test]
    fn connection_options_no_ip() {
        let id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid");
        let opts = ConnectionOptions::for_current_platform(id, 3);

        assert_eq!(opts, round_trip_options(&opts));
    }

    #[test]
    fn connection_options_ipv4() {
        let id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid");
        let opts = ConnectionOptions {
            origin_local_ip: Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))),
            ..ConnectionOptions::for_current_platform(id, 0)
        };

        assert_eq!(opts, round_trip_options(&opts));
    }

    #[test]
    fn connection_options_ipv6() {
        let id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid");
        let opts = ConnectionOptions {
            origin_local_ip: Some(IpAddr::V6(Ipv6Addr::LOCALHOST)),
            ..ConnectionOptions::for_current_platform(id, 0)
        };

        assert_eq!(opts, round_trip_options(&opts));
    }

    // -- ConnectionDetails ------------------------------------------------

    fn round_trip_details(details: &ConnectionDetails) -> ConnectionDetails {
        let mut msg = ::capnp::message::Builder::new_default();
        details.marshal_capnp(msg.init_root());
        let reader = msg
            .get_root_as_reader::<tunnelrpc_capnp::connection_details::Reader<'_>>()
            .expect("get_root_as_reader");
        ConnectionDetails::unmarshal_capnp(reader).expect("unmarshal")
    }

    #[test]
    fn connection_details_round_trip() {
        let details = ConnectionDetails {
            uuid: Uuid::parse_str("22222222-2222-2222-2222-222222222222").expect("uuid"),
            location: "SFO".into(),
            is_remotely_managed: true,
        };

        assert_eq!(details, round_trip_details(&details));
    }

    // -- ConnectionError --------------------------------------------------

    fn round_trip_error(err: &ConnectionError) -> ConnectionError {
        let mut msg = ::capnp::message::Builder::new_default();
        err.marshal_capnp(msg.init_root());
        let reader = msg
            .get_root_as_reader::<tunnelrpc_capnp::connection_error::Reader<'_>>()
            .expect("get_root_as_reader");
        ConnectionError::unmarshal_capnp(reader).expect("unmarshal")
    }

    #[test]
    fn connection_error_retryable() {
        let err = ConnectionError {
            cause: "overloaded".into(),
            retry_after_ns: 5_000_000_000,
            should_retry: true,
        };

        assert_eq!(err, round_trip_error(&err));
    }

    #[test]
    fn connection_error_fatal() {
        let err = ConnectionError {
            cause: "unauthorized".into(),
            retry_after_ns: 0,
            should_retry: false,
        };

        assert_eq!(err, round_trip_error(&err));
    }

    // -- ConnectionResponse (union) ---------------------------------------

    fn round_trip_response(resp: &ConnectionResponse) -> ConnectionResponse {
        let mut msg = ::capnp::message::Builder::new_default();
        resp.marshal_capnp(msg.init_root());
        let reader = msg
            .get_root_as_reader::<tunnelrpc_capnp::connection_response::Reader<'_>>()
            .expect("get_root_as_reader");
        ConnectionResponse::unmarshal_capnp(reader).expect("unmarshal")
    }

    #[test]
    fn connection_response_success() {
        let resp = ConnectionResponse::Success(ConnectionDetails {
            uuid: Uuid::parse_str("33333333-3333-3333-3333-333333333333").expect("uuid"),
            location: "LAX".into(),
            is_remotely_managed: false,
        });

        assert_eq!(resp, round_trip_response(&resp));
    }

    #[test]
    fn connection_response_error_with_retry() {
        let resp = ConnectionResponse::Error(ConnectionError {
            cause: "too many connections".into(),
            retry_after_ns: 10_000_000_000,
            should_retry: true,
        });

        assert_eq!(resp, round_trip_response(&resp));
    }

    // -- Wire serialization round-trip ------------------------------------

    #[test]
    fn wire_round_trip_connection_response() {
        let resp = ConnectionResponse::Success(ConnectionDetails {
            uuid: Uuid::parse_str("44444444-4444-4444-4444-444444444444").expect("uuid"),
            location: "DFW".into(),
            is_remotely_managed: true,
        });

        let mut msg = ::capnp::message::Builder::new_default();
        resp.marshal_capnp(msg.init_root());

        let mut buf = Vec::new();
        ::capnp::serialize::write_message(&mut buf, &msg).expect("write");

        let msg_reader =
            ::capnp::serialize::read_message(&mut buf.as_slice(), ::capnp::message::ReaderOptions::new())
                .expect("read");

        let decoded = ConnectionResponse::unmarshal_capnp(
            msg_reader
                .get_root::<tunnelrpc_capnp::connection_response::Reader<'_>>()
                .expect("get_root"),
        )
        .expect("unmarshal");

        assert_eq!(resp, decoded);
    }

    // -- parse_ip_from_bytes ----------------------------------------------

    #[test]
    fn parse_ip_v4() {
        let ip = parse_ip_from_bytes(&[10, 0, 0, 1]);

        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    }

    #[test]
    fn parse_ip_v6() {
        let ip = parse_ip_from_bytes(&Ipv6Addr::LOCALHOST.octets());

        assert_eq!(ip, Some(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[test]
    fn parse_ip_empty() {
        assert_eq!(parse_ip_from_bytes(&[]), None);
    }

    #[test]
    fn parse_ip_invalid_length() {
        assert_eq!(parse_ip_from_bytes(&[1, 2, 3]), None);
    }

    // -- RegisterUdpSessionResponse (CDC-009) -----------------------------

    fn round_trip_udp_session_response(resp: &RegisterUdpSessionResponse) -> RegisterUdpSessionResponse {
        let mut msg = ::capnp::message::Builder::new_default();
        resp.marshal_capnp(msg.init_root());
        let reader = msg
            .get_root_as_reader::<tunnelrpc_capnp::register_udp_session_response::Reader<'_>>()
            .expect("get_root_as_reader");
        RegisterUdpSessionResponse::unmarshal_capnp(reader).expect("unmarshal")
    }

    #[test]
    fn udp_session_response_success() {
        let resp = RegisterUdpSessionResponse {
            err: String::new(),
            spans: vec![0x0a, 0x0b, 0x0c],
        };
        assert_eq!(resp, round_trip_udp_session_response(&resp));
        assert!(resp.is_ok());
    }

    #[test]
    fn udp_session_response_error() {
        let resp = RegisterUdpSessionResponse {
            err: "session limit reached".into(),
            spans: Vec::new(),
        };
        assert_eq!(resp, round_trip_udp_session_response(&resp));
        assert!(!resp.is_ok());
    }

    // -- UpdateConfigurationResponse (CDC-010) ----------------------------

    fn round_trip_update_config_response(resp: &UpdateConfigurationResponse) -> UpdateConfigurationResponse {
        let mut msg = ::capnp::message::Builder::new_default();
        resp.marshal_capnp(msg.init_root());
        let reader = msg
            .get_root_as_reader::<tunnelrpc_capnp::update_configuration_response::Reader<'_>>()
            .expect("get_root_as_reader");
        UpdateConfigurationResponse::unmarshal_capnp(reader).expect("unmarshal")
    }

    #[test]
    fn update_config_response_success() {
        let resp = UpdateConfigurationResponse {
            latest_applied_version: 42,
            err: String::new(),
        };
        assert_eq!(resp, round_trip_update_config_response(&resp));
        assert!(resp.is_ok());
    }

    #[test]
    fn update_config_response_with_error() {
        let resp = UpdateConfigurationResponse {
            latest_applied_version: 41,
            err: "invalid ingress rule".into(),
        };
        assert_eq!(resp, round_trip_update_config_response(&resp));
        assert!(!resp.is_ok());
    }

    // -- UpdateLocalConfigurationRequest (CDC-008) ------------------------

    #[test]
    fn update_local_config_from_bytes() {
        let config = b"{\"ingress\":[]}";
        let req = UpdateLocalConfigurationRequest::from_config_bytes(config);
        assert_eq!(req.config, config.to_vec());
    }

    #[test]
    fn update_local_config_round_trip() {
        let config = b"{\"warp-routing\":{\"enabled\":true}}";
        let req = UpdateLocalConfigurationRequest::from_config_bytes(config);
        let encoded = req.to_capnp_bytes();
        let decoded = UpdateLocalConfigurationRequest::from_config_bytes(&encoded);
        assert_eq!(req, decoded);
    }

    // -- RegisterUdpSessionRequest (CDC-009) ------------------------------

    #[test]
    fn udp_session_request_from_rpc_params() {
        let uuid = Uuid::parse_str("55555555-5555-5555-5555-555555555555").expect("uuid");
        let req = RegisterUdpSessionRequest::from_rpc_params(
            uuid.as_bytes(),
            &[10, 0, 0, 1],
            8080,
            5_000_000_000,
            "trace-ctx",
        )
        .expect("from_rpc_params");
        assert_eq!(req.session_id, uuid);
        assert_eq!(req.dst_ip, vec![10, 0, 0, 1]);
        assert_eq!(req.dst_port, 8080);
        assert_eq!(req.close_after_idle_hint_ns, 5_000_000_000);
        assert_eq!(req.trace_context, "trace-ctx");
        assert_eq!(req.session_id_bytes(), uuid.as_bytes().to_vec());
    }

    #[test]
    fn udp_session_request_invalid_session_id() {
        assert!(RegisterUdpSessionRequest::from_rpc_params(&[1, 2, 3], &[], 0, 0, "").is_none());
    }

    // -- UpdateConfigurationRequest (CDC-010) -----------------------------

    #[test]
    fn update_config_request_from_rpc_params() {
        let req = UpdateConfigurationRequest::from_rpc_params(7, b"raw-config");
        assert_eq!(req.version, 7);
        assert_eq!(req.config, b"raw-config".to_vec());
    }
}
