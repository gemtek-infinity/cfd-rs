use std::path::PathBuf;

use url::Url;

use crate::error::{ConfigError, Result};

use super::IngressService;

pub(super) fn parse_ingress_service(field: &'static str, value: &str) -> Result<IngressService> {
    if let Some(path) = value.strip_prefix("unix+tls:") {
        return Ok(IngressService::UnixSocketTls(PathBuf::from(path)));
    }

    if let Some(path) = value.strip_prefix("unix:") {
        return Ok(IngressService::UnixSocket(PathBuf::from(path)));
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
        return Ok(IngressService::HttpStatus(parsed));
    }

    if value == "hello_world" {
        return Ok(IngressService::HelloWorld);
    }
    if value == "bastion" {
        return Ok(IngressService::Bastion);
    }
    if value == "socks-proxy" {
        return Ok(IngressService::SocksProxy);
    }

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

    if is_http_scheme(url.scheme()) {
        Ok(IngressService::Http(url))
    } else {
        Ok(IngressService::TcpOverWebsocket(url))
    }
}

pub(super) fn is_http_scheme(scheme: &str) -> bool {
    matches!(scheme, "http" | "https" | "ws" | "wss")
}
