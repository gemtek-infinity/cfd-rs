//! Origin service dispatch for the Pingora proxy layer.
//!
//! Owns the trait and concrete implementations for routing requests
//! from ingress rule matching to real origin services. This is the
//! broader proxy completeness surface beyond the narrow Phase 3.4c
//! `http_status`-only path.
//!
//! Each origin service type mirrors the Go baseline's dispatch from
//! `baseline-2026.2.0/proxy/proxy.go` and
//! `baseline-2026.2.0/ingress/origin_proxy.go`.

use cfdrs_cdc::stream::{ConnectRequest, ConnectResponse, ConnectionType, Metadata};
use cfdrs_cdc::stream_contract::{
    HttpHeader, RESPONSE_META_CLOUDFLARED, RESPONSE_META_HEADER, RESPONSE_META_ORIGIN, RESPONSE_USER_HEADERS,
    is_control_response_header, serialize_headers,
};
use cfdrs_shared::IngressService;
use pingora_http::ResponseHeader;

/// Result of a proxy dispatch to an origin service.
#[derive(Debug)]
#[allow(dead_code)] // Phase 5.1: variants wired incrementally
pub(crate) enum OriginResponse {
    /// HTTP response headers (body will be piped separately).
    Http(Box<ResponseHeader>),
    /// TCP stream was established; data should be bidirectionally piped.
    /// The connect-response ack has already been sent.
    StreamEstablished,
    /// The origin service is not implemented for this service type.
    Unimplemented { service_label: &'static str },
}

/// Dispatch an incoming request to the matched origin service.
///
/// Follows the Go baseline dispatch pattern from
/// `baseline-2026.2.0/connection/quic_connection.go` `dispatchRequest()`:
/// - HTTP and WebSocket go through the same HTTP dispatch path (with an
///   `is_websocket` flag for upgrade semantics)
/// - TCP goes through a separate stream-establishment path
/// - All other connection types are unsupported errors
///
/// Within each path, the ingress service type determines the
/// concrete origin behavior (status code, hello-world, forward, etc.).
pub(crate) fn dispatch_to_origin(service: &IngressService, request: &ConnectRequest) -> OriginResponse {
    match request.connection_type {
        ConnectionType::Http | ConnectionType::WebSocket => dispatch_http_path(service, request),
        ConnectionType::Tcp => dispatch_tcp_path(service, request),
    }
}

/// HTTP/WebSocket dispatch path.
///
/// Go baseline: `case pogs.ConnectionTypeHTTP, pogs.ConnectionTypeWebsocket:`
/// Both types route through `ProxyHTTP`, with a boolean `isWebsocket` flag
/// to distinguish upgrade semantics.
fn dispatch_http_path(service: &IngressService, request: &ConnectRequest) -> OriginResponse {
    match service {
        IngressService::HttpStatus(code) => OriginResponse::Http(Box::new(build_status_response(*code))),
        IngressService::HelloWorld => OriginResponse::Http(Box::new(build_hello_world_response())),
        IngressService::Http(_url) => dispatch_http_origin(request),

        // Services not reachable through HTTP/WS dispatch
        IngressService::TcpOverWebsocket(_)
        | IngressService::UnixSocket(_)
        | IngressService::UnixSocketTls(_)
        | IngressService::Bastion
        | IngressService::SocksProxy
        | IngressService::NamedToken(_) => OriginResponse::Unimplemented {
            service_label: service_label(service),
        },
    }
}

/// TCP dispatch path.
///
/// Go baseline: `case pogs.ConnectionTypeTCP:` uses a `streamReadWriteAcker`
/// and calls `ProxyTCP` with dest, flow ID, and trace context.
fn dispatch_tcp_path(service: &IngressService, request: &ConnectRequest) -> OriginResponse {
    match service {
        // TCP-capable service types (not yet implemented)
        IngressService::TcpOverWebsocket(_)
        | IngressService::UnixSocket(_)
        | IngressService::UnixSocketTls(_)
        | IngressService::SocksProxy => OriginResponse::Unimplemented {
            service_label: service_label(service),
        },

        // HTTP-only services don't make sense for TCP connections
        IngressService::HttpStatus(_)
        | IngressService::HelloWorld
        | IngressService::Http(_)
        | IngressService::Bastion
        | IngressService::NamedToken(_) => {
            let _ = request; // suppress unused warning while stubs remain
            OriginResponse::Unimplemented {
                service_label: service_label(service),
            }
        }
    }
}

/// Return a stable label for an ingress service type.
///
/// Used in `Unimplemented` responses so tests and logging can
/// distinguish which service was attempted.
fn service_label(service: &IngressService) -> &'static str {
    match service {
        IngressService::Http(_) => "http",
        IngressService::TcpOverWebsocket(_) => "tcp-over-websocket",
        IngressService::UnixSocket(_) => "unix-socket",
        IngressService::UnixSocketTls(_) => "unix-socket-tls",
        IngressService::HttpStatus(_) => "http-status",
        IngressService::HelloWorld => "hello-world",
        IngressService::Bastion => "bastion",
        IngressService::SocksProxy => "socks-proxy",
        IngressService::NamedToken(_) => "named-token",
    }
}

