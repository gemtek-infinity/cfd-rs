//! Per-stream request/response types for the cloudflare tunnel protocol.
//!
//! Each QUIC data stream carries a `ConnectRequest` from the edge to the
//! tunnel client, followed by a `ConnectResponse` from the client back to
//! the edge. After the response, the stream becomes a bidirectional pipe
//! between the eyeball and the origin service.
//!
//! These types match the behavioral contract from
//! `baseline-2026.2.0/old-impl/tunnelrpc/pogs/quic_metadata_protocol.go`.

use serde::{Deserialize, Serialize};

/// Metadata key for the HTTP method in a ConnectRequest.
pub const HTTP_METHOD_KEY: &str = "HttpMethod";

/// Metadata key for the HTTP host in a ConnectRequest.
pub const HTTP_HOST_KEY: &str = "HttpHost";

/// Metadata key prefix for HTTP headers in a ConnectRequest.
///
/// Individual headers are encoded as `HttpHeader:Header-Name`.
pub const HTTP_HEADER_KEY: &str = "HttpHeader";

/// Metadata key for the HTTP status in a ConnectResponse.
pub const HTTP_STATUS_KEY: &str = "HttpStatus";

/// Metadata key for QUIC flow tracking.
pub const FLOW_ID_KEY: &str = "FlowID";

/// Metadata key for Cloudflare trace ID propagation.
pub const CF_TRACE_ID_KEY: &str = "cf-trace-id";

/// Metadata key for content length.
pub const CONTENT_LENGTH_KEY: &str = "HttpHeader:Content-Length";

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
            Self::Http => "HTTP",
            Self::WebSocket => "WebSocket",
            Self::Tcp => "TCP",
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
            .find(|m| m.key == key)
            .map(|m| m.val.as_str())
    }

    /// Extract the HTTP method from metadata, defaulting to GET.
    pub fn http_method(&self) -> &str {
        self.metadata_value(HTTP_METHOD_KEY).unwrap_or("GET")
    }

    /// Extract the HTTP host from metadata.
    pub fn http_host(&self) -> Option<&str> {
        self.metadata_value(HTTP_HOST_KEY)
    }

    /// Extract HTTP headers from metadata.
    ///
    /// Headers are encoded as `HttpHeader:Header-Name` keys.
    pub fn http_headers(&self) -> impl Iterator<Item = (&str, &str)> {
        let prefix = format!("{HTTP_HEADER_KEY}:");
        self.metadata.iter().filter_map(move |m| {
            m.key
                .strip_prefix(&prefix)
                .map(|header_name| (header_name, m.val.as_str()))
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
            metadata.push(Metadata::new(format!("{HTTP_HEADER_KEY}:{name}"), value));
        }

        Self::success(metadata)
    }

    /// Create a TCP ack response (no error, optional trace propagation).
    pub fn tcp_ack(trace_propagation: Option<&str>) -> Self {
        let metadata = trace_propagation
            .map(|tp| vec![Metadata::new("cf-trace-context", tp)])
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

    #[test]
    fn connection_type_roundtrip() {
        assert_eq!(ConnectionType::from_u16(0), Some(ConnectionType::Http));
        assert_eq!(ConnectionType::from_u16(1), Some(ConnectionType::WebSocket));
        assert_eq!(ConnectionType::from_u16(2), Some(ConnectionType::Tcp));
        assert_eq!(ConnectionType::from_u16(3), None);
    }

    #[test]
    fn connect_request_metadata_extraction() {
        let request = ConnectRequest {
            dest: "http://localhost:8080/api".into(),
            connection_type: ConnectionType::Http,
            metadata: vec![
                Metadata::new(HTTP_METHOD_KEY, "POST"),
                Metadata::new(HTTP_HOST_KEY, "example.com"),
                Metadata::new(format!("{HTTP_HEADER_KEY}:Content-Type"), "application/json"),
                Metadata::new(format!("{HTTP_HEADER_KEY}:Authorization"), "Bearer tok"),
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
        let resp = ConnectResponse::http(200, vec![("Content-Type".into(), "text/html".into())]);
        assert!(resp.is_ok());
        assert_eq!(resp.metadata.len(), 2);
        assert_eq!(resp.metadata[0].key, HTTP_STATUS_KEY);
        assert_eq!(resp.metadata[0].val, "200");
    }

    #[test]
    fn connect_response_error() {
        let resp = ConnectResponse::error("origin unreachable");
        assert!(!resp.is_ok());
        assert_eq!(resp.error, "origin unreachable");
    }

    #[test]
    fn connect_response_tcp_ack() {
        let resp = ConnectResponse::tcp_ack(Some("trace-abc"));
        assert!(resp.is_ok());
        assert_eq!(resp.metadata.len(), 1);

        let resp_no_trace = ConnectResponse::tcp_ack(None);
        assert!(resp_no_trace.metadata.is_empty());
    }

    #[test]
    fn default_http_method_is_get() {
        let request = ConnectRequest {
            dest: "/".into(),
            connection_type: ConnectionType::Http,
            metadata: vec![],
        };
        assert_eq!(request.http_method(), "GET");
    }
}
