use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::config::error::{ConfigError, Result};

/// Opaque Go-compatible duration string (e.g. "30s", "1m30s").
///
/// Preserves exact serialization round-trip with the Go `time.Duration`
/// text format. Parsing into a Rust `Duration` is intentionally deferred
/// until the runtime slice that needs real timing.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct DurationSpec(String);

impl DurationSpec {
    /// Construct from a value known to be a valid Go duration literal.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the inner duration string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DurationSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for DurationSpec {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

impl From<&str> for DurationSpec {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

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
    pub proxy_type: Option<ProxyType>,
    #[serde(rename = "ipRules", default)]
    pub ip_rules: Vec<IngressIpRule>,
    #[serde(rename = "http2Origin", default)]
    pub http2_origin: Option<bool>,
    #[serde(default)]
    pub access: Option<AccessConfig>,
}

/// The type of proxy used for connecting to the origin.
///
/// Replaces the raw `String` used in the Go implementation to prevent
/// silent mismatch from typos or unexpected values.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyType {
    /// SOCKS5 proxy.
    Socks,
}

impl fmt::Display for ProxyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Socks => f.write_str("socks"),
        }
    }
}

impl FromStr for ProxyType {
    type Err = ConfigError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "socks" => Ok(Self::Socks),
            other => Err(ConfigError::invariant(format!("unknown proxy type: {other}"))),
        }
    }
}

impl OriginRequestConfig {
    pub fn builder() -> OriginRequestConfigBuilder {
        OriginRequestConfigBuilder::default()
    }

    pub fn materialized_config_defaults(raw: &Self) -> Self {
        super::origin_request::materialize_defaults(raw)
    }

    pub fn with_overrides(&self, overrides: &Self) -> Self {
        super::origin_request::merge_overrides(self, overrides)
    }
}

/// Fluent builder for programmatic construction of [`OriginRequestConfig`].
///
/// All fields start as `None` / empty and can be set individually.
/// Call [`build`](Self::build) to produce the final config.
#[derive(Debug, Clone, Default)]
pub struct OriginRequestConfigBuilder {
    connect_timeout: Option<DurationSpec>,
    tls_timeout: Option<DurationSpec>,
    tcp_keep_alive: Option<DurationSpec>,
    no_happy_eyeballs: Option<bool>,
    keep_alive_connections: Option<u32>,
    keep_alive_timeout: Option<DurationSpec>,
    http_host_header: Option<String>,
    origin_server_name: Option<String>,
    match_sni_to_host: Option<bool>,
    ca_pool: Option<PathBuf>,
    no_tls_verify: Option<bool>,
    disable_chunked_encoding: Option<bool>,
    bastion_mode: Option<bool>,
    proxy_address: Option<String>,
    proxy_port: Option<u16>,
    proxy_type: Option<ProxyType>,
    ip_rules: Vec<IngressIpRule>,
    http2_origin: Option<bool>,
    access: Option<AccessConfig>,
}

impl OriginRequestConfigBuilder {
    pub fn connect_timeout(mut self, value: DurationSpec) -> Self {
        self.connect_timeout = Some(value);
        self
    }

    pub fn tls_timeout(mut self, value: DurationSpec) -> Self {
        self.tls_timeout = Some(value);
        self
    }

    pub fn tcp_keep_alive(mut self, value: DurationSpec) -> Self {
        self.tcp_keep_alive = Some(value);
        self
    }

    pub fn no_happy_eyeballs(mut self, value: bool) -> Self {
        self.no_happy_eyeballs = Some(value);
        self
    }

    pub fn keep_alive_connections(mut self, value: u32) -> Self {
        self.keep_alive_connections = Some(value);
        self
    }

    pub fn keep_alive_timeout(mut self, value: DurationSpec) -> Self {
        self.keep_alive_timeout = Some(value);
        self
    }

    pub fn http_host_header(mut self, value: String) -> Self {
        self.http_host_header = Some(value);
        self
    }

    pub fn origin_server_name(mut self, value: String) -> Self {
        self.origin_server_name = Some(value);
        self
    }

