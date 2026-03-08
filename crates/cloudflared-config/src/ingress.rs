#![forbid(unsafe_code)]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{ConfigError, Result};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct DurationSpec(pub String);

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct AccessConfig {
    #[serde(default)]
    pub required: bool,
    #[serde(rename = "teamName", default)]
    pub team_name: String,
    #[serde(rename = "audTag", default)]
    pub aud_tag: Vec<String>,
    #[serde(default)]
    pub environment: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct IngressIpRule {
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub ports: Vec<u16>,
    #[serde(default)]
    pub allow: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct OriginRequestConfig {
    #[serde(rename = "connectTimeout", default)]
    pub connect_timeout: Option<DurationSpec>,
    #[serde(rename = "tlsTimeout", default)]
    pub tls_timeout: Option<DurationSpec>,
    #[serde(rename = "tcpKeepAlive", default)]
    pub tcp_keep_alive: Option<DurationSpec>,
    #[serde(rename = "noHappyEyeballs", default)]
    pub no_happy_eyeballs: Option<bool>,
    #[serde(rename = "keepAliveConnections", default)]
    pub keep_alive_connections: Option<u32>,
    #[serde(rename = "keepAliveTimeout", default)]
    pub keep_alive_timeout: Option<DurationSpec>,
    #[serde(rename = "httpHostHeader", default)]
    pub http_host_header: Option<String>,
    #[serde(rename = "originServerName", default)]
    pub origin_server_name: Option<String>,
    #[serde(rename = "matchSNItoHost", default)]
    pub match_sni_to_host: Option<bool>,
    #[serde(rename = "caPool", default)]
    pub ca_pool: Option<PathBuf>,
    #[serde(rename = "noTLSVerify", default)]
    pub no_tls_verify: Option<bool>,
    #[serde(rename = "disableChunkedEncoding", default)]
    pub disable_chunked_encoding: Option<bool>,
    #[serde(rename = "bastionMode", default)]
    pub bastion_mode: Option<bool>,
    #[serde(rename = "proxyAddress", default)]
    pub proxy_address: Option<String>,
    #[serde(rename = "proxyPort", default)]
    pub proxy_port: Option<u16>,
    #[serde(rename = "proxyType", default)]
    pub proxy_type: Option<String>,
    #[serde(rename = "ipRules", default)]
    pub ip_rules: Vec<IngressIpRule>,
    #[serde(rename = "http2Origin", default)]
    pub http2_origin: Option<bool>,
    #[serde(default)]
    pub access: Option<AccessConfig>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct RawIngressRule {
    #[serde(default)]
    pub hostname: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(rename = "originRequest", default)]
    pub origin_request: OriginRequestConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum IngressService {
    Uri(Url),
    UnixSocket(PathBuf),
    UnixSocketTls(PathBuf),
    NamedToken(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct IngressMatch {
    pub hostname: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct IngressRule {
    pub matcher: IngressMatch,
    pub service: IngressService,
    pub origin_request: OriginRequestConfig,
}

impl IngressService {
    pub fn parse(field: &'static str, value: &str) -> Result<Self> {
        if let Some(path) = value.strip_prefix("unix+tls:") {
            return Ok(Self::UnixSocketTls(PathBuf::from(path)));
        }

        if let Some(path) = value.strip_prefix("unix:") {
            return Ok(Self::UnixSocket(PathBuf::from(path)));
        }

        match Url::parse(value) {
            Ok(url) => Ok(Self::Uri(url)),
            Err(url::ParseError::RelativeUrlWithoutBase) => Ok(Self::NamedToken(value.to_owned())),
            Err(source) => Err(ConfigError::invalid_url(field, value, source)),
        }
    }
}

impl IngressRule {
    pub fn from_raw(raw: RawIngressRule) -> Result<Self> {
        let service = match raw.service {
            Some(service) => IngressService::parse("service", &service)?,
            None => return Err(ConfigError::invariant("ingress rule is missing service")),
        };

        Ok(Self {
            matcher: IngressMatch {
                hostname: raw.hostname,
                path: raw.path,
            },
            service,
            origin_request: raw.origin_request,
        })
    }

    pub fn is_catch_all(&self) -> bool {
        self.matcher.hostname.is_none() && self.matcher.path.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::{IngressRule, IngressService, RawIngressRule};

    fn ok<T, E: std::fmt::Display>(result: std::result::Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("unexpected error: {error}"),
        }
    }

    #[test]
    fn service_parser_recognizes_url_services() {
        let service = ok(IngressService::parse("service", "https://localhost:8080"));

        match service {
            IngressService::Uri(url) => assert_eq!(url.scheme(), "https"),
            other => panic!("expected URI service, found {other:?}"),
        }
    }

    #[test]
    fn raw_rule_without_hostname_or_path_is_catch_all() {
        let rule = ok(IngressRule::from_raw(RawIngressRule {
            service: Some("https://localhost:8080".to_owned()),
            ..RawIngressRule::default()
        }));

        assert!(rule.is_catch_all());
    }
}
