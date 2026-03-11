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

impl OriginRequestConfig {
    pub fn materialized_config_defaults(raw: &Self) -> Self {
        super::origin_request::materialize_defaults(raw)
    }

    pub fn with_overrides(&self, overrides: &Self) -> Self {
        super::origin_request::merge_overrides(self, overrides)
    }
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
    Http(Url),
    TcpOverWebsocket(Url),
    UnixSocket(PathBuf),
    UnixSocketTls(PathBuf),
    HttpStatus(u16),
    HelloWorld,
    Bastion,
    SocksProxy,
    NamedToken(String),
}

impl IngressService {
    pub fn parse(field: &'static str, value: &str) -> Result<Self> {
        super::service_parser::parse_ingress_service(field, value)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct IngressMatch {
    pub hostname: Option<String>,
    pub punycode_hostname: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct IngressRule {
    pub matcher: IngressMatch,
    pub service: IngressService,
    pub origin_request: OriginRequestConfig,
}

impl IngressRule {
    pub fn from_raw(
        raw: RawIngressRule,
        inherited_origin_request: &OriginRequestConfig,
        rule_index: usize,
        total_rules: usize,
    ) -> Result<Self> {
        let service = parse_required_service(raw.service.as_deref())?;
        let matcher = build_ingress_match(raw.hostname, raw.path, rule_index, total_rules)?;

        Ok(Self {
            matcher,
            service,
            origin_request: inherited_origin_request.with_overrides(&raw.origin_request),
        })
    }

    pub fn is_catch_all(&self) -> bool {
        self.matcher.hostname.is_none() && self.matcher.path.is_none()
    }

    pub fn matches(&self, hostname: &str, path: &str) -> bool {
        super::matching::matches_rule(self, hostname, path)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct IngressFlagRequest {
    pub hello_world: bool,
    pub bastion: bool,
    pub url: Option<String>,
    pub unix_socket: Option<String>,
}

impl IngressFlagRequest {
    pub fn from_flags(flags: &[String]) -> Self {
        super::flag_surface::parse_flag_request(flags)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct NormalizedIngress {
    pub rules: Vec<IngressRule>,
    pub defaults: OriginRequestConfig,
}

impl NormalizedIngress {
    pub fn from_flag_request(request: &IngressFlagRequest) -> Result<Self> {
        super::flag_surface::normalize_from_flag_request(request)
    }

    pub fn find_matching_rule(&self, hostname: &str, path: &str) -> Option<usize> {
        super::matching::find_matching_rule(&self.rules, hostname, path)
    }
}

fn parse_required_service(service: Option<&str>) -> Result<IngressService> {
    match service {
        Some(service) => IngressService::parse("service", service),
        None => Err(ConfigError::invariant("ingress rule is missing service")),
    }
}

fn build_ingress_match(
    hostname: Option<String>,
    path: Option<String>,
    rule_index: usize,
    total_rules: usize,
) -> Result<IngressMatch> {
    super::validation::validate_hostname(hostname.as_deref(), path.as_deref(), rule_index, total_rules)?;
    let punycode_hostname = super::validation::normalized_punycode_hostname(hostname.as_deref())?;

    Ok(IngressMatch {
        hostname,
        punycode_hostname,
        path,
    })
}
