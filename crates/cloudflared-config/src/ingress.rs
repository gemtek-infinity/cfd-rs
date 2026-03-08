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
    HttpStatus(u16),
    HelloWorld,
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

        match Url::parse(value) {
            Ok(url) => {
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
                Ok(Self::Uri(url))
            }
            Err(url::ParseError::RelativeUrlWithoutBase) => Ok(Self::NamedToken(value.to_owned())),
            Err(source) => Err(ConfigError::invalid_url(field, value, source)),
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
}

pub fn default_no_ingress_rule() -> IngressRule {
    IngressRule {
        matcher: IngressMatch::default(),
        service: IngressService::HttpStatus(503),
        origin_request: OriginRequestConfig::default(),
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

#[cfg(test)]
mod tests {
    use super::{IngressRule, IngressService, RawIngressRule, default_no_ingress_rule};
    use crate::error::ConfigError;

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
}
