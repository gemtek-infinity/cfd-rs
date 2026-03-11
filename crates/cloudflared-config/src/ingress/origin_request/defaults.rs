mod value_defaults;

use self::value_defaults::{
    default_http_connect_timeout, default_keep_alive_timeout, default_proxy_address, default_tcp_keep_alive,
    default_tls_timeout,
};
use super::super::{DEFAULT_KEEP_ALIVE_CONNECTIONS, IngressFlagRequest, OriginRequestConfig};

pub(super) fn materialize_defaults(raw: &OriginRequestConfig) -> OriginRequestConfig {
    OriginRequestConfig {
        connect_timeout: raw.connect_timeout.clone().or_else(default_http_connect_timeout),
        tls_timeout: raw.tls_timeout.clone().or_else(default_tls_timeout),
        tcp_keep_alive: raw.tcp_keep_alive.clone().or_else(default_tcp_keep_alive),
        no_happy_eyeballs: Some(raw.no_happy_eyeballs.unwrap_or(false)),
        keep_alive_connections: Some(
            raw.keep_alive_connections
                .unwrap_or(DEFAULT_KEEP_ALIVE_CONNECTIONS),
        ),
        keep_alive_timeout: raw.keep_alive_timeout.clone().or_else(default_keep_alive_timeout),
        http_host_header: raw.http_host_header.clone(),
        origin_server_name: raw.origin_server_name.clone(),
        match_sni_to_host: Some(raw.match_sni_to_host.unwrap_or(false)),
        ca_pool: raw.ca_pool.clone(),
        no_tls_verify: Some(raw.no_tls_verify.unwrap_or(false)),
        disable_chunked_encoding: Some(raw.disable_chunked_encoding.unwrap_or(false)),
        bastion_mode: Some(raw.bastion_mode.unwrap_or(false)),
        proxy_address: raw.proxy_address.clone().or_else(default_proxy_address),
        proxy_port: Some(raw.proxy_port.unwrap_or(0)),
        proxy_type: raw.proxy_type.clone(),
        ip_rules: raw.ip_rules.clone(),
        http2_origin: Some(raw.http2_origin.unwrap_or(false)),
        access: raw.access.clone(),
    }
}

pub(super) fn flag_defaults(request: &IngressFlagRequest) -> OriginRequestConfig {
    OriginRequestConfig {
        connect_timeout: default_http_connect_timeout(),
        tls_timeout: default_tls_timeout(),
        tcp_keep_alive: default_tcp_keep_alive(),
        no_happy_eyeballs: Some(false),
        keep_alive_connections: Some(DEFAULT_KEEP_ALIVE_CONNECTIONS),
        keep_alive_timeout: default_keep_alive_timeout(),
        http_host_header: None,
        origin_server_name: None,
        match_sni_to_host: Some(false),
        ca_pool: None,
        no_tls_verify: Some(false),
        disable_chunked_encoding: Some(false),
        bastion_mode: Some(request.bastion),
        proxy_address: default_proxy_address(),
        proxy_port: Some(0),
        proxy_type: None,
        ip_rules: Vec::new(),
        http2_origin: Some(false),
        access: None,
    }
}
