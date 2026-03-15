// ---------------------------------------------------------------------------
// Per-stream metadata keys (CDC-014)
// ---------------------------------------------------------------------------

use base64::Engine as _;

pub const HTTP_METHOD_KEY: &str = "HttpMethod";
pub const HTTP_HOST_KEY: &str = "HttpHost";
pub const HTTP_HEADER_KEY: &str = "HttpHeader";
pub const HTTP_STATUS_KEY: &str = "HttpStatus";
pub const FLOW_ID_KEY: &str = "FlowID";
pub const CF_TRACE_ID_KEY: &str = "cf-trace-id";
#[cfg(test)]
pub const CONTENT_LENGTH_KEY: &str = "HttpHeader:Content-Length";
pub const TRACE_CONTEXT_KEY: &str = "cf-trace-context";
pub const DEFAULT_HTTP_METHOD: &str = "GET";
pub const HTTP_LABEL: &str = "HTTP";
pub const WEBSOCKET_LABEL: &str = "WebSocket";
pub const TCP_LABEL: &str = "TCP";
const HEADER_SEPARATOR: &str = ":";

pub fn header_metadata_key(name: &str) -> String {
    format!("{HTTP_HEADER_KEY}{HEADER_SEPARATOR}{name}")
}

pub fn header_metadata_prefix() -> String {
    header_metadata_key("")
}

// ---------------------------------------------------------------------------
// Internal transport headers (CDC-015)
//
// These constants define the baseline wire-header keys for serialized user
// headers and response metadata.  They will be consumed when stream-level
// header handling is wired up.
// ---------------------------------------------------------------------------

/// Header carrying base64-serialized request headers from cloudflared to edge.
pub const REQUEST_USER_HEADERS: &str = "cf-cloudflared-request-headers";

/// Header carrying base64-serialized response headers from edge to cloudflared.
pub const RESPONSE_USER_HEADERS: &str = "cf-cloudflared-response-headers";

/// Header carrying response source metadata (JSON).
pub const RESPONSE_META_HEADER: &str = "cf-cloudflared-response-meta";

// ---------------------------------------------------------------------------
// Response meta header values (CDC-016)
// ---------------------------------------------------------------------------

/// Response originated from the origin server.
pub const RESPONSE_META_ORIGIN: &str = r#"{"src":"origin"}"#;

/// Response originated from cloudflared itself.
pub const RESPONSE_META_CLOUDFLARED: &str = r#"{"src":"cloudflared"}"#;

/// Response originated from cloudflared with flow rate limiting active.
pub const RESPONSE_META_CLOUDFLARED_FLOW_LIMITED: &str = r#"{"src":"cloudflared","flow_rate_limited":true}"#;

// ---------------------------------------------------------------------------
// Control response header stripping (CDC-017)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
const CONTROL_HEADER_PREFIXES: &[&str] = &[":", "cf-int-", "cf-cloudflared-", "cf-proxy-"];

/// Test whether a response header is an internal control header that should
/// be stripped before forwarding to the eyeball.
///
/// Matches Go's `IsControlResponseHeader` from `connection/header.go`.
pub fn is_control_response_header(header_name: &str) -> bool {
    let lower = header_name.to_ascii_lowercase();

    CONTROL_HEADER_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

/// WebSocket client header names that are handled by the upgrade handshake
/// and should not be forwarded as user headers.
///
/// Matches Go's `IsWebsocketClientHeader` from `connection/header.go`.
pub fn is_websocket_client_header(header_name: &str) -> bool {
    let lower = header_name.to_ascii_lowercase();
    lower == "sec-websocket-accept" || lower == "connection" || lower == "upgrade"
}

// ---------------------------------------------------------------------------
// Transport header serialization (CDC-015)
// ---------------------------------------------------------------------------

/// The base64 engine matching Go's `base64.RawStdEncoding`:
/// standard alphabet (A-Za-z0-9+/), no padding.
fn header_b64_engine() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD_NO_PAD
}

const PAIR_SEPARATOR: char = ';';
const NAME_VALUE_SEPARATOR: char = ':';

/// A single deserialized HTTP header (name + value pair).
///
/// Matches Go's `HTTPHeader` in `connection/header.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

/// Serialize HTTP headers into the baseline wire format.
///
/// Format: `base64(name):base64(value)` pairs joined by `;`.
/// Uses base64 standard alphabet with no padding, matching
/// Go's `base64.RawStdEncoding`.
///
/// Matches Go's `SerializeHeaders` in `connection/header.go`.
pub fn serialize_headers(headers: &[HttpHeader]) -> String {
    let engine = header_b64_engine();
    let mut buf = String::new();

    for header in headers {
        if !buf.is_empty() {
            buf.push(PAIR_SEPARATOR);
        }
        buf.push_str(&engine.encode(&header.name));
        buf.push(NAME_VALUE_SEPARATOR);
        buf.push_str(&engine.encode(&header.value));
    }

    buf
}