    pub fn match_sni_to_host(mut self, value: bool) -> Self {
        self.match_sni_to_host = Some(value);
        self
    }

    pub fn ca_pool(mut self, value: PathBuf) -> Self {
        self.ca_pool = Some(value);
        self
    }

    pub fn no_tls_verify(mut self, value: bool) -> Self {
        self.no_tls_verify = Some(value);
        self
    }

    pub fn disable_chunked_encoding(mut self, value: bool) -> Self {
        self.disable_chunked_encoding = Some(value);
        self
    }

    pub fn bastion_mode(mut self, value: bool) -> Self {
        self.bastion_mode = Some(value);
        self
    }

    pub fn proxy_address(mut self, value: String) -> Self {
        self.proxy_address = Some(value);
        self
    }

    pub fn proxy_port(mut self, value: u16) -> Self {
        self.proxy_port = Some(value);
        self
    }

    pub fn proxy_type(mut self, value: ProxyType) -> Self {
        self.proxy_type = Some(value);
        self
    }

    pub fn ip_rules(mut self, value: Vec<IngressIpRule>) -> Self {
        self.ip_rules = value;
        self
    }

    pub fn http2_origin(mut self, value: bool) -> Self {
        self.http2_origin = Some(value);
        self
    }

    pub fn access(mut self, value: AccessConfig) -> Self {
        self.access = Some(value);
        self
    }

    pub fn build(self) -> OriginRequestConfig {
        OriginRequestConfig {
            connect_timeout: self.connect_timeout,
            tls_timeout: self.tls_timeout,
            tcp_keep_alive: self.tcp_keep_alive,
            no_happy_eyeballs: self.no_happy_eyeballs,
            keep_alive_connections: self.keep_alive_connections,
            keep_alive_timeout: self.keep_alive_timeout,
            http_host_header: self.http_host_header,
            origin_server_name: self.origin_server_name,
            match_sni_to_host: self.match_sni_to_host,
            ca_pool: self.ca_pool,
            no_tls_verify: self.no_tls_verify,
            disable_chunked_encoding: self.disable_chunked_encoding,
            bastion_mode: self.bastion_mode,
            proxy_address: self.proxy_address,
            proxy_port: self.proxy_port,
            proxy_type: self.proxy_type,
            ip_rules: self.ip_rules,
            http2_origin: self.http2_origin,
            access: self.access,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_spec_display_matches_inner_value() {
        let spec = DurationSpec::new("30s");
        assert_eq!(spec.to_string(), "30s");
    }

    #[test]
    fn duration_spec_as_str_returns_inner_value() {
        let spec = DurationSpec::new("1m30s");
        assert_eq!(spec.as_str(), "1m30s");
    }

    #[test]
    fn duration_spec_from_str_round_trips() {
        let spec: DurationSpec = "45s".parse().expect("infallible parse");
        assert_eq!(spec.as_str(), "45s");
    }

    #[test]
    fn duration_spec_from_ref_str() {
        let spec = DurationSpec::from("10m");
        assert_eq!(spec.as_str(), "10m");
    }

    #[test]
    fn duration_spec_equality() {
        assert_eq!(DurationSpec::new("30s"), DurationSpec::from("30s"));
    }

    #[test]
    fn proxy_type_display_outputs_lowercase() {
        assert_eq!(ProxyType::Socks.to_string(), "socks");
    }

    #[test]
    fn proxy_type_from_str_parses_socks() {
        let proxy_type: ProxyType = "socks".parse().expect("valid proxy type");
        assert_eq!(proxy_type, ProxyType::Socks);
    }

    #[test]
    fn proxy_type_from_str_rejects_unknown() {
        let result = "http".parse::<ProxyType>();
        assert!(result.is_err());
    }

    #[test]
    fn proxy_type_serde_round_trip() {
        let json = serde_json::to_string(&ProxyType::Socks).expect("serialize");
        assert_eq!(json, "\"socks\"");

        let parsed: ProxyType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, ProxyType::Socks);
    }
}
