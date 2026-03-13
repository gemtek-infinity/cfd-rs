use super::super::OriginRequestConfig;

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

fn merge_optional<T>(current: Option<T>, override_value: Option<T>) -> Option<T> {
    override_value.or(current)
}