/// Deserialize HTTP headers from the baseline wire format.
///
/// Inverse of [`serialize_headers`]. Returns `None` if the format is
/// malformed (wrong number of `:` separators) or base64 decoding fails.
///
/// Matches Go's `DeserializeHeaders` in `connection/header.go`.
pub fn deserialize_headers(serialized: &str) -> Option<Vec<HttpHeader>> {
    let engine = header_b64_engine();
    let mut headers = Vec::new();

    for pair in serialized.split(PAIR_SEPARATOR) {
        if pair.is_empty() {
            continue;
        }

        // Go requires exactly 2 parts when splitting on ':'
        let parts: Vec<&str> = pair.split(NAME_VALUE_SEPARATOR).collect();
        if parts.len() != 2 {
            return None;
        }

        let name_bytes = engine.decode(parts[0]).ok()?;
        let value_bytes = engine.decode(parts[1]).ok()?;

        headers.push(HttpHeader {
            name: String::from_utf8(name_bytes).ok()?,
            value: String::from_utf8(value_bytes).ok()?,
        });
    }

    Some(headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_header_stripping() {
        // Should be stripped
        assert!(is_control_response_header(":status"));
        assert!(is_control_response_header("cf-int-something"));
        assert!(is_control_response_header("cf-cloudflared-request-headers"));
        assert!(is_control_response_header("cf-cloudflared-response-headers"));
        assert!(is_control_response_header("cf-cloudflared-response-meta"));
        assert!(is_control_response_header("cf-proxy-something"));

        // Case-insensitive
        assert!(is_control_response_header("Cf-Int-Something"));
        assert!(is_control_response_header("CF-CLOUDFLARED-RESPONSE-META"));

        // Should NOT be stripped
        assert!(!is_control_response_header("content-type"));
        assert!(!is_control_response_header("x-custom-header"));
        assert!(!is_control_response_header("authorization"));
    }

    #[test]
    fn response_meta_values_are_valid_json() {
        // Verify the pre-generated constants are valid JSON by parsing them
        let origin: serde_json::Value =
            serde_json::from_str(RESPONSE_META_ORIGIN).expect("origin meta should be valid JSON");
        assert_eq!(origin["src"], "origin");

        let cfd: serde_json::Value =
            serde_json::from_str(RESPONSE_META_CLOUDFLARED).expect("cloudflared meta should be valid JSON");
        assert_eq!(cfd["src"], "cloudflared");

        let limited: serde_json::Value = serde_json::from_str(RESPONSE_META_CLOUDFLARED_FLOW_LIMITED)
            .expect("flow-limited meta should be valid JSON");
        assert_eq!(limited["src"], "cloudflared");
        assert_eq!(limited["flow_rate_limited"], true);
    }

    #[test]
    fn header_metadata_key_format() {
        assert_eq!(header_metadata_key("Content-Type"), "HttpHeader:Content-Type");
        assert_eq!(header_metadata_prefix(), "HttpHeader:");
    }

    #[test]
    fn websocket_client_header_detection() {
        assert!(is_websocket_client_header("sec-websocket-accept"));
        assert!(is_websocket_client_header("Sec-Websocket-Accept"));
        assert!(is_websocket_client_header("connection"));
        assert!(is_websocket_client_header("Connection"));
        assert!(is_websocket_client_header("upgrade"));
        assert!(is_websocket_client_header("Upgrade"));

        assert!(!is_websocket_client_header("sec-websocket-key"));
        assert!(!is_websocket_client_header("content-type"));
        assert!(!is_websocket_client_header(""));
    }

    #[test]
    fn header_serialization_roundtrip() {
        let original = vec![
            HttpHeader {
                name: "Content-Type".into(),
                value: "application/json".into(),
            },
            HttpHeader {
                name: "Authorization".into(),
                value: "Bearer token123".into(),
            },
        ];

        let serialized = serialize_headers(&original);
        let deserialized = deserialize_headers(&serialized).expect("should deserialize");

        assert_eq!(deserialized, original);
    }

    #[test]
    fn header_serialization_empty() {
        let serialized = serialize_headers(&[]);
        assert_eq!(serialized, "");

        let deserialized = deserialize_headers("").expect("empty should parse");
        assert!(deserialized.is_empty());
    }

    #[test]
    fn header_serialization_single() {
        let original = vec![HttpHeader {
            name: "Host".into(),
            value: "example.com".into(),
        }];

        let serialized = serialize_headers(&original);
        assert!(
            !serialized.contains(';'),
            "single header should have no separator"
        );

        let deserialized = deserialize_headers(&serialized).expect("should parse");
        assert_eq!(deserialized, original);
    }

    #[test]
    fn header_serialization_no_padding() {
        // Verify base64 output has no '=' padding characters
        let headers = vec![HttpHeader {
            name: "X".into(),
            value: "Y".into(),
        }];

        let serialized = serialize_headers(&headers);
        assert!(
            !serialized.contains('='),
            "serialized headers must not contain base64 padding: {serialized}"
        );
    }

    #[test]
    fn header_deserialization_rejects_malformed() {
        // No colon separator
        assert!(deserialize_headers("abc").is_none());

        // Too many colons in one pair
        assert!(deserialize_headers("abc:def:ghi").is_none());

        // Invalid base64
        assert!(deserialize_headers("!!!:???").is_none());
    }

    #[test]
    fn header_serialization_special_characters() {
        let original = vec![HttpHeader {
            name: "X-Custom-Header".into(),
            value: "value with spaces & special=chars".into(),
        }];

        let serialized = serialize_headers(&original);
        let deserialized = deserialize_headers(&serialized).expect("should roundtrip");
        assert_eq!(deserialized, original);
    }
}
