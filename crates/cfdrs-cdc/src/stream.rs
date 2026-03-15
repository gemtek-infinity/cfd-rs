//! Per-stream request/response types for the cloudflare tunnel protocol.
//!
//! Each QUIC data stream carries a `ConnectRequest` from the edge to the
//! tunnel client, followed by a `ConnectResponse` from the client back to
//! the edge. After the response, the stream becomes a bidirectional pipe
//! between the eyeball and the origin service.
//!
//! These types match the behavioral contract from
//! `baseline-2026.2.0/tunnelrpc/pogs/quic_metadata_protocol.go`.

use serde::{Deserialize, Serialize};

use crate::stream_contract::{
    CF_TRACE_ID_KEY, DEFAULT_HTTP_METHOD, FLOW_ID_KEY, HTTP_HOST_KEY, HTTP_LABEL, HTTP_METHOD_KEY,
    HTTP_STATUS_KEY, TCP_LABEL, TRACE_CONTEXT_KEY, WEBSOCKET_LABEL, header_metadata_key,
    header_metadata_prefix,
};

/// Connection type for a QUIC data stream.
///
/// Matches the Go baseline's `ConnectionType` from
/// `tunnelrpc/pogs/quic_metadata_protocol.go`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum ConnectionType {
    /// Standard HTTP request.
    Http = 0,
    /// WebSocket upgrade request.
    WebSocket = 1,
    /// Raw TCP stream (WARP routing, SSH bastion, etc.).
    Tcp = 2,
}

impl ConnectionType {
    /// Parse from the wire representation.
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0 => Some(Self::Http),
            1 => Some(Self::WebSocket),
            2 => Some(Self::Tcp),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Http => HTTP_LABEL,
            Self::WebSocket => WEBSOCKET_LABEL,
            Self::Tcp => TCP_LABEL,
        }
    }
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Key-value metadata pair carried in connect request/response messages.
///
/// Used for HTTP headers, flow tracking, and trace propagation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    pub key: String,
    pub val: String,
}

impl Metadata {
    pub fn new(key: impl Into<String>, val: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            val: val.into(),
        }
    }
}

/// Per-stream request from the edge to the tunnel client.
///
/// Carried at the beginning of each QUIC data stream. Tells the tunnel
/// client what type of connection this is and where to route it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectRequest {
    /// Destination address or URL for the request.
    pub dest: String,
    /// The type of connection (HTTP, WebSocket, or TCP).
    pub connection_type: ConnectionType,
    /// Key-value metadata pairs (HTTP headers, flow IDs, trace context).
    pub metadata: Vec<Metadata>,
}

impl ConnectRequest {
    /// Look up a metadata value by key.
    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|metadata| metadata.key == key)
            .map(|metadata| metadata.val.as_str())
    }

    /// Extract the HTTP method from metadata, defaulting to GET.
    pub fn http_method(&self) -> &str {
        self.metadata_value(HTTP_METHOD_KEY)
            .unwrap_or(DEFAULT_HTTP_METHOD)
    }

    /// Extract the HTTP host from metadata.
    pub fn http_host(&self) -> Option<&str> {
        self.metadata_value(HTTP_HOST_KEY)
    }

    /// Extract HTTP headers from metadata.
    ///
    /// Headers are encoded as `HttpHeader:Header-Name` keys.
    pub fn http_headers(&self) -> impl Iterator<Item = (&str, &str)> {
        let prefix = header_metadata_prefix();
        self.metadata.iter().filter_map(move |metadata| {
            metadata
                .key
                .strip_prefix(&prefix)
                .map(|header_name| (header_name, metadata.val.as_str()))
        })
    }

    /// Extract the flow ID from metadata, if present.
    pub fn flow_id(&self) -> Option<&str> {
        self.metadata_value(FLOW_ID_KEY)
    }

    /// Extract the trace ID from metadata, if present.
    pub fn trace_id(&self) -> Option<&str> {
        self.metadata_value(CF_TRACE_ID_KEY)
    }
}

