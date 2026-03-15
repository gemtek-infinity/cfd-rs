//! Cap'n Proto wire codec for `ConnectRequest` and `ConnectResponse` (CDC-011,
//! CDC-012).
//!
//! Bridges between the domain types in [`stream`](crate::stream) and the
//! generated Cap'n Proto bindings from `quic_metadata_protocol.capnp`.
//!
//! The Go baseline encodes these with `pogs.Insert`/`pogs.Extract` through
//! the `ToPogs`/`FromPogs` methods in
//! `baseline-2026.2.0/tunnelrpc/pogs/quic_metadata_protocol.go`.
//!
//! Schema truth:
//! `baseline-2026.2.0/tunnelrpc/proto/quic_metadata_protocol.capnp`

use crate::quic_metadata_protocol_capnp;
use crate::registration_codec::read_capnp_text;
use crate::stream::{ConnectRequest, ConnectResponse, ConnectionType, Metadata};

// ---------------------------------------------------------------------------
// ConnectionType mapping
// ---------------------------------------------------------------------------

fn to_capnp_connection_type(ct: ConnectionType) -> quic_metadata_protocol_capnp::ConnectionType {
    match ct {
        ConnectionType::Http => quic_metadata_protocol_capnp::ConnectionType::Http,
        ConnectionType::WebSocket => quic_metadata_protocol_capnp::ConnectionType::Websocket,
        ConnectionType::Tcp => quic_metadata_protocol_capnp::ConnectionType::Tcp,
    }
}

fn from_capnp_connection_type(ct: quic_metadata_protocol_capnp::ConnectionType) -> ConnectionType {
    match ct {
        quic_metadata_protocol_capnp::ConnectionType::Http => ConnectionType::Http,
        quic_metadata_protocol_capnp::ConnectionType::Websocket => ConnectionType::WebSocket,
        quic_metadata_protocol_capnp::ConnectionType::Tcp => ConnectionType::Tcp,
    }
}

// ---------------------------------------------------------------------------
// Metadata — @0xe1446b97bfd1cd37
// ---------------------------------------------------------------------------

impl Metadata {
    /// Write a metadata entry into a Cap'n Proto `Metadata` builder.
    pub fn marshal_capnp(&self, mut builder: quic_metadata_protocol_capnp::metadata::Builder<'_>) {
        builder.set_key(&self.key);
        builder.set_val(&self.val);
    }

    /// Read a metadata entry from a Cap'n Proto `Metadata` reader.
    pub fn unmarshal_capnp(
        reader: quic_metadata_protocol_capnp::metadata::Reader<'_>,
    ) -> ::capnp::Result<Self> {
        Ok(Self {
            key: read_capnp_text(reader.get_key()?)?,
            val: read_capnp_text(reader.get_val()?)?,
        })
    }
}

// ---------------------------------------------------------------------------
// ConnectRequest — @0xc47116a1045e4061
// ---------------------------------------------------------------------------

impl ConnectRequest {
    /// Write a connect request into a Cap'n Proto `ConnectRequest` builder.
    pub fn marshal_capnp(&self, mut builder: quic_metadata_protocol_capnp::connect_request::Builder<'_>) {
        builder.set_dest(&self.dest);
        builder.set_type(to_capnp_connection_type(self.connection_type));

        let mut metadata_list = builder.init_metadata(self.metadata.len() as u32);

        for (i, entry) in self.metadata.iter().enumerate() {
            entry.marshal_capnp(metadata_list.reborrow().get(i as u32));
        }
    }

    /// Read a connect request from a Cap'n Proto `ConnectRequest` reader.
    pub fn unmarshal_capnp(
        reader: quic_metadata_protocol_capnp::connect_request::Reader<'_>,
    ) -> ::capnp::Result<Self> {
        let dest = read_capnp_text(reader.get_dest()?)?;

        let connection_type = reader
            .get_type()
            .map(from_capnp_connection_type)
            .map_err(|e| ::capnp::Error::failed(format!("unknown connection type: {e:?}")))?;

        let metadata_reader = reader.get_metadata()?;
        let mut metadata = Vec::with_capacity(metadata_reader.len() as usize);

        for i in 0..metadata_reader.len() {
            metadata.push(Metadata::unmarshal_capnp(metadata_reader.get(i))?);
        }

        Ok(Self {
            dest,
            connection_type,
            metadata,
        })
    }
}

// ---------------------------------------------------------------------------
// ConnectResponse — @0xb1032ec91cef8727
// ---------------------------------------------------------------------------