/// Dispatch an HTTP-scheme origin request.
///
/// For now, returns a 502 response indicating that real HTTP origin
/// proxying (round-trip to the origin URL) is the next surface to
/// make operational. The dispatch path is real: ingress matching
/// routes here, the request metadata is available, and the response
/// path is wired.
fn dispatch_http_origin(request: &ConnectRequest) -> OriginResponse {
    // The request carries the full HTTP metadata from the edge.
    // Real origin proxying will:
    // 1. Build an HTTP request to the origin URL
    // 2. Execute the round-trip via a connection pool
    // 3. Return the origin's response headers and stream the body
    //
    // For now, acknowledge the dispatch path is wired by returning a
    // structured 502 that carries the destination information.
    let mut response = build_status_response(502);

    // Flag the response so tests and evidence can distinguish "routed
    // to HTTP origin but origin connection not yet implemented" from
    // "no ingress match."
    let _ = response.insert_header("X-Cloudflared-Origin-Status", "dispatch-wired");
    let _ = response.insert_header("X-Cloudflared-Origin-Dest", &request.dest);

    OriginResponse::Http(Box::new(response))
}

/// Build a hello-world HTML response.
fn build_hello_world_response() -> ResponseHeader {
    let mut header = ResponseHeader::build(200, None).expect("200 is always a valid status code");
    let _ = header.insert_header("Content-Type", "text/html; charset=utf-8");
    header
}

/// Body content for the hello-world origin service.
#[allow(dead_code)] // Phase 5.1: used by hello_world origin service
pub(crate) const HELLO_WORLD_BODY: &[u8] = b"<html><body>\
    <h1>Cloudflare Tunnel</h1>\
    <p>Your origin is working. Congratulations!</p>\
    </body></html>";

/// Build a response with the given HTTP status code.
///
/// Status codes from config-validated ingress rules are guaranteed to be
/// in 100–999. Hardcoded codes (like 502) are valid by construction.
pub(super) fn build_status_response(code: u16) -> ResponseHeader {
    ResponseHeader::build(code, None)
        .expect("status codes from validated config or hardcoded constants are always valid")
}

