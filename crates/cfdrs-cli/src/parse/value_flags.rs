use super::{FlagMatcher, ParseState, surface_contract};
use std::ffi::{OsStr, OsString};

pub(super) fn try_parse_value_flag(
    arg: &OsStr,
    args: &mut impl Iterator<Item = OsString>,
    state: &mut ParseState,
) -> Result<bool, String> {
    let matcher = &mut FlagMatcher::new(arg, args, &mut state.flags);

    try_config_and_credential_flags(matcher)?;
    try_logging_flags(matcher)?;
    try_metrics_and_process_flags(matcher)?;
    try_tunnel_identity_flags(matcher)?;
    try_connection_tuning_flags(matcher)?;
    try_management_flags(matcher)?;
    try_origin_and_proxy_flags(matcher)?;
    try_quic_flow_control_flags(matcher)?;
    try_deprecated_api_flags(matcher)?;

    if matcher.matched() {
        state.any_flag_set = true;
    }

    Ok(matcher.matched())
}

fn try_config_and_credential_flags<I: Iterator<Item = OsString>>(
    matcher: &mut FlagMatcher<'_, I>,
) -> Result<(), String> {
    matcher
        .path(surface_contract::CONFIG_FLAG, |flags| &mut flags.config_path)?
        .path_alias("--credentials-file", "--cred-file", |flags| {
            &mut flags.credentials_file
        })?
        .string("--credentials-contents", |flags| &mut flags.credentials_contents)?
        .string("--token", |flags| &mut flags.token)?
        .path("--token-file", |flags| &mut flags.token_file)?
        .path("--origincert", |flags| &mut flags.origincert)?;

    Ok(())
}

fn try_logging_flags<I: Iterator<Item = OsString>>(matcher: &mut FlagMatcher<'_, I>) -> Result<(), String> {
    matcher
        .string("--loglevel", |flags| &mut flags.loglevel)?
        .string("--transport-loglevel", |flags| &mut flags.transport_loglevel)?
        .path("--logfile", |flags| &mut flags.logfile)?
        .path("--log-directory", |flags| &mut flags.log_directory)?
        .string("--output", |flags| &mut flags.log_format_output)?;

    Ok(())
}

fn try_metrics_and_process_flags<I: Iterator<Item = OsString>>(
    matcher: &mut FlagMatcher<'_, I>,
) -> Result<(), String> {
    matcher
        .string("--metrics", |flags| &mut flags.metrics)?
        .path("--pidfile", |flags| &mut flags.pidfile)?
        .string("--grace-period", |flags| &mut flags.grace_period)?;

    Ok(())
}

fn try_tunnel_identity_flags<I: Iterator<Item = OsString>>(
    matcher: &mut FlagMatcher<'_, I>,
) -> Result<(), String> {
    matcher
        .string("--url", |flags| &mut flags.url)?
        .string_alias("--name", "-n", |flags| &mut flags.tunnel_name)?
        .string_alias("--protocol", "-p", |flags| &mut flags.protocol)?
        .push("--edge", |flags| &mut flags.edge)?
        .string("--region", |flags| &mut flags.region)?
        .string("--edge-ip-version", |flags| &mut flags.edge_ip_version)?
        .string("--edge-bind-address", |flags| &mut flags.edge_bind_address)?
        .string("--hostname", |flags| &mut flags.hostname)?
        .string("--id", |flags| &mut flags.tunnel_id)?
        .string("--lb-pool", |flags| &mut flags.lb_pool)?
        .push("--tag", |flags| &mut flags.tag)?
        .push_alias("--features", "-F", |flags| &mut flags.features)?
        .string("--label", |flags| &mut flags.label)?;

    Ok(())
}

fn try_connection_tuning_flags<I: Iterator<Item = OsString>>(
    matcher: &mut FlagMatcher<'_, I>,
) -> Result<(), String> {
    matcher
        .string("--autoupdate-freq", |flags| &mut flags.autoupdate_freq)?
        .string("--metrics-update-freq", |flags| &mut flags.metrics_update_freq)?
        .u32_val("--retries", |flags| &mut flags.retries)?
        .u32_val("--ha-connections", |flags| &mut flags.ha_connections)?
        .u32_val("--max-edge-addr-retries", |flags| {
            &mut flags.max_edge_addr_retries
        })?
        .string("--rpc-timeout", |flags| &mut flags.rpc_timeout)?
        .string("--heartbeat-interval", |flags| &mut flags.heartbeat_interval)?
        .u32_val("--heartbeat-count", |flags| &mut flags.heartbeat_count)?
        .string("--write-stream-timeout", |flags| &mut flags.write_stream_timeout)?
        .u64_val("--max-active-flows", |flags| &mut flags.max_active_flows)?;

    Ok(())
}

fn try_management_flags<I: Iterator<Item = OsString>>(
    matcher: &mut FlagMatcher<'_, I>,
) -> Result<(), String> {
    matcher
        .string("--management-hostname", |flags| &mut flags.management_hostname)?
        .string("--api-url", |flags| &mut flags.api_url)?
        .string("--trace-output", |flags| &mut flags.trace_output)?;

    Ok(())
}

fn try_origin_and_proxy_flags<I: Iterator<Item = OsString>>(
    matcher: &mut FlagMatcher<'_, I>,
) -> Result<(), String> {
    matcher
        .string("--unix-socket", |flags| &mut flags.unix_socket)?
        .string("--http-host-header", |flags| &mut flags.http_host_header)?
        .string("--origin-server-name", |flags| &mut flags.origin_server_name)?
        .string_alias("--origin-ca-pool", "--cacert", |flags| &mut flags.origin_ca_pool)?
        .string("--icmpv4-src", |flags| &mut flags.icmpv4_src)?
        .string("--icmpv6-src", |flags| &mut flags.icmpv6_src)?
        .string("--proxy-address", |flags| &mut flags.proxy_address)?
        .u16_val("--proxy-port", |flags| &mut flags.proxy_port)?
        .string("--proxy-connect-timeout", |flags| {
            &mut flags.proxy_connect_timeout
        })?
        .string("--proxy-tls-timeout", |flags| &mut flags.proxy_tls_timeout)?
        .string("--proxy-tcp-keepalive", |flags| &mut flags.proxy_tcp_keepalive)?
        .u32_val("--proxy-keepalive-connections", |flags| {
            &mut flags.proxy_keepalive_connections
        })?
        .string("--proxy-keepalive-timeout", |flags| {
            &mut flags.proxy_keepalive_timeout
        })?
        .string("--service-op-ip", |flags| &mut flags.service_op_ip)?;

    Ok(())
}

fn try_quic_flow_control_flags<I: Iterator<Item = OsString>>(
    matcher: &mut FlagMatcher<'_, I>,
) -> Result<(), String> {
    matcher
        .u64_val("--quic-connection-level-flow-control-limit", |flags| {
            &mut flags.quic_conn_flow_control
        })?
        .u64_val("--quic-stream-level-flow-control-limit", |flags| {
            &mut flags.quic_stream_flow_control
        })?;

    Ok(())
}

fn try_deprecated_api_flags<I: Iterator<Item = OsString>>(
    matcher: &mut FlagMatcher<'_, I>,
) -> Result<(), String> {
    matcher
        .string("--api-key", |flags| &mut flags.api_key)?
        .string("--api-email", |flags| &mut flags.api_email)?
        .string("--api-ca-key", |flags| &mut flags.api_ca_key)?;

    Ok(())
}
