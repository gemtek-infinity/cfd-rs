// ---------------------------------------------------------------------------
// Per-stream metadata keys (CDC-014)
// ---------------------------------------------------------------------------

pub(crate) const HTTP_METHOD_KEY: &str = "HttpMethod";
pub(crate) const HTTP_HOST_KEY: &str = "HttpHost";
pub(crate) const HTTP_HEADER_KEY: &str = "HttpHeader";
pub(crate) const HTTP_STATUS_KEY: &str = "HttpStatus";
pub(crate) const FLOW_ID_KEY: &str = "FlowID";
pub(crate) const CF_TRACE_ID_KEY: &str = "cf-trace-id";
#[cfg(test)]
pub(crate) const CONTENT_LENGTH_KEY: &str = "HttpHeader:Content-Length";
pub(crate) const TRACE_CONTEXT_KEY: &str = "cf-trace-context";
pub(crate) const DEFAULT_HTTP_METHOD: &str = "GET";
pub(crate) const HTTP_LABEL: &str = "HTTP";
pub(crate) const WEBSOCKET_LABEL: &str = "WebSocket";
pub(crate) const TCP_LABEL: &str = "TCP";
const HEADER_SEPARATOR: &str = ":";

pub(crate) fn header_metadata_key(name: &str) -> String {
    format!("{HTTP_HEADER_KEY}{HEADER_SEPARATOR}{name}")
}

pub(crate) fn header_metadata_prefix() -> String {
    header_metadata_key("")
}

// ---------------------------------------------------------------------------
// Internal transport headers (CDC-015)
//
// These constants define the baseline wire-header keys for serialized user
// headers and response metadata.  They will be consumed when stream-level
// header handling is wired up.
// ---------------------------------------------------------------------------

#[allow(dead_code)]
/// Header carrying base64-serialized request headers from cloudflared to edge.
pub(crate) const REQUEST_USER_HEADERS: &str = "cf-cloudflared-request-headers";

#[allow(dead_code)]
/// Header carrying base64-serialized response headers from edge to cloudflared.
pub(crate) const RESPONSE_USER_HEADERS: &str = "cf-cloudflared-response-headers";

#[allow(dead_code)]
/// Header carrying response source metadata (JSON).
pub(crate) const RESPONSE_META_HEADER: &str = "cf-cloudflared-response-meta";

// ---------------------------------------------------------------------------
// Response meta header values (CDC-016)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
/// Response originated from the origin server.
pub(crate) const RESPONSE_META_ORIGIN: &str = r#"{"src":"origin"}"#;

#[allow(dead_code)]
/// Response originated from cloudflared itself.
pub(crate) const RESPONSE_META_CLOUDFLARED: &str = r#"{"src":"cloudflared"}"#;

#[allow(dead_code)]
/// Response originated from cloudflared with flow rate limiting active.
pub(crate) const RESPONSE_META_CLOUDFLARED_FLOW_LIMITED: &str =
    r#"{"src":"cloudflared","flow_rate_limited":true}"#;

// ---------------------------------------------------------------------------
// Control response header stripping (CDC-017)
// ---------------------------------------------------------------------------

/// Prefixes that identify internal control response headers.
///
/// Headers with these prefixes are stripped in the eyeball ← origin
/// direction. Matches Go's `IsControlResponseHeader` in `header.go`.
#[allow(dead_code)]
const CONTROL_HEADER_PREFIXES: &[&str] = &[":", "cf-int-", "cf-cloudflared-", "cf-proxy-"];

/// Test whether a response header is an internal control header that should
/// be stripped before forwarding to the eyeball.
///
/// Matches Go's `IsControlResponseHeader` from `connection/header.go`.
#[allow(dead_code)]
pub(crate) fn is_control_response_header(header_name: &str) -> bool {
    let lower = header_name.to_ascii_lowercase();

    CONTROL_HEADER_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
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
}