impl ConnectResponse {
    /// Write a connect response into a Cap'n Proto `ConnectResponse` builder.
    pub fn marshal_capnp(&self, mut builder: quic_metadata_protocol_capnp::connect_response::Builder<'_>) {
        builder.set_error(&self.error);

        let mut metadata_list = builder.init_metadata(self.metadata.len() as u32);

        for (i, entry) in self.metadata.iter().enumerate() {
            entry.marshal_capnp(metadata_list.reborrow().get(i as u32));
        }
    }

    /// Read a connect response from a Cap'n Proto `ConnectResponse` reader.
    pub fn unmarshal_capnp(
        reader: quic_metadata_protocol_capnp::connect_response::Reader<'_>,
    ) -> ::capnp::Result<Self> {
        let error = read_capnp_text(reader.get_error()?)?;

        let metadata_reader = reader.get_metadata()?;
        let mut metadata = Vec::with_capacity(metadata_reader.len() as usize);

        for i in 0..metadata_reader.len() {
            metadata.push(Metadata::unmarshal_capnp(metadata_reader.get(i))?);
        }

        Ok(Self { error, metadata })
    }
}

// ---------------------------------------------------------------------------
// Wire-level encode / decode
// ---------------------------------------------------------------------------

/// Encode a `ConnectRequest` into Cap'n Proto wire format.
pub fn encode_connect_request(request: &ConnectRequest) -> Vec<u8> {
    let mut message = ::capnp::message::Builder::new_default();
    let builder = message.init_root::<quic_metadata_protocol_capnp::connect_request::Builder<'_>>();
    request.marshal_capnp(builder);

    let mut buf = Vec::new();
    ::capnp::serialize::write_message(&mut buf, &message).expect("capnp write to Vec should not fail");
    buf
}

/// Parse a `ConnectRequest` from Cap'n Proto wire format.
///
/// Returns `None` if the data is malformed or contains an unknown
/// connection type.
pub fn decode_connect_request(data: &[u8]) -> Option<ConnectRequest> {
    let reader =
        ::capnp::serialize::read_message_from_flat_slice(&mut &*data, ::capnp::message::ReaderOptions::new())
            .ok()?;

    let root = reader
        .get_root::<quic_metadata_protocol_capnp::connect_request::Reader<'_>>()
        .ok()?;

    ConnectRequest::unmarshal_capnp(root).ok()
}

/// Encode a `ConnectResponse` into Cap'n Proto wire format.
pub fn encode_connect_response(response: &ConnectResponse) -> Vec<u8> {
    let mut message = ::capnp::message::Builder::new_default();
    let builder = message.init_root::<quic_metadata_protocol_capnp::connect_response::Builder<'_>>();
    response.marshal_capnp(builder);

    let mut buf = Vec::new();
    ::capnp::serialize::write_message(&mut buf, &message).expect("capnp write to Vec should not fail");
    buf
}