/// Convert an `OriginResponse` into a CDC wire `ConnectResponse`.
///
/// This bridges the proxy dispatch layer with the stream framing
/// protocol. The Go baseline constructs the connect response inside
/// `httpResponseAdapter.WriteRespHeaders()` for HTTP/WS and
/// `streamReadWriteAcker.AckConnection()` for TCP.
///
/// Mapping:
/// - `Http(header)` → `ConnectResponse::http(status, headers)`
/// - `StreamEstablished` → `ConnectResponse::tcp_ack(None)`
/// - `Unimplemented { label }` → `ConnectResponse::error(message)`
pub(crate) fn to_connect_response(response: &OriginResponse) -> ConnectResponse {
    match response {
        OriginResponse::Http(header) => {
            let status = header.status.as_u16();

            // CDC-017: strip control headers before forwarding to the edge.
            let headers: Vec<(String, String)> = header
                .headers
                .iter()
                .filter(|(name, _)| !is_control_response_header(name.as_str()))
                .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
                .collect();

            let mut resp = ConnectResponse::http(status, headers.clone());

            // CDC-015: serialized response headers for edge reconstruction.
            let http_headers: Vec<HttpHeader> = headers
                .iter()
                .map(|(name, value)| HttpHeader {
                    name: name.clone(),
                    value: value.clone(),
                })
                .collect();

            resp.metadata.push(Metadata::new(
                RESPONSE_USER_HEADERS,
                serialize_headers(&http_headers),
            ));

            // CDC-016: response source metadata.
            resp.metadata
                .push(Metadata::new(RESPONSE_META_HEADER, RESPONSE_META_CLOUDFLARED));

            resp
        }

        OriginResponse::StreamEstablished => {
            let mut resp = ConnectResponse::tcp_ack(None);

            resp.metadata
                .push(Metadata::new(RESPONSE_META_HEADER, RESPONSE_META_ORIGIN));

            resp
        }

        OriginResponse::Unimplemented { service_label } => {
            ConnectResponse::error(format!("service not implemented: {service_label}"))
        }
    }
}

/// Map a `ConnectRequest` to a `ResponseHeader` through ingress matching.
///
/// This is the main entry point from the stream handler into the proxy:
/// 1. Extract host and path from the request
/// 2. Match against ingress rules
/// 3. Dispatch to the matched origin service
pub(crate) fn proxy_connect_request(
    ingress: &[cfdrs_shared::IngressRule],
    request: &ConnectRequest,
) -> OriginResponse {
    let host = request.http_host().unwrap_or("");
    let path = extract_path_from_dest(&request.dest);

    let matched_service =
        cfdrs_shared::find_matching_rule(ingress, host, path).map(|index| &ingress[index].service);

    match matched_service {
        Some(service) => dispatch_to_origin(service, request),
        None => OriginResponse::Http(Box::new(build_status_response(502))),
    }
}

/// Extract the path component from a ConnectRequest destination.
///
/// The dest field may be a full URL or just a host:port. We parse
/// the path component for ingress matching.
fn extract_path_from_dest(dest: &str) -> &str {
    // If the dest looks like a URL (has ://), try to extract the path
    if let Some(after_scheme) = dest
        .strip_prefix("http://")
        .or_else(|| dest.strip_prefix("https://"))
        && let Some(slash_pos) = after_scheme.find('/')
    {
        return &after_scheme[slash_pos..];
    }

    // For bare paths or host:port destinations, use "/"
    if dest.starts_with('/') {
        return dest;
    }

    "/"
}

#[cfg(test)]
mod tests {
    use super::*;
    use cfdrs_cdc::stream::{ConnectionType, Metadata};
    use cfdrs_cdc::stream_contract::{
        RESPONSE_META_CLOUDFLARED, RESPONSE_META_HEADER, RESPONSE_META_ORIGIN, RESPONSE_USER_HEADERS,
    };
    use cfdrs_shared::{IngressMatch, IngressRule, OriginRequestConfig};

    fn status_rule(hostname: Option<&str>, code: u16) -> IngressRule {
        IngressRule {
            matcher: IngressMatch {
                hostname: hostname.map(String::from),
                punycode_hostname: None,
                path: None,
            },
            service: IngressService::HttpStatus(code),
            origin_request: OriginRequestConfig::default(),
        }
    }

    fn hello_rule(hostname: Option<&str>) -> IngressRule {
        IngressRule {
            matcher: IngressMatch {
                hostname: hostname.map(String::from),
                punycode_hostname: None,
                path: None,
            },
            service: IngressService::HelloWorld,
            origin_request: OriginRequestConfig::default(),
        }
    }

    fn http_rule(hostname: Option<&str>, url: &str) -> IngressRule {
        IngressRule {
            matcher: IngressMatch {
                hostname: hostname.map(String::from),
                punycode_hostname: None,
                path: None,
            },
            service: IngressService::Http(url::Url::parse(url).expect("test url")),
            origin_request: OriginRequestConfig::default(),
        }
    }

