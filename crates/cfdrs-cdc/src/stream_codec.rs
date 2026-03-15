//! Binary wire codec for `ConnectRequest` and `ConnectResponse` (CDC-012).
//!
//! The current interim wire format uses big-endian length-prefixed fields:
//!
//! ```text
//! ConnectRequest:
//!   [2 bytes: connection-type (u16 BE)]
//!   [2 bytes: dest-len (u16 BE)]  [dest-len bytes: dest string]
//!   [2 bytes: metadata-count (u16 BE)]
//!   for each metadata entry:
//!     [2 bytes: key-len (u16 BE)]  [key-len bytes: key string]
//!     [2 bytes: val-len (u16 BE)]  [val-len bytes: val string]
//!
//! ConnectResponse:
//!   [2 bytes: error-len (u16 BE)]  [error-len bytes: error string]
//!   [2 bytes: metadata-count (u16 BE)]
//!   for each metadata entry:
//!     [2 bytes: key-len (u16 BE)]  [key-len bytes: key string]
//!     [2 bytes: val-len (u16 BE)]  [val-len bytes: val string]
//! ```
//!
//! **Note:** The Go baseline uses Cap'n Proto for `ConnectRequest`/
//! `ConnectResponse` encoding. This custom binary format is an interim
//! interop wire format used until Cap'n Proto codec support is admitted.
//!
//! Schema truth:
//! `baseline-2026.2.0/tunnelrpc/proto/quic_metadata_protocol.capnp`

use crate::stream::{ConnectRequest, ConnectResponse, ConnectionType, Metadata};

// ---------------------------------------------------------------------------
// ConnectRequest decode
// ---------------------------------------------------------------------------

/// Parse a `ConnectRequest` from the interim binary wire format.
///
/// Returns `None` if the data is too short, contains invalid UTF-8,
/// or has an unknown connection type.
pub fn decode_connect_request(data: &[u8]) -> Option<ConnectRequest> {
    if data.len() < 6 {
        return None;
    }

    let mut offset = 0;

    // Connection type (2 bytes, big-endian).
    let conn_type_raw = read_u16(data, &mut offset)?;
    let connection_type = ConnectionType::from_u16(conn_type_raw)?;

    // Dest string (2-byte length prefix + string).
    let dest = read_length_prefixed_str(data, &mut offset)?;

    // Metadata entries.
    let metadata = read_metadata_list(data, &mut offset)?;

    Some(ConnectRequest {
        dest: dest.to_owned(),
        connection_type,
        metadata,
    })
}

// ---------------------------------------------------------------------------
// ConnectRequest encode
// ---------------------------------------------------------------------------

/// Encode a `ConnectRequest` into the interim binary wire format.
pub fn encode_connect_request(request: &ConnectRequest) -> Vec<u8> {
    let mut buf = Vec::new();

    // Connection type (2 bytes, big-endian).
    buf.extend_from_slice(&(request.connection_type as u16).to_be_bytes());

    // Dest string.
    write_length_prefixed_str(&mut buf, &request.dest);

    // Metadata.
    write_metadata_list(&mut buf, &request.metadata);

    buf
}

// ---------------------------------------------------------------------------
// ConnectResponse decode
// ---------------------------------------------------------------------------

/// Parse a `ConnectResponse` from the interim binary wire format.
///
/// Returns `None` if the data is too short or contains invalid UTF-8.
pub fn decode_connect_response(data: &[u8]) -> Option<ConnectResponse> {
    if data.len() < 4 {
        return None;
    }

    let mut offset = 0;

    // Error string (2-byte length prefix + string).
    let error = read_length_prefixed_str(data, &mut offset)?;

    // Metadata entries.
    let metadata = read_metadata_list(data, &mut offset)?;

    Some(ConnectResponse {
        error: error.to_owned(),
        metadata,
    })
}

// ---------------------------------------------------------------------------
// ConnectResponse encode
// ---------------------------------------------------------------------------

/// Encode a `ConnectResponse` into the interim binary wire format.
pub fn encode_connect_response(response: &ConnectResponse) -> Vec<u8> {
    let mut buf = Vec::new();

    // Error string.
    write_length_prefixed_str(&mut buf, &response.error);

    // Metadata.
    write_metadata_list(&mut buf, &response.metadata);

    buf
}

// ---------------------------------------------------------------------------
// Wire primitives
// ---------------------------------------------------------------------------

fn read_u16(data: &[u8], offset: &mut usize) -> Option<u16> {
    if *offset + 2 > data.len() {
        return None;
    }

    let value = u16::from_be_bytes([data[*offset], data[*offset + 1]]);
    *offset += 2;
    Some(value)
}

fn read_length_prefixed_str<'a>(data: &'a [u8], offset: &mut usize) -> Option<&'a str> {
    let len = read_u16(data, offset)? as usize;

    if *offset + len > data.len() {
        return None;
    }

    let s = std::str::from_utf8(&data[*offset..*offset + len]).ok()?;
    *offset += len;
    Some(s)
}

fn read_metadata_list(data: &[u8], offset: &mut usize) -> Option<Vec<Metadata>> {
    let count = read_u16(data, offset)? as usize;
    let mut metadata = Vec::with_capacity(count);

    for _ in 0..count {
        let key = read_length_prefixed_str(data, offset)?;
        let val = read_length_prefixed_str(data, offset)?;

        metadata.push(Metadata::new(key, val));
    }

    Some(metadata)
}

fn write_length_prefixed_str(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    buf.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    buf.extend_from_slice(bytes);
}

fn write_metadata_list(buf: &mut Vec<u8>, metadata: &[Metadata]) {
    buf.extend_from_slice(&(metadata.len() as u16).to_be_bytes());

    for entry in metadata {
        write_length_prefixed_str(buf, &entry.key);
        write_length_prefixed_str(buf, &entry.val);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream_contract::{HTTP_HOST_KEY, HTTP_METHOD_KEY, HTTP_STATUS_KEY, header_metadata_key};

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
    fn decode_rejects_unknown_connection_type() {
        // Connection type 99 is not valid.
        let data = [0x00, 99, 0x00, 0x00, 0x00, 0x00];
        assert!(decode_connect_request(&data).is_none());
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
}