/// Per-stream response from the tunnel client back to the edge.
///
/// Sent after the tunnel client processes the `ConnectRequest`. On success
/// the error field is empty and the stream becomes a bidirectional pipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectResponse {
    /// Non-empty if the connection failed.
    pub error: String,
    /// Response metadata (HTTP status, headers, trace propagation).
    pub metadata: Vec<Metadata>,
}

impl ConnectResponse {
    /// Create a successful response with the given metadata.
    pub fn success(metadata: Vec<Metadata>) -> Self {
        Self {
            error: String::new(),
            metadata,
        }
    }

    /// Create an error response.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            metadata: Vec::new(),
        }
    }

    /// Create a successful HTTP response with status code and headers.
    pub fn http(status: u16, headers: Vec<(String, String)>) -> Self {
        let mut metadata = Vec::with_capacity(1 + headers.len());
        metadata.push(Metadata::new(HTTP_STATUS_KEY, status.to_string()));

        for (name, value) in headers {
            metadata.push(Metadata::new(header_metadata_key(&name), value));
        }

        Self::success(metadata)
    }

    /// Create a TCP ack response (no error, optional trace propagation).
    pub fn tcp_ack(trace_propagation: Option<&str>) -> Self {
        let metadata = trace_propagation
            .map(|trace_context| vec![Metadata::new(TRACE_CONTEXT_KEY, trace_context)])
            .unwrap_or_default();
        Self::success(metadata)
    }

    /// Whether the response indicates success (no error).
    pub fn is_ok(&self) -> bool {
        self.error.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream_contract::{
        CONTENT_LENGTH_KEY, HTTP_LABEL, TCP_LABEL, TRACE_CONTEXT_KEY, WEBSOCKET_LABEL, header_metadata_key,
    };

    #[test]
    fn connection_type_roundtrip() {
        assert_eq!(ConnectionType::from_u16(0), Some(ConnectionType::Http));
        assert_eq!(ConnectionType::from_u16(1), Some(ConnectionType::WebSocket));
        assert_eq!(ConnectionType::from_u16(2), Some(ConnectionType::Tcp));
        assert_eq!(ConnectionType::from_u16(3), None);
        assert_eq!(ConnectionType::Http.as_str(), HTTP_LABEL);
        assert_eq!(ConnectionType::WebSocket.as_str(), WEBSOCKET_LABEL);
        assert_eq!(ConnectionType::Tcp.as_str(), TCP_LABEL);
    }

    #[test]
    fn connect_request_metadata_extraction() {
        let request = ConnectRequest {
            dest: "http://localhost:8080/api".into(),
            connection_type: ConnectionType::Http,
            metadata: vec![
                Metadata::new(HTTP_METHOD_KEY, "POST"),
                Metadata::new(HTTP_HOST_KEY, "example.com"),
                Metadata::new(header_metadata_key("Content-Type"), "application/json"),
                Metadata::new(header_metadata_key("Authorization"), "Bearer tok"),
                Metadata::new(FLOW_ID_KEY, "flow-123"),
            ],
        };

        assert_eq!(request.http_method(), "POST");
        assert_eq!(request.http_host(), Some("example.com"));
        assert_eq!(request.flow_id(), Some("flow-123"));

        let headers: Vec<_> = request.http_headers().collect();
        assert_eq!(headers.len(), 2);
        assert!(headers.contains(&("Content-Type", "application/json")));
        assert!(headers.contains(&("Authorization", "Bearer tok")));
    }

    #[test]
    fn connect_response_http() {
        let response = ConnectResponse::http(200, vec![("Content-Type".into(), "text/html".into())]);
        assert!(response.is_ok());
        assert_eq!(response.metadata.len(), 2);
        assert_eq!(response.metadata[0].key, HTTP_STATUS_KEY);
        assert_eq!(response.metadata[0].val, "200");
    }

    #[test]
    fn connect_response_error() {
        let response = ConnectResponse::error("origin unreachable");
        assert!(!response.is_ok());
        assert_eq!(response.error, "origin unreachable");
    }

    #[test]
    fn connect_response_tcp_ack() {
        let response = ConnectResponse::tcp_ack(Some("trace-abc"));
        assert!(response.is_ok());
        assert_eq!(response.metadata.len(), 1);
        assert_eq!(response.metadata[0].key, TRACE_CONTEXT_KEY);

        let no_trace_response = ConnectResponse::tcp_ack(None);
        assert!(no_trace_response.metadata.is_empty());
    }

    #[test]
    fn default_http_method_is_get() {
        let request = ConnectRequest {
            dest: "/".into(),
            connection_type: ConnectionType::Http,
            metadata: vec![],
        };
        assert_eq!(request.http_method(), DEFAULT_HTTP_METHOD);
    }

    #[test]
    fn content_length_key_remains_exact() {
        assert_eq!(CONTENT_LENGTH_KEY, header_metadata_key("Content-Length"));
    }

    // --- CDC-018: strengthened stream round-trip evidence ---

    #[test]
    fn connect_request_serde_roundtrip() {
        let request = ConnectRequest {
            dest: "http://10.0.0.1:8080/api".into(),
            connection_type: ConnectionType::Http,
            metadata: vec![
                Metadata::new(HTTP_METHOD_KEY, "POST"),
                Metadata::new(HTTP_HOST_KEY, "example.com"),
                Metadata::new(header_metadata_key("Content-Type"), "application/json"),
                Metadata::new(FLOW_ID_KEY, "flow-42"),
                Metadata::new(CF_TRACE_ID_KEY, "abc-123"),
            ],
        };
        let json = serde_json::to_string(&request).expect("serialize");
        let deserialized: ConnectRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(request, deserialized);
    }

    #[test]
    fn connect_response_serde_roundtrip() {
        let response = ConnectResponse::http(403, vec![("X-Custom".into(), "blocked".into())]);
        let json = serde_json::to_string(&response).expect("serialize");
        let deserialized: ConnectResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(response, deserialized);
    }

    #[test]
    fn connection_type_from_u16_exhaustive_boundary() {
        // Only values 0, 1, 2 are valid — matching Go's ConnectionType enum
        for v in 0..=2u16 {
            assert!(ConnectionType::from_u16(v).is_some(), "value {v} should parse");
        }
        for v in 3..=10u16 {
            assert!(
                ConnectionType::from_u16(v).is_none(),
                "value {v} should not parse"
            );
        }
        assert!(ConnectionType::from_u16(u16::MAX).is_none());
    }

    #[test]
    fn websocket_request_metadata_extraction() {
        let request = ConnectRequest {
            dest: "ws://localhost:9000".into(),
            connection_type: ConnectionType::WebSocket,
            metadata: vec![
                Metadata::new(HTTP_METHOD_KEY, DEFAULT_HTTP_METHOD),
                Metadata::new(HTTP_HOST_KEY, "ws.example.com"),
            ],
        };
        assert_eq!(request.http_method(), DEFAULT_HTTP_METHOD);
        assert_eq!(request.http_host(), Some("ws.example.com"));
        assert_eq!(request.flow_id(), None);
        assert_eq!(request.trace_id(), None);
        assert_eq!(request.http_headers().count(), 0);
    }

    #[test]
    fn tcp_request_missing_metadata_returns_none() {
        let request = ConnectRequest {
            dest: "10.0.0.1:22".into(),
            connection_type: ConnectionType::Tcp,
            metadata: vec![],
        };
        assert_eq!(request.http_host(), None);
        assert_eq!(request.flow_id(), None);
        assert_eq!(request.trace_id(), None);
        assert_eq!(request.http_method(), DEFAULT_HTTP_METHOD);
    }

    #[test]
    fn connect_response_http_status_codes() {
        for status in [200, 301, 403, 404, 500, 502, 503] {
            let response = ConnectResponse::http(status, vec![]);
            assert!(response.is_ok());
            assert_eq!(response.metadata[0].val, status.to_string());
        }
    }
}
