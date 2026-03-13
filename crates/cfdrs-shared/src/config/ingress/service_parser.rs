use std::path::PathBuf;

use url::Url;

use crate::config::error::{ConfigError, Result};

use super::IngressService;

/// Classification of URL schemes for origin service routing.
///
/// Centralizes the scheme-to-service-kind mapping so that the two call
/// sites (config-file parsing and flag parsing) share one decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OriginSchemeClass {
    /// HTTP-family scheme (http, https, ws, wss).
    HttpLike,
    /// Any other scheme is tunneled over WebSocket.
    TcpOverWebsocket,
}

impl OriginSchemeClass {
    pub(super) fn from_scheme(scheme: &str) -> Self {
        match scheme {
            "http" | "https" | "ws" | "wss" => Self::HttpLike,
            _ => Self::TcpOverWebsocket,
        }
    }

    pub(super) fn into_ingress_service(self, url: Url) -> IngressService {
        match self {
            Self::HttpLike => IngressService::Http(url),
            Self::TcpOverWebsocket => IngressService::TcpOverWebsocket(url),
        }
    }
}

pub(super) fn parse_ingress_service(field: &'static str, value: &str) -> Result<IngressService> {
    if let Some(service) = parse_prefixed_service(value)? {
        return Ok(service);
    }

    if let Some(service) = parse_keyword_service(value) {
        return Ok(service);
    }

    parse_url_service(field, value)
}

fn parse_prefixed_service(value: &str) -> Result<Option<IngressService>> {
    if let Some(path) = value.strip_prefix("unix+tls:") {
        return Ok(Some(IngressService::UnixSocketTls(PathBuf::from(path))));
    }

    if let Some(path) = value.strip_prefix("unix:") {
        return Ok(Some(IngressService::UnixSocket(PathBuf::from(path))));
    }

    if let Some(code) = value.strip_prefix("http_status:") {
        let parsed = code
            .parse::<u16>()
            .map_err(|_| ConfigError::invalid_ingress_service(value, "status code must be an integer"))?;
        if !(100..=999).contains(&parsed) {
            return Err(ConfigError::invalid_ingress_service(
                value,
                "status code must be between 100 and 999",
            ));
        }
        return Ok(Some(IngressService::HttpStatus(parsed)));
    }

    Ok(None)
}

fn parse_keyword_service(value: &str) -> Option<IngressService> {
    match value {
        "hello_world" => Some(IngressService::HelloWorld),
        "bastion" => Some(IngressService::Bastion),
        "socks-proxy" => Some(IngressService::SocksProxy),
        _ => None,
    }
}

fn parse_url_service(field: &'static str, value: &str) -> Result<IngressService> {
    let mut url = Url::parse(value).map_err(|source| match source {
        url::ParseError::RelativeUrlWithoutBase => {
            ConfigError::invalid_ingress_service(value, "address must include a scheme and hostname")
        }
        other => ConfigError::invalid_url(field, value, other),
    })?;

    if url.scheme().is_empty() || url.host_str().is_none() {
        return Err(ConfigError::invalid_ingress_service(
            value,
            "address must include a scheme and hostname",
        ));
    }

    if !url.path().is_empty() && url.path() != "/" {
        return Err(ConfigError::invalid_ingress_service(
            value,
            "origin service addresses must not include a path",
        ));
    }
    if url.path() == "/" {
        url.set_path("");
    }

    Ok(OriginSchemeClass::from_scheme(url.scheme()).into_ingress_service(url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheme_classification_http_family() {
        assert_eq!(
            OriginSchemeClass::from_scheme("http"),
            OriginSchemeClass::HttpLike
        );
        assert_eq!(
            OriginSchemeClass::from_scheme("https"),
            OriginSchemeClass::HttpLike
        );
        assert_eq!(OriginSchemeClass::from_scheme("ws"), OriginSchemeClass::HttpLike);
        assert_eq!(OriginSchemeClass::from_scheme("wss"), OriginSchemeClass::HttpLike);
    }

    #[test]
    fn scheme_classification_non_http() {
        assert_eq!(
            OriginSchemeClass::from_scheme("tcp"),
            OriginSchemeClass::TcpOverWebsocket
        );
        assert_eq!(
            OriginSchemeClass::from_scheme("ssh"),
            OriginSchemeClass::TcpOverWebsocket
        );
        assert_eq!(
            OriginSchemeClass::from_scheme("ftp"),
            OriginSchemeClass::TcpOverWebsocket
        );
    }

    #[test]
    fn http_like_produces_http_service() {
        let url = Url::parse("https://localhost:8080").expect("valid url");
        let service = OriginSchemeClass::HttpLike.into_ingress_service(url.clone());
        assert_eq!(service, IngressService::Http(url));
    }

    #[test]
    fn tcp_over_websocket_produces_correct_service() {
        let url = Url::parse("tcp://localhost:22").expect("valid url");
        let service = OriginSchemeClass::TcpOverWebsocket.into_ingress_service(url.clone());
        assert_eq!(service, IngressService::TcpOverWebsocket(url));
    }
}
