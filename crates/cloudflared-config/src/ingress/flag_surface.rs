use std::path::PathBuf;

use url::Url;

use crate::error::{ConfigError, Result};

use super::{IngressFlagRequest, IngressMatch, IngressRule, IngressService, NormalizedIngress};

pub(super) fn parse_flag_request(flags: &[String]) -> IngressFlagRequest {
    let mut request = IngressFlagRequest::default();

    for flag in flags {
        if flag == "--hello-world" {
            request.hello_world = true;
            continue;
        }

        if let Some(value) = flag.strip_prefix("--hello-world=") {
            request.hello_world = value == "true";
            continue;
        }

        if flag == "--bastion" {
            request.bastion = true;
            continue;
        }

        if let Some(value) = flag.strip_prefix("--bastion=") {
            request.bastion = value == "true";
            continue;
        }

        if let Some(value) = flag.strip_prefix("--url=") {
            request.url = Some(value.to_owned());
            continue;
        }

        if let Some(value) = flag.strip_prefix("--unix-socket=") {
            request.unix_socket = Some(value.to_owned());
        }
    }

    request
}

pub(super) fn normalize_from_flag_request(request: &IngressFlagRequest) -> Result<NormalizedIngress> {
    let defaults = super::origin_request::flag_defaults(request);
    let service = parse_single_origin_service(request)?;
    Ok(NormalizedIngress {
        rules: vec![IngressRule {
            matcher: IngressMatch::default(),
            service,
            origin_request: defaults.clone(),
        }],
        defaults,
    })
}

pub(super) fn parse_ingress_flags(flags: &[String]) -> Result<NormalizedIngress> {
    let request = parse_flag_request(flags);
    normalize_from_flag_request(&request)
}

fn parse_single_origin_service(request: &IngressFlagRequest) -> Result<IngressService> {
    if request.hello_world {
        return Ok(IngressService::HelloWorld);
    }

    if request.bastion {
        return Ok(IngressService::Bastion);
    }

    if let Some(url) = request.url.as_deref() {
        return parse_flag_origin_url(url);
    }

    if let Some(unix_socket) = request.unix_socket.as_deref() {
        return Ok(IngressService::UnixSocket(PathBuf::from(unix_socket)));
    }

    Err(ConfigError::NoIngressRulesFlags)
}

fn parse_flag_origin_url(value: &str) -> Result<IngressService> {
    let mut url = Url::parse(value).map_err(|source| ConfigError::invalid_url("url", value, source))?;

    if url.scheme().is_empty() || url.host_str().is_none() {
        return Err(ConfigError::invalid_ingress_service(
            value,
            "address must include a scheme and hostname",
        ));
    }

    if !url.path().is_empty() && url.path() != "/" {
        url.set_path("");
        url.set_query(None);
        url.set_fragment(None);
    }

    if url.path() == "/" {
        url.set_path("");
    }

    if matches!(url.scheme(), "http" | "https" | "ws" | "wss") {
        Ok(IngressService::Http(url))
    } else {
        Ok(IngressService::TcpOverWebsocket(url))
    }
}