/// Parse a `ConnectResponse` from Cap'n Proto wire format.
///
/// Returns `None` if the data is malformed.
pub fn decode_connect_response(data: &[u8]) -> Option<ConnectResponse> {
    let reader =
        ::capnp::serialize::read_message_from_flat_slice(&mut &*data, ::capnp::message::ReaderOptions::new())
            .ok()?;

    let root = reader
        .get_root::<quic_metadata_protocol_capnp::connect_response::Reader<'_>>()
        .ok()?;

    ConnectResponse::unmarshal_capnp(root).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream_contract::{HTTP_HOST_KEY, HTTP_METHOD_KEY, HTTP_STATUS_KEY, header_metadata_key};

    // -----------------------------------------------------------------------
    // Wire-level roundtrips (encode → decode)
    // -----------------------------------------------------------------------

    #[test]
    fn connect_request_roundtrip() {
        let original = ConnectRequest {
            dest: "http://localhost:8080/api".into(),
            connection_type: ConnectionType::Http,
            metadata: vec![
                Metadata::new(HTTP_METHOD_KEY, "POST"),
                Metadata::new(HTTP_HOST_KEY, "example.com"),
                Metadata::new(header_metadata_key("Content-Type"), "application/json"),
            ],
        };

        let wire = encode_connect_request(&original);
        let parsed = decode_connect_request(&wire).expect("roundtrip should succeed");

        assert_eq!(parsed, original);
    }

    #[test]
    fn connect_response_roundtrip() {
        let original = ConnectResponse::http(200, vec![("Content-Type".into(), "text/html".into())]);

        let wire = encode_connect_response(&original);
        let parsed = decode_connect_response(&wire).expect("roundtrip should succeed");

        assert_eq!(parsed, original);
    }

    #[test]
    fn connect_response_error_roundtrip() {
        let original = ConnectResponse::error("origin unreachable");

        let wire = encode_connect_response(&original);
        let parsed = decode_connect_response(&wire).expect("roundtrip should succeed");

        assert_eq!(parsed, original);
    }

    #[test]
    fn connect_request_all_connection_types() {
        for (conn_type, label) in [
            (ConnectionType::Http, "HTTP"),
            (ConnectionType::WebSocket, "WebSocket"),
            (ConnectionType::Tcp, "TCP"),
        ] {
            let original = ConnectRequest {
                dest: format!("test-{label}"),
                connection_type: conn_type,
                metadata: vec![],
            };

            let wire = encode_connect_request(&original);
            let parsed = decode_connect_request(&wire).expect("should parse");

            assert_eq!(parsed.connection_type, conn_type);
            assert_eq!(parsed.dest, format!("test-{label}"));
        }
    }

    #[test]
    fn decode_rejects_truncated_data() {
        assert!(decode_connect_request(&[]).is_none());
        assert!(decode_connect_request(&[0, 0]).is_none());
        assert!(decode_connect_response(&[]).is_none());
        assert!(decode_connect_response(&[0]).is_none());
    }

    #[test]
    fn connect_response_with_status_and_headers() {
        let response = ConnectResponse {
            error: String::new(),
            metadata: vec![
                Metadata::new(HTTP_STATUS_KEY, "404"),
                Metadata::new(header_metadata_key("Content-Type"), "text/plain"),
            ],
        };

        let wire = encode_connect_response(&response);
        let parsed = decode_connect_response(&wire).expect("should parse");

        assert!(parsed.is_ok());
        assert_eq!(parsed.metadata.len(), 2);
        assert_eq!(parsed.metadata[0].val, "404");
    }

    #[test]
    fn empty_metadata_roundtrip() {
        let request = ConnectRequest {
            dest: "/".into(),
            connection_type: ConnectionType::Tcp,
            metadata: vec![],
        };

        let wire = encode_connect_request(&request);
        let parsed = decode_connect_request(&wire).expect("should parse");

        assert_eq!(parsed.metadata.len(), 0);
        assert_eq!(parsed.dest, "/");
    }

    // -----------------------------------------------------------------------
    // Marshal / unmarshal (builder → reader without wire serialization)
    // -----------------------------------------------------------------------

    #[test]
    fn metadata_marshal_unmarshal() {
        let original = Metadata::new("TestKey", "TestValue");

        let mut message = ::capnp::message::Builder::new_default();
        let builder = message.init_root::<quic_metadata_protocol_capnp::metadata::Builder<'_>>();
        original.marshal_capnp(builder);

        let reader = message
            .get_root_as_reader::<quic_metadata_protocol_capnp::metadata::Reader<'_>>()
            .expect("should build reader");
        let parsed = Metadata::unmarshal_capnp(reader).expect("should unmarshal");

        assert_eq!(parsed, original);
    }

    #[test]
    fn connect_request_marshal_unmarshal() {
        let original = ConnectRequest {
            dest: "http://app.example.com:8080".into(),
            connection_type: ConnectionType::WebSocket,
            metadata: vec![
                Metadata::new(HTTP_METHOD_KEY, "GET"),
                Metadata::new(HTTP_HOST_KEY, "app.example.com"),
            ],
        };

        let mut message = ::capnp::message::Builder::new_default();
        let builder = message.init_root::<quic_metadata_protocol_capnp::connect_request::Builder<'_>>();
        original.marshal_capnp(builder);

        let reader = message
            .get_root_as_reader::<quic_metadata_protocol_capnp::connect_request::Reader<'_>>()
            .expect("should build reader");
        let parsed = ConnectRequest::unmarshal_capnp(reader).expect("should unmarshal");

        assert_eq!(parsed, original);
    }

    #[test]
    fn connect_response_marshal_unmarshal() {
        let original = ConnectResponse::error("upstream timeout");

        let mut message = ::capnp::message::Builder::new_default();
        let builder = message.init_root::<quic_metadata_protocol_capnp::connect_response::Builder<'_>>();
        original.marshal_capnp(builder);

        let reader = message
            .get_root_as_reader::<quic_metadata_protocol_capnp::connect_response::Reader<'_>>()
            .expect("should build reader");
        let parsed = ConnectResponse::unmarshal_capnp(reader).expect("should unmarshal");

        assert_eq!(parsed, original);
    }
}
