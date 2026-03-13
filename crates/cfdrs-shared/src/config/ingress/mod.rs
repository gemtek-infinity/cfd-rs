use crate::config::error::Result;

mod flag_surface;
mod matching;
mod origin_request;
mod service_parser;
mod types;
mod validation;

pub use self::types::{
    AccessConfig, DurationSpec, IngressFlagRequest, IngressIpRule, IngressMatch, IngressRule, IngressService,
    NormalizedIngress, OriginRequestConfig, OriginRequestConfigBuilder, ProxyType, RawIngressRule,
};

pub const NO_INGRESS_RULES_FLAGS_MESSAGE: &str = concat!(
    "No ingress rules were defined in provided config (if any) nor from the provided flags, ",
    "cloudflared will return 503 for all incoming HTTP requests"
);

const DEFAULT_HTTP_CONNECT_TIMEOUT: &str = "30s";
const DEFAULT_TLS_TIMEOUT: &str = "10s";
const DEFAULT_TCP_KEEP_ALIVE: &str = "30s";
const DEFAULT_KEEP_ALIVE_TIMEOUT: &str = "1m30s";
const DEFAULT_PROXY_ADDRESS: &str = "127.0.0.1";
const DEFAULT_KEEP_ALIVE_CONNECTIONS: u32 = 100;

pub fn default_no_ingress_rule() -> IngressRule {
    IngressRule {
        matcher: IngressMatch::default(),
        service: IngressService::HttpStatus(503),
        origin_request: OriginRequestConfig::default(),
    }
}

pub fn find_matching_rule(rules: &[IngressRule], hostname: &str, path: &str) -> Option<usize> {
    self::matching::find_matching_rule(rules, hostname, path)
}

pub fn parse_ingress_flags(flags: &[String]) -> Result<NormalizedIngress> {
    self::flag_surface::parse_ingress_flags(flags)
}

#[cfg(test)]
mod tests;