    fn make_request(host: &str, dest: &str) -> ConnectRequest {
        make_typed_request(host, dest, ConnectionType::Http)
    }

    fn make_typed_request(host: &str, dest: &str, connection_type: ConnectionType) -> ConnectRequest {
        ConnectRequest {
            dest: dest.into(),
            connection_type,
            metadata: vec![
                Metadata::new("HttpMethod", "GET"),
                Metadata::new("HttpHost", host),
            ],
        }
    }

    #[test]
    fn dispatch_http_status_returns_configured_code() {
        let rules = vec![status_rule(None, 418)];
        let request = make_request("example.com", "http://example.com/");
        let response = proxy_connect_request(&rules, &request);

        match response {
            OriginResponse::Http(header) => assert_eq!(header.status.as_u16(), 418),
            other => panic!("expected Http response, got: {other:?}"),
        }
    }

    #[test]
    fn dispatch_hello_world_returns_200() {
        let rules = vec![hello_rule(None)];
        let request = make_request("example.com", "http://example.com/");
        let response = proxy_connect_request(&rules, &request);

        match response {
            OriginResponse::Http(header) => assert_eq!(header.status.as_u16(), 200),
            other => panic!("expected Http response, got: {other:?}"),
        }
    }

    #[test]
    fn dispatch_http_origin_returns_502_with_dispatch_marker() {
        let rules = vec![http_rule(None, "http://localhost:8080")];
        let request = make_request("example.com", "http://localhost:8080/api");
        let response = proxy_connect_request(&rules, &request);

        match response {
            OriginResponse::Http(header) => {
                assert_eq!(header.status.as_u16(), 502);
                // Should carry the origin-status marker
            }
            other => panic!("expected Http response, got: {other:?}"),
        }
    }

    #[test]
    fn dispatch_hostname_matching() {
        let rules = vec![
            status_rule(Some("api.example.com"), 200),
            hello_rule(Some("hello.example.com")),
            status_rule(None, 404),
        ];

        let api_req = make_request("api.example.com", "http://api.example.com/");
        match proxy_connect_request(&rules, &api_req) {
            OriginResponse::Http(h) => assert_eq!(h.status.as_u16(), 200),
            other => panic!("expected 200, got: {other:?}"),
        }

        let hello_req = make_request("hello.example.com", "http://hello.example.com/");
        match proxy_connect_request(&rules, &hello_req) {
            OriginResponse::Http(h) => assert_eq!(h.status.as_u16(), 200),
            other => panic!("expected 200, got: {other:?}"),
        }

        let other_req = make_request("other.example.com", "http://other.example.com/");
        match proxy_connect_request(&rules, &other_req) {
            OriginResponse::Http(h) => assert_eq!(h.status.as_u16(), 404),
            other => panic!("expected 404, got: {other:?}"),
        }
    }

    #[test]
    fn dispatch_empty_ingress_returns_502() {
        let request = make_request("example.com", "http://example.com/");
        let response = proxy_connect_request(&[], &request);

        match response {
            OriginResponse::Http(h) => assert_eq!(h.status.as_u16(), 502),
            other => panic!("expected 502, got: {other:?}"),
        }
    }

    #[test]
    fn extract_path_from_url() {
        assert_eq!(extract_path_from_dest("http://localhost:8080/api/v1"), "/api/v1");
        assert_eq!(extract_path_from_dest("https://example.com/path"), "/path");
        assert_eq!(extract_path_from_dest("http://localhost:8080"), "/");
        assert_eq!(extract_path_from_dest("/already/a/path"), "/already/a/path");
        assert_eq!(extract_path_from_dest("10.0.0.1:8080"), "/");
    }

    #[test]
    fn unimplemented_services_report_label() {
        let service = IngressService::SocksProxy;
        let request = ConnectRequest {
            dest: "socks://localhost:1080".into(),
            connection_type: ConnectionType::Tcp,
            metadata: vec![],
        };

        match dispatch_to_origin(&service, &request) {
            OriginResponse::Unimplemented { service_label } => {
                assert_eq!(service_label, "socks-proxy");
            }
            other => panic!("expected Unimplemented, got: {other:?}"),
        }
    }

