use super::{
    DEFAULT_HTTP_CONNECT_TIMEOUT, DEFAULT_KEEP_ALIVE_CONNECTIONS, DEFAULT_KEEP_ALIVE_TIMEOUT,
    DEFAULT_PROXY_ADDRESS, DEFAULT_TCP_KEEP_ALIVE, DEFAULT_TLS_TIMEOUT, DurationSpec, IngressFlagRequest,
    OriginRequestConfig,
};

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

pub(super) fn merge_overrides(
    base: &OriginRequestConfig,
    overrides: &OriginRequestConfig,
) -> OriginRequestConfig {
    let mut merged = base.clone();

    merged.connect_timeout = merge_optional(merged.connect_timeout, overrides.connect_timeout.clone());
    merged.tls_timeout = merge_optional(merged.tls_timeout, overrides.tls_timeout.clone());
    merged.tcp_keep_alive = merge_optional(merged.tcp_keep_alive, overrides.tcp_keep_alive.clone());
    merged.no_happy_eyeballs = merge_optional(merged.no_happy_eyeballs, overrides.no_happy_eyeballs);
    merged.keep_alive_connections =
        merge_optional(merged.keep_alive_connections, overrides.keep_alive_connections);
    merged.keep_alive_timeout =
        merge_optional(merged.keep_alive_timeout, overrides.keep_alive_timeout.clone());
    merged.http_host_header = merge_optional(merged.http_host_header, overrides.http_host_header.clone());
    merged.origin_server_name =
        merge_optional(merged.origin_server_name, overrides.origin_server_name.clone());
    merged.match_sni_to_host = merge_optional(merged.match_sni_to_host, overrides.match_sni_to_host);
    merged.ca_pool = merge_optional(merged.ca_pool, overrides.ca_pool.clone());
    merged.no_tls_verify = merge_optional(merged.no_tls_verify, overrides.no_tls_verify);
    merged.disable_chunked_encoding = merge_optional(
        merged.disable_chunked_encoding,
        overrides.disable_chunked_encoding,
    );
    merged.bastion_mode = merge_optional(merged.bastion_mode, overrides.bastion_mode);
    merged.proxy_address = merge_optional(merged.proxy_address, overrides.proxy_address.clone());
    merged.proxy_port = merge_optional(merged.proxy_port, overrides.proxy_port);
    merged.proxy_type = merge_optional(merged.proxy_type, overrides.proxy_type.clone());
    merged.http2_origin = merge_optional(merged.http2_origin, overrides.http2_origin);
    merged.access = merge_optional(merged.access, overrides.access.clone());

    if !overrides.ip_rules.is_empty() {
        merged.ip_rules = overrides.ip_rules.clone();
    }

    merged
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

fn merge_optional<T>(current: Option<T>, override_value: Option<T>) -> Option<T> {
    override_value.or(current)
}

fn default_http_connect_timeout() -> Option<DurationSpec> {
    Some(DurationSpec(DEFAULT_HTTP_CONNECT_TIMEOUT.to_owned()))
}

fn default_tls_timeout() -> Option<DurationSpec> {
    Some(DurationSpec(DEFAULT_TLS_TIMEOUT.to_owned()))
}

fn default_tcp_keep_alive() -> Option<DurationSpec> {
    Some(DurationSpec(DEFAULT_TCP_KEEP_ALIVE.to_owned()))
}

fn default_keep_alive_timeout() -> Option<DurationSpec> {
    Some(DurationSpec(DEFAULT_KEEP_ALIVE_TIMEOUT.to_owned()))
}

fn default_proxy_address() -> Option<String> {
    Some(DEFAULT_PROXY_ADDRESS.to_owned())
}
