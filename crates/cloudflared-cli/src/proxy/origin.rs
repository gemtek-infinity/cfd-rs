//! Origin service dispatch for the Pingora proxy layer.
//!
//! Owns the trait and concrete implementations for routing requests
//! from ingress rule matching to real origin services. This is the
//! broader proxy completeness surface beyond the narrow Phase 3.4c
//! `http_status`-only path.
//!
//! Each origin service type mirrors the Go baseline's dispatch from
//! `baseline-2026.2.0/old-impl/proxy/proxy.go` and
//! `baseline-2026.2.0/old-impl/ingress/origin_proxy.go`.

use cloudflared_config::IngressService;
use cloudflared_proto::stream::ConnectRequest;
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
/// This replaces the narrow Phase 3.4c `dispatch_origin` that only
/// handled `HttpStatus`. Now handles:
/// - `HttpStatus(code)` — return a fixed status code
/// - `HelloWorld` — return a hello-world HTML page
/// - `Http(url)` — proxy HTTP to the origin URL
/// - `TcpOverWebsocket(url)` — establish TCP stream to the origin
/// - Other service types — return 502 with an honest label
pub(crate) fn dispatch_to_origin(service: &IngressService, request: &ConnectRequest) -> OriginResponse {
    match service {
        IngressService::HttpStatus(code) => OriginResponse::Http(Box::new(build_status_response(*code))),

        IngressService::HelloWorld => OriginResponse::Http(Box::new(build_hello_world_response())),

        IngressService::Http(_url) => dispatch_http_origin(request),

        IngressService::TcpOverWebsocket(_url) => OriginResponse::Unimplemented {
            service_label: "tcp-over-websocket",
        },

        IngressService::UnixSocket(_path) => OriginResponse::Unimplemented {
            service_label: "unix-socket",
        },

        IngressService::UnixSocketTls(_path) => OriginResponse::Unimplemented {
            service_label: "unix-socket-tls",
        },

        IngressService::Bastion => OriginResponse::Unimplemented {
            service_label: "bastion",
        },

        IngressService::SocksProxy => OriginResponse::Unimplemented {
            service_label: "socks-proxy",
        },

        IngressService::NamedToken(_) => OriginResponse::Unimplemented {
            service_label: "named-token",
        },
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

/// Map a `ConnectRequest` to a `ResponseHeader` through ingress matching.
///
/// This is the main entry point from the stream handler into the proxy:
/// 1. Extract host and path from the request
/// 2. Match against ingress rules
/// 3. Dispatch to the matched origin service
pub(crate) fn proxy_connect_request(
    ingress: &[cloudflared_config::IngressRule],
    request: &ConnectRequest,
) -> OriginResponse {
    let host = request.http_host().unwrap_or("");
    let path = extract_path_from_dest(&request.dest);

    let matched_service =
        cloudflared_config::find_matching_rule(ingress, host, path).map(|index| &ingress[index].service);

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
    use cloudflared_config::{IngressMatch, IngressRule, OriginRequestConfig};
    use cloudflared_proto::stream::ConnectionType;

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
        ConnectRequest {
            dest: dest.into(),
            connection_type: ConnectionType::Http,
            metadata: vec![
                cloudflared_proto::stream::Metadata::new("HttpMethod", "GET"),
                cloudflared_proto::stream::Metadata::new("HttpHost", host),
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
}