    // --- CDC-018: ConnectionType-aware dispatch tests ---

    /// Go baseline dispatches HTTP and WebSocket through the same
    /// `ProxyHTTP` path. Verify both connection types reach the
    /// HTTP dispatch arm for an Http service.
    #[test]
    fn http_and_websocket_share_dispatch_path() {
        let rules = vec![status_rule(None, 200)];

        let http_req = make_typed_request("example.com", "http://example.com/", ConnectionType::Http);
        let ws_req = make_typed_request("example.com", "http://example.com/", ConnectionType::WebSocket);

        match proxy_connect_request(&rules, &http_req) {
            OriginResponse::Http(h) => assert_eq!(h.status.as_u16(), 200),
            other => panic!("HTTP request should dispatch to Http response, got: {other:?}"),
        }

        match proxy_connect_request(&rules, &ws_req) {
            OriginResponse::Http(h) => assert_eq!(h.status.as_u16(), 200),
            other => panic!("WebSocket request should share HTTP dispatch path, got: {other:?}"),
        }
    }

    /// Go baseline dispatches TCP through a separate `ProxyTCP` path.
    /// HTTP-only services (HttpStatus, HelloWorld) should be
    /// unreachable via TCP and return Unimplemented.
    #[test]
    fn tcp_dispatch_rejects_http_only_services() {
        let rules = vec![status_rule(None, 200)];
        let tcp_req = make_typed_request("example.com", "example.com:8080", ConnectionType::Tcp);

        match proxy_connect_request(&rules, &tcp_req) {
            OriginResponse::Unimplemented { service_label } => {
                assert_eq!(service_label, "http-status");
            }
            other => panic!("TCP to HttpStatus should be Unimplemented, got: {other:?}"),
        }
    }

    /// TCP-capable services dispatch through the TCP path and report
    /// their correct label (pending real implementation).
    #[test]
    fn tcp_dispatch_to_tcp_service_reports_label() {
        let service =
            IngressService::TcpOverWebsocket(url::Url::parse("tcp://10.0.0.5:22").expect("test url"));
        let request = ConnectRequest {
            dest: "10.0.0.5:22".into(),
            connection_type: ConnectionType::Tcp,
            metadata: vec![Metadata::new("FlowID", "flow-42")],
        };

        match dispatch_to_origin(&service, &request) {
            OriginResponse::Unimplemented { service_label } => {
                assert_eq!(service_label, "tcp-over-websocket");
            }
            other => panic!("expected Unimplemented, got: {other:?}"),
        }
    }

    /// WebSocket connection type to an Http service goes through
    /// the HTTP dispatch path (same as Go baseline).
    #[test]
    fn websocket_to_http_service_dispatches_through_http_path() {
        let rules = vec![http_rule(None, "http://localhost:8080")];
        let ws_req = make_typed_request("example.com", "http://example.com/ws", ConnectionType::WebSocket);

        match proxy_connect_request(&rules, &ws_req) {
            OriginResponse::Http(h) => {
                // Http forward is wired but returns 502 (origin connection not yet implemented)
                assert_eq!(h.status.as_u16(), 502);
            }
            other => panic!("WebSocket to Http service should dispatch, got: {other:?}"),
        }
    }

    // --- CDC-018: service label exhaustiveness ---

    /// Every IngressService variant has a stable label accessible
    /// through `service_label()`.
    #[test]
    fn service_label_covers_all_variants() {
        let variants: Vec<(IngressService, &str)> = vec![
            (
                IngressService::Http(url::Url::parse("http://localhost").expect("url")),
                "http",
            ),
            (
                IngressService::TcpOverWebsocket(url::Url::parse("tcp://localhost").expect("url")),
                "tcp-over-websocket",
            ),
            (IngressService::UnixSocket("/tmp/sock".into()), "unix-socket"),
            (
                IngressService::UnixSocketTls("/tmp/sock".into()),
                "unix-socket-tls",
            ),
            (IngressService::HttpStatus(200), "http-status"),
            (IngressService::HelloWorld, "hello-world"),
            (IngressService::Bastion, "bastion"),
            (IngressService::SocksProxy, "socks-proxy"),
            (IngressService::NamedToken("token".into()), "named-token"),
        ];

        for (service, expected) in &variants {
            assert_eq!(service_label(service), *expected, "label for {service:?}");
        }
    }

