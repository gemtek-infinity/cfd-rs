use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{ConfigError, Result};

pub const NO_INGRESS_RULES_CLI_MESSAGE: &str = "No ingress rules were defined in provided config (if any) \
                                                nor from the cli, cloudflared will return 503 for all \
                                                incoming HTTP requests";

const DEFAULT_HTTP_CONNECT_TIMEOUT: &str = "30s";
const DEFAULT_TLS_TIMEOUT: &str = "10s";
const DEFAULT_TCP_KEEP_ALIVE: &str = "30s";
const DEFAULT_KEEP_ALIVE_TIMEOUT: &str = "90s";
const DEFAULT_PROXY_ADDRESS: &str = "127.0.0.1";
const DEFAULT_KEEP_ALIVE_CONNECTIONS: u32 = 100;

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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct CliIngressRequest {
    pub hello_world: bool,
    pub bastion: bool,
    pub url: Option<String>,
    pub unix_socket: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct NormalizedIngress {
    pub rules: Vec<IngressRule>,
    pub defaults: OriginRequestConfig,
}

impl IngressService {
    pub fn parse(field: &'static str, value: &str) -> Result<Self> {
        if let Some(path) = value.strip_prefix("unix+tls:") {
            return Ok(Self::UnixSocketTls(PathBuf::from(path)));
        }

        if let Some(path) = value.strip_prefix("unix:") {
            return Ok(Self::UnixSocket(PathBuf::from(path)));
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
            return Ok(Self::HttpStatus(parsed));
        }

        if value == "hello_world" {
            return Ok(Self::HelloWorld);
        }
        if value == "bastion" {
            return Ok(Self::Bastion);
        }
        if value == "socks-proxy" {
            return Ok(Self::SocksProxy);
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
            Ok(Self::Http(url))
        } else {
            Ok(Self::TcpOverWebsocket(url))
        }
    }
}

impl IngressRule {
    pub fn from_raw(raw: RawIngressRule, rule_index: usize, total_rules: usize) -> Result<Self> {
        let service = match raw.service {
            Some(service) => IngressService::parse("service", &service)?,
            None => return Err(ConfigError::invariant("ingress rule is missing service")),
        };

        validate_hostname(
            raw.hostname.as_deref(),
            raw.path.as_deref(),
            rule_index,
            total_rules,
        )?;
        let punycode_hostname = normalized_punycode_hostname(raw.hostname.as_deref())?;

        Ok(Self {
            matcher: IngressMatch {
                hostname: raw.hostname,
                punycode_hostname,
                path: raw.path,
            },
            service,
            origin_request: raw.origin_request,
        })
    }

    pub fn is_catch_all(&self) -> bool {
        self.matcher.hostname.is_none() && self.matcher.path.is_none()
    }

    pub fn matches(&self, hostname: &str, path: &str) -> bool {
        let hostname = strip_port(hostname);
        let host_match = match self.matcher.hostname.as_deref() {
            None | Some("") | Some("*") => true,
            Some(rule_host) => match_host(rule_host, hostname),
        };
        let punycode_match = self
            .matcher
            .punycode_hostname
            .as_deref()
            .is_some_and(|rule_host| match_host(rule_host, hostname));
        let path_match = self
            .matcher
            .path
            .as_deref()
            .is_none_or(|pattern| match_path(pattern, path));

        (host_match || punycode_match) && path_match
    }
}

impl CliIngressRequest {
    pub fn from_flags(flags: &[String]) -> Self {
        let mut request = Self::default();

        for flag in flags {
            if let Some(value) = flag.strip_prefix("--hello-world=") {
                request.hello_world = value == "true";
            } else if flag == "--hello-world" {
                request.hello_world = true;
            } else if let Some(value) = flag.strip_prefix("--bastion=") {
                request.bastion = value == "true";
            } else if flag == "--bastion" {
                request.bastion = true;
            } else if let Some(value) = flag.strip_prefix("--url=") {
                request.url = Some(value.to_owned());
            } else if let Some(value) = flag.strip_prefix("--unix-socket=") {
                request.unix_socket = Some(value.to_owned());
            }
        }

        request
    }
}

impl NormalizedIngress {
    pub fn from_cli_request(request: &CliIngressRequest) -> Result<Self> {
        let defaults = default_single_origin_origin_request(request);
        let service = parse_single_origin_service(request)?;
        Ok(Self {
            rules: vec![IngressRule {
                matcher: IngressMatch::default(),
                service,
                origin_request: defaults.clone(),
            }],
            defaults,
        })
    }

    pub fn find_matching_rule(&self, hostname: &str, path: &str) -> Option<usize> {
        find_matching_rule(&self.rules, hostname, path)
    }
}

pub fn default_no_ingress_rule() -> IngressRule {
    IngressRule {
        matcher: IngressMatch::default(),
        service: IngressService::HttpStatus(503),
        origin_request: OriginRequestConfig::default(),
    }
}

pub fn find_matching_rule(rules: &[IngressRule], hostname: &str, path: &str) -> Option<usize> {
    if rules.is_empty() {
        return None;
    }

    for (index, rule) in rules.iter().enumerate() {
        if rule.matches(hostname, path) {
            return Some(index);
        }
    }

    Some(rules.len() - 1)
}

pub fn parse_cli_ingress(flags: &[String]) -> Result<NormalizedIngress> {
    let request = CliIngressRequest::from_flags(flags);
    NormalizedIngress::from_cli_request(&request)
}

fn parse_single_origin_service(request: &CliIngressRequest) -> Result<IngressService> {
    if request.hello_world {
        return Ok(IngressService::HelloWorld);
    }
    if request.bastion {
        return Ok(IngressService::Bastion);
    }
    if let Some(url) = request.url.as_deref() {
        return parse_cli_origin_url(url);
    }
    if let Some(unix_socket) = request.unix_socket.as_deref() {
        return Ok(IngressService::UnixSocket(PathBuf::from(unix_socket)));
    }

    Err(ConfigError::NoIngressRulesCli)
}

fn parse_cli_origin_url(value: &str) -> Result<IngressService> {
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

    if is_http_scheme(url.scheme()) {
        Ok(IngressService::Http(url))
    } else {
        Ok(IngressService::TcpOverWebsocket(url))
    }
}

fn default_single_origin_origin_request(request: &CliIngressRequest) -> OriginRequestConfig {
    OriginRequestConfig {
        connect_timeout: Some(DurationSpec(DEFAULT_HTTP_CONNECT_TIMEOUT.to_owned())),
        tls_timeout: Some(DurationSpec(DEFAULT_TLS_TIMEOUT.to_owned())),
        tcp_keep_alive: Some(DurationSpec(DEFAULT_TCP_KEEP_ALIVE.to_owned())),
        no_happy_eyeballs: Some(false),
        keep_alive_connections: Some(DEFAULT_KEEP_ALIVE_CONNECTIONS),
        keep_alive_timeout: Some(DurationSpec(DEFAULT_KEEP_ALIVE_TIMEOUT.to_owned())),
        http_host_header: None,
        origin_server_name: None,
        match_sni_to_host: Some(false),
        ca_pool: None,
        no_tls_verify: Some(false),
        disable_chunked_encoding: Some(false),
        bastion_mode: request.bastion.then_some(true),
        proxy_address: Some(DEFAULT_PROXY_ADDRESS.to_owned()),
        proxy_port: None,
        proxy_type: None,
        ip_rules: Vec::new(),
        http2_origin: Some(false),
        access: None,
    }
}

fn validate_hostname(
    hostname: Option<&str>,
    path: Option<&str>,
    rule_index: usize,
    total_rules: usize,
) -> Result<()> {
    let hostname = hostname.unwrap_or_default();
    let path = path.unwrap_or_default();

    if hostname.contains(':') {
        return Err(ConfigError::IngressHostnameContainsPort);
    }
    if hostname.rfind('*').is_some_and(|index| index > 0) {
        return Err(ConfigError::IngressBadWildcard);
    }

    let is_catch_all = (hostname.is_empty() || hostname == "*") && path.is_empty();
    let is_last_rule = rule_index + 1 == total_rules;
    if is_last_rule && !is_catch_all {
        return Err(ConfigError::IngressLastRuleNotCatchAll);
    }
    if !is_last_rule && is_catch_all {
        return Err(ConfigError::IngressCatchAllNotLast {
            index: rule_index + 1,
            hostname: hostname.to_owned(),
        });
    }

    Ok(())
}

fn normalized_punycode_hostname(hostname: Option<&str>) -> Result<Option<String>> {
    let Some(hostname) = hostname else {
        return Ok(None);
    };
    if hostname.is_empty() || hostname == "*" || hostname.contains('*') {
        return Ok(None);
    }

    let url = Url::parse(&format!("https://{hostname}"))
        .map_err(|source| ConfigError::invalid_url("hostname", hostname, source))?;
    let Some(punycode) = url.host_str() else {
        return Ok(None);
    };
    if punycode == hostname {
        Ok(None)
    } else {
        Ok(Some(punycode.to_owned()))
    }
}

fn is_http_scheme(scheme: &str) -> bool {
    matches!(scheme, "http" | "https" | "ws" | "wss")
}

fn match_host(rule_host: &str, req_host: &str) -> bool {
    if rule_host == req_host {
        return true;
    }

    if let Some(suffix) = rule_host.strip_prefix("*.") {
        let suffix = format!(".{suffix}");
        return req_host.ends_with(&suffix);
    }

    false
}

fn match_path(pattern: &str, path: &str) -> bool {
    path.contains(pattern)
}

fn strip_port(hostname: &str) -> &str {
    if hostname.starts_with('[') {
        return hostname;
    }

    if let Some((host, port)) = hostname.rsplit_once(':')
        && !host.contains(':')
        && !host.is_empty()
        && !port.is_empty()
        && port.chars().all(|ch| ch.is_ascii_digit())
    {
        return host;
    }

    hostname
}

#[cfg(test)]
mod tests {
    use super::{
        CliIngressRequest, IngressRule, IngressService, NormalizedIngress, RawIngressRule,
        default_no_ingress_rule, find_matching_rule, parse_cli_ingress,
    };
    use crate::error::ConfigError;

    fn ok<T, E: std::fmt::Display>(result: std::result::Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("unexpected error: {error}"),
        }
    }

    #[test]
    fn service_parser_recognizes_http_services() {
        let service = ok(IngressService::parse("service", "https://localhost:8080"));

        match service {
            IngressService::Http(url) => assert_eq!(url.scheme(), "https"),
            other => panic!("expected HTTP service, found {other:?}"),
        }
    }

    #[test]
    fn service_parser_recognizes_tcp_over_websocket_services() {
        let service = ok(IngressService::parse("service", "tcp://localhost:8080"));

        match service {
            IngressService::TcpOverWebsocket(url) => assert_eq!(url.scheme(), "tcp"),
            other => panic!("expected TCP-over-websocket service, found {other:?}"),
        }
    }

    #[test]
    fn raw_rule_without_hostname_or_path_is_catch_all() {
        let rule = ok(IngressRule::from_raw(
            RawIngressRule {
                service: Some("https://localhost:8080".to_owned()),
                ..RawIngressRule::default()
            },
            0,
            1,
        ));

        assert!(rule.is_catch_all());
    }

    #[test]
    fn wildcard_not_at_start_is_rejected() {
        let error = IngressRule::from_raw(
            RawIngressRule {
                hostname: Some("test.*.example.com".to_owned()),
                service: Some("https://localhost:8080".to_owned()),
                ..RawIngressRule::default()
            },
            0,
            1,
        )
        .expect_err("wildcard should be rejected");

        assert!(matches!(error, ConfigError::IngressBadWildcard));
    }

    #[test]
    fn no_ingress_default_rule_is_http_503() {
        assert_eq!(default_no_ingress_rule().service, IngressService::HttpStatus(503));
    }

    #[test]
    fn unicode_hostname_captures_punycode() {
        let rule = ok(IngressRule::from_raw(
            RawIngressRule {
                hostname: Some("môô.cloudflare.com".to_owned()),
                service: Some("https://localhost:8080".to_owned()),
                ..RawIngressRule::default()
            },
            0,
            2,
        ));

        assert_eq!(
            rule.matcher.punycode_hostname.as_deref(),
            Some("xn--m-xgaa.cloudflare.com")
        );
    }

    #[test]
    fn matching_prefers_first_matching_rule_and_strips_port() {
        let rules = vec![
            ok(IngressRule::from_raw(
                RawIngressRule {
                    hostname: Some("tunnel-a.example.com".to_owned()),
                    service: Some("https://localhost:8080".to_owned()),
                    ..RawIngressRule::default()
                },
                0,
                3,
            )),
            ok(IngressRule::from_raw(
                RawIngressRule {
                    hostname: Some("tunnel-b.example.com".to_owned()),
                    path: Some("/health".to_owned()),
                    service: Some("https://localhost:8081".to_owned()),
                    ..RawIngressRule::default()
                },
                1,
                3,
            )),
            ok(IngressRule::from_raw(
                RawIngressRule {
                    service: Some("http_status:404".to_owned()),
                    ..RawIngressRule::default()
                },
                2,
                3,
            )),
        ];

        assert_eq!(
            find_matching_rule(&rules, "tunnel-a.example.com:443", "/"),
            Some(0)
        );
        assert_eq!(
            find_matching_rule(&rules, "tunnel-b.example.com", "/health"),
            Some(1)
        );
        assert_eq!(
            find_matching_rule(&rules, "tunnel-b.example.com", "/index.html"),
            Some(2)
        );
        assert_eq!(find_matching_rule(&rules, "unknown.example.com", "/"), Some(2));
    }

    #[test]
    fn unicode_rule_matches_punycode_hostname() {
        let rules = vec![
            ok(IngressRule::from_raw(
                RawIngressRule {
                    hostname: Some("môô.cloudflare.com".to_owned()),
                    service: Some("https://localhost:8080".to_owned()),
                    ..RawIngressRule::default()
                },
                0,
                2,
            )),
            ok(IngressRule::from_raw(
                RawIngressRule {
                    service: Some("http_status:404".to_owned()),
                    ..RawIngressRule::default()
                },
                1,
                2,
            )),
        ];

        assert_eq!(
            find_matching_rule(&rules, "xn--m-xgaa.cloudflare.com", "/"),
            Some(0)
        );
    }

    #[test]
    fn cli_request_parses_flags() {
        let request = CliIngressRequest::from_flags(&[
            "--url=http://localhost:8080".to_owned(),
            "--hello-world=false".to_owned(),
        ]);

        assert_eq!(request.url.as_deref(), Some("http://localhost:8080"));
        assert!(!request.hello_world);
    }

    #[test]
    fn cli_ingress_hello_world_normalizes() {
        let ingress = ok(parse_cli_ingress(&["--hello-world=true".to_owned()]));

        assert_eq!(ingress.rules.len(), 1);
        assert_eq!(ingress.rules[0].service, IngressService::HelloWorld);
        assert_eq!(
            ingress
                .defaults
                .connect_timeout
                .as_ref()
                .map(|value| value.0.as_str()),
            Some("30s")
        );
    }

    #[test]
    fn cli_ingress_bastion_sets_bastion_mode() {
        let ingress = ok(NormalizedIngress::from_cli_request(&CliIngressRequest {
            bastion: true,
            ..CliIngressRequest::default()
        }));

        assert_eq!(ingress.rules[0].service, IngressService::Bastion);
        assert_eq!(ingress.defaults.bastion_mode, Some(true));
    }

    #[test]
    fn cli_ingress_without_origin_is_an_error() {
        let error = parse_cli_ingress(&[]).expect_err("missing origin should fail");

        assert!(matches!(error, ConfigError::NoIngressRulesCli));
        assert_eq!(error.category(), "no-ingress-rules-cli");
    }
}