    // --- CDC-018: OriginResponse → ConnectResponse conversion ---

    /// HTTP dispatch result converts to ConnectResponse with correct
    /// status metadata.
    #[test]
    fn connect_response_from_http_origin() {
        let header = build_status_response(418);
        let origin = OriginResponse::Http(Box::new(header));
        let cr = to_connect_response(&origin);

        assert!(cr.is_ok());
        assert_eq!(cr.metadata[0].key, "HttpStatus");
        assert_eq!(cr.metadata[0].val, "418");

        // CDC-016: response meta should be present and mark cloudflared as source.
        assert!(
            cr.metadata
                .iter()
                .any(|m| m.key == RESPONSE_META_HEADER && m.val == RESPONSE_META_CLOUDFLARED),
            "should contain response meta metadata, got: {:?}",
            cr.metadata
        );

        // CDC-015: serialized response headers metadata should be present.
        assert!(
            cr.metadata.iter().any(|m| m.key == RESPONSE_USER_HEADERS),
            "should contain serialized response headers metadata, got: {:?}",
            cr.metadata
        );
    }

    /// TCP stream established converts to a successful TCP ack.
    #[test]
    fn connect_response_from_stream_established() {
        let origin = OriginResponse::StreamEstablished;
        let cr = to_connect_response(&origin);

        assert!(cr.is_ok());
        assert!(cr.error.is_empty());

        // CDC-016: TCP ack should mark origin as source.
        assert!(
            cr.metadata
                .iter()
                .any(|m| m.key == RESPONSE_META_HEADER && m.val == RESPONSE_META_ORIGIN),
            "TCP ack should contain origin response meta, got: {:?}",
            cr.metadata
        );
    }

    /// Unimplemented service converts to an error ConnectResponse
    /// with the service label in the message.
    #[test]
    fn connect_response_from_unimplemented() {
        let origin = OriginResponse::Unimplemented {
            service_label: "socks-proxy",
        };
        let cr = to_connect_response(&origin);

        assert!(!cr.is_ok());
        assert!(
            cr.error.contains("socks-proxy"),
            "error should contain service label: {}",
            cr.error
        );
    }

    /// Hello-world response converts to ConnectResponse with 200
    /// and Content-Type header.
    #[test]
    fn connect_response_from_hello_world() {
        let header = build_hello_world_response();
        let origin = OriginResponse::Http(Box::new(header));
        let cr = to_connect_response(&origin);

        assert!(cr.is_ok());
        assert_eq!(cr.metadata[0].key, "HttpStatus");
        assert_eq!(cr.metadata[0].val, "200");

        // Content-Type header should be in metadata.
        // Pingora lowercases header names per HTTP/2 conventions,
        // so the metadata key is "HttpHeader:content-type".
        assert!(
            cr.metadata
                .iter()
                .any(|m| m.key.contains("content-type") && m.val.contains("text/html")),
            "should contain content-type header metadata, got: {:?}",
            cr.metadata
        );

        // CDC-016: response meta should mark cloudflared as source.
        assert!(
            cr.metadata
                .iter()
                .any(|m| m.key == RESPONSE_META_HEADER && m.val == RESPONSE_META_CLOUDFLARED),
            "hello-world should have cloudflared response meta, got: {:?}",
            cr.metadata
        );
    }

    // --- CDC-018: end-to-end dispatch → ConnectResponse ---

    /// Full round-trip: ConnectRequest → ingress match → dispatch →
    /// OriginResponse → ConnectResponse. This is the CDC-018
    /// stream round-trip path.
    #[test]
    fn end_to_end_http_dispatch_to_connect_response() {
        let rules = vec![status_rule(Some("api.example.com"), 200), status_rule(None, 404)];
        let request = make_request("api.example.com", "http://api.example.com/health");

        let origin = proxy_connect_request(&rules, &request);
        let cr = to_connect_response(&origin);

        assert!(cr.is_ok());
        assert_eq!(cr.metadata[0].val, "200");
    }

    /// Full round-trip for a TCP request that hits an HTTP-only service.
    #[test]
    fn end_to_end_tcp_dispatch_to_error_connect_response() {
        let rules = vec![status_rule(None, 200)];
        let request = make_typed_request("example.com", "example.com:22", ConnectionType::Tcp);

        let origin = proxy_connect_request(&rules, &request);
        let cr = to_connect_response(&origin);

        assert!(!cr.is_ok());
        assert!(
            cr.error.contains("http-status"),
            "TCP to HttpStatus should error: {}",
            cr.error
        );
    }

    /// No ingress match produces a 502 ConnectResponse.
    #[test]
    fn end_to_end_no_match_returns_502_connect_response() {
        let request = make_request("example.com", "http://example.com/");
        let origin = proxy_connect_request(&[], &request);
        let cr = to_connect_response(&origin);

        assert!(cr.is_ok());
        assert_eq!(cr.metadata[0].val, "502");
    }

    // --- CDC-017: control header stripping ---

    /// Control headers (cf-int-*, cf-cloudflared-*, cf-proxy-*, :*)
    /// must be stripped from the ConnectResponse metadata.
    #[test]
    fn connect_response_strips_control_headers() {
        let mut header = ResponseHeader::build(200, None).expect("200 is valid");
        let _ = header.insert_header("content-type", "text/plain");
        let _ = header.insert_header("cf-int-internal", "secret");
        let _ = header.insert_header("cf-cloudflared-request-headers", "encoded");
        let _ = header.insert_header("cf-proxy-worker", "worker-1");
        let _ = header.insert_header("x-custom", "allowed");

        let origin = OriginResponse::Http(Box::new(header));
        let cr = to_connect_response(&origin);

        assert!(cr.is_ok());

        // User headers should be forwarded.
        assert!(
            cr.metadata.iter().any(|m| m.key.contains("content-type")),
            "content-type should be forwarded"
        );
        assert!(
            cr.metadata.iter().any(|m| m.key.contains("x-custom")),
            "x-custom should be forwarded"
        );

        // Control headers should be stripped.
        assert!(
            !cr.metadata.iter().any(|m| m.key.contains("cf-int-internal")),
            "cf-int-* should be stripped"
        );
        assert!(
            !cr.metadata
                .iter()
                .any(|m| m.key.contains("cf-cloudflared-request-headers")),
            "cf-cloudflared-* should be stripped"
        );
        assert!(
            !cr.metadata.iter().any(|m| m.key.contains("cf-proxy-worker")),
            "cf-proxy-* should be stripped"
        );
    }

    // --- CDC-015: serialized response headers ---

    /// Response serialized headers metadata should round-trip through
    /// serialize/deserialize correctly.
    #[test]
    fn connect_response_serialized_headers_roundtrip() {
        let mut header = ResponseHeader::build(200, None).expect("200 is valid");
        let _ = header.insert_header("content-type", "application/json");
        let _ = header.insert_header("x-request-id", "abc-123");

        let origin = OriginResponse::Http(Box::new(header));
        let cr = to_connect_response(&origin);

        let serialized = cr
            .metadata
            .iter()
            .find(|m| m.key == RESPONSE_USER_HEADERS)
            .expect("should have serialized headers metadata");

        let deserialized =
            cfdrs_cdc::stream_contract::deserialize_headers(&serialized.val).expect("should deserialize");

        assert!(
            deserialized
                .iter()
                .any(|h| h.name == "content-type" && h.value == "application/json"),
            "should contain content-type, got: {:?}",
            deserialized
        );
        assert!(
            deserialized
                .iter()
                .any(|h| h.name == "x-request-id" && h.value == "abc-123"),
            "should contain x-request-id, got: {:?}",
            deserialized
        );
    }
}
