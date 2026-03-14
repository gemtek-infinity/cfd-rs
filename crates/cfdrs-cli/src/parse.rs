use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use super::types::{GlobalFlags, TunnelSubcommand};
use super::{Cli, Command, surface_contract};

#[derive(Default)]
struct ParseState {
    flags: GlobalFlags,
    command: Option<Command>,
    help_requested: bool,
    version_requested: bool,
    short_version: bool,
    any_flag_set: bool,
    /// After a top-level command word is consumed, remaining positional
    /// args for subcommand parsing land here before being resolved.
    awaiting_subcommand: Option<SubcommandContext>,
}

/// Tracks which top-level command was seen so subsequent positional
/// args can be resolved as subcommands.
#[derive(Debug)]
enum SubcommandContext {
    Tunnel,
    Service,
    Access,
    Tail,
    Management,
    Route,
    RouteIp,
    Vnet,
    Ingress,
}

pub fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Cli, String> {
    let mut args = args.into_iter();
    let _ = args.next();

    let mut state = ParseState::default();

    while let Some(arg) = args.next() {
        handle_argument(arg, &mut args, &mut state)?;
    }

    Ok(finalize_cli(state))
}

fn handle_argument(
    arg: OsString,
    args: &mut impl Iterator<Item = OsString>,
    state: &mut ParseState,
) -> Result<(), String> {
    let token = arg.to_string_lossy();
    let token_str = token.as_ref();

    // Help and version flags always take priority.
    if surface_contract::is_help_token(token_str) {
        state.help_requested = true;
        return Ok(());
    }

    if surface_contract::is_version_token(token_str) {
        state.version_requested = true;
        return Ok(());
    }

    if surface_contract::is_short_version_token(token_str) {
        state.short_version = true;
        return Ok(());
    }

    // Try known flags.
    if try_parse_flag(arg.as_os_str(), args, state)? {
        return Ok(());
    }

    // If we're already inside a command that expects subcommands,
    // try to resolve this token as a subcommand.
    if let Some(ctx) = &state.awaiting_subcommand {
        match ctx {
            SubcommandContext::Tunnel => {
                if let Some(sub) = surface_contract::parse_tunnel_subcommand(token_str) {
                    // Subcommands with their own sub-subcommands enter deeper parsing.
                    match &sub {
                        TunnelSubcommand::Route(_) => {
                            state.awaiting_subcommand = Some(SubcommandContext::Route);
                        }
                        TunnelSubcommand::Vnet(_) => {
                            state.awaiting_subcommand = Some(SubcommandContext::Vnet);
                        }
                        TunnelSubcommand::Ingress(_) => {
                            state.awaiting_subcommand = Some(SubcommandContext::Ingress);
                        }
                        _ => {
                            state.awaiting_subcommand = None;
                        }
                    }
                    state.command = Some(Command::Tunnel(sub));
                    return Ok(());
                }
            }

            SubcommandContext::Service => {
                if let Some(action) = surface_contract::parse_service_subcommand(token_str) {
                    state.command = Some(Command::Service(action));
                    state.awaiting_subcommand = None;
                    return Ok(());
                }
            }

            SubcommandContext::Access => {
                if let Some(sub) = surface_contract::parse_access_subcommand(token_str) {
                    state.command = Some(Command::Access(sub));
                    state.awaiting_subcommand = None;
                    return Ok(());
                }
            }

            SubcommandContext::Tail => {
                if let Some(sub) = surface_contract::parse_tail_subcommand(token_str) {
                    state.command = Some(Command::Tail(sub));
                    state.awaiting_subcommand = None;
                    return Ok(());
                }
            }

            SubcommandContext::Management => {
                if let Some(sub) = surface_contract::parse_management_subcommand(token_str) {
                    state.command = Some(Command::Management(sub));
                    state.awaiting_subcommand = None;
                    return Ok(());
                }
            }

            SubcommandContext::Route => {
                if let Some(sub) = surface_contract::parse_route_subcommand(token_str) {
                    match &sub {
                        super::types::RouteSubcommand::Ip(_) => {
                            state.awaiting_subcommand = Some(SubcommandContext::RouteIp);
                        }
                        _ => {
                            state.awaiting_subcommand = None;
                        }
                    }
                    state.command = Some(Command::Tunnel(TunnelSubcommand::Route(sub)));
                    return Ok(());
                }
            }

            SubcommandContext::RouteIp => {
                if let Some(sub) = surface_contract::parse_ip_route_subcommand(token_str) {
                    state.command = Some(Command::Tunnel(TunnelSubcommand::Route(
                        super::types::RouteSubcommand::Ip(sub),
                    )));
                    state.awaiting_subcommand = None;
                    return Ok(());
                }
            }

            SubcommandContext::Vnet => {
                if let Some(sub) = surface_contract::parse_vnet_subcommand(token_str) {
                    state.command = Some(Command::Tunnel(TunnelSubcommand::Vnet(sub)));
                    state.awaiting_subcommand = None;
                    return Ok(());
                }
            }

            SubcommandContext::Ingress => {
                if let Some(sub) = surface_contract::parse_ingress_subcommand(token_str) {
                    state.command = Some(Command::Tunnel(TunnelSubcommand::Ingress(sub)));
                    state.awaiting_subcommand = None;
                    return Ok(());
                }
            }
        }

        // Not a known subcommand — collect as rest arg.
        state.flags.rest_args.push(token_str.to_owned());
        return Ok(());
    }

    // Try top-level command word.
    if let Some(command) = surface_contract::parse_command_token(token_str) {
        // For commands with subcommands, enter subcommand parsing mode.
        match &command {
            Command::Tunnel(TunnelSubcommand::Bare) => {
                state.awaiting_subcommand = Some(SubcommandContext::Tunnel);
                state.command = Some(command);
            }

            Command::Service(_) => {
                state.awaiting_subcommand = Some(SubcommandContext::Service);
                state.command = Some(command);
            }

            Command::Access(_) => {
                state.awaiting_subcommand = Some(SubcommandContext::Access);
                state.command = Some(command);
            }

            Command::Tail(_) => {
                state.awaiting_subcommand = Some(SubcommandContext::Tail);
                state.command = Some(command);
            }

            Command::Management(_) => {
                state.awaiting_subcommand = Some(SubcommandContext::Management);
                state.command = Some(command);
            }

            _ => {
                set_command(&mut state.command, command)?;
            }
        }

        return Ok(());
    }

    // After a command has been set, collect unknown args for forward
    // compatibility with subcommand-level flags we have not yet parsed.
    if state.command.is_some() {
        state.flags.rest_args.push(token_str.to_owned());
        return Ok(());
    }

    // At the top level with no command word set, unknown flags and
    // positional args are errors — matching Go urfave/cli behavior.
    if token_str.starts_with('-') {
        return Err(surface_contract::unknown_flag_message(token_str));
    }

    Err(surface_contract::unknown_argument_message(token_str))
}

/// Try to parse the argument as a known flag.
/// Returns `true` if the argument was consumed as a flag.
fn try_parse_flag(
    arg: &OsStr,
    args: &mut impl Iterator<Item = OsString>,
    state: &mut ParseState,
) -> Result<bool, String> {
    // --config VALUE or --config=VALUE
    if let Some(value) = try_string_flag(arg, args, surface_contract::CONFIG_FLAG)? {
        set_path_flag(&mut state.flags.config_path, value, surface_contract::CONFIG_FLAG)?;
        state.any_flag_set = true;
        return Ok(true);
    }

    // --credentials-file VALUE or --cred-file VALUE
    if let Some(value) =
        try_string_flag(arg, args, "--credentials-file")?.or(try_string_flag(arg, args, "--cred-file")?)
    {
        set_path_flag(&mut state.flags.credentials_file, value, "--credentials-file")?;
        state.any_flag_set = true;
        return Ok(true);
    }

    // --credentials-contents VALUE
    if let Some(value) = try_string_flag(arg, args, "--credentials-contents")? {
        state.flags.credentials_contents = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --token VALUE
    if let Some(value) = try_string_flag(arg, args, "--token")? {
        state.flags.token = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --token-file VALUE
    if let Some(value) = try_string_flag(arg, args, "--token-file")? {
        set_path_flag(&mut state.flags.token_file, value, "--token-file")?;
        state.any_flag_set = true;
        return Ok(true);
    }

    // --origincert VALUE
    if let Some(value) = try_string_flag(arg, args, "--origincert")? {
        set_path_flag(&mut state.flags.origincert, value, "--origincert")?;
        state.any_flag_set = true;
        return Ok(true);
    }

    // --loglevel VALUE
    if let Some(value) = try_string_flag(arg, args, "--loglevel")? {
        state.flags.loglevel = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --transport-loglevel VALUE
    if let Some(value) = try_string_flag(arg, args, "--transport-loglevel")? {
        state.flags.transport_loglevel = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --logfile VALUE
    if let Some(value) = try_string_flag(arg, args, "--logfile")? {
        set_path_flag(&mut state.flags.logfile, value, "--logfile")?;
        state.any_flag_set = true;
        return Ok(true);
    }

    // --log-directory VALUE
    if let Some(value) = try_string_flag(arg, args, "--log-directory")? {
        set_path_flag(&mut state.flags.log_directory, value, "--log-directory")?;
        state.any_flag_set = true;
        return Ok(true);
    }

    // --output VALUE (log format)
    if let Some(value) = try_string_flag(arg, args, "--output")? {
        state.flags.log_format_output = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --metrics VALUE
    if let Some(value) = try_string_flag(arg, args, "--metrics")? {
        state.flags.metrics = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --pidfile VALUE
    if let Some(value) = try_string_flag(arg, args, "--pidfile")? {
        set_path_flag(&mut state.flags.pidfile, value, "--pidfile")?;
        state.any_flag_set = true;
        return Ok(true);
    }

    // --grace-period VALUE
    if let Some(value) = try_string_flag(arg, args, "--grace-period")? {
        state.flags.grace_period = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --url VALUE
    if let Some(value) = try_string_flag(arg, args, "--url")? {
        state.flags.url = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --name or -n VALUE
    if let Some(value) = try_string_flag(arg, args, "--name")?.or(try_string_flag(arg, args, "-n")?) {
        state.flags.tunnel_name = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --protocol or -p VALUE
    if let Some(value) = try_string_flag(arg, args, "--protocol")?.or(try_string_flag(arg, args, "-p")?) {
        state.flags.protocol = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --edge VALUE (hidden, repeated)
    if let Some(value) = try_string_flag(arg, args, "--edge")? {
        state.flags.edge.push(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --region VALUE
    if let Some(value) = try_string_flag(arg, args, "--region")? {
        state.flags.region = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --edge-ip-version VALUE
    if let Some(value) = try_string_flag(arg, args, "--edge-ip-version")? {
        state.flags.edge_ip_version = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --edge-bind-address VALUE
    if let Some(value) = try_string_flag(arg, args, "--edge-bind-address")? {
        state.flags.edge_bind_address = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --hostname VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--hostname")? {
        state.flags.hostname = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --id VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--id")? {
        state.flags.tunnel_id = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --lb-pool VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--lb-pool")? {
        state.flags.lb_pool = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --tag VALUE (hidden, repeated)
    if let Some(value) = try_string_flag(arg, args, "--tag")? {
        state.flags.tag.push(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --features or -F VALUE (repeated)
    if let Some(value) = try_string_flag(arg, args, "--features")?.or(try_string_flag(arg, args, "-F")?) {
        state.flags.features.push(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --label VALUE
    if let Some(value) = try_string_flag(arg, args, "--label")? {
        state.flags.label = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --autoupdate-freq VALUE
    if let Some(value) = try_string_flag(arg, args, "--autoupdate-freq")? {
        state.flags.autoupdate_freq = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --metrics-update-freq VALUE
    if let Some(value) = try_string_flag(arg, args, "--metrics-update-freq")? {
        state.flags.metrics_update_freq = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --retries VALUE
    if let Some(value) = try_string_flag(arg, args, "--retries")? {
        state.flags.retries = Some(parse_u32(&value, "--retries")?);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --ha-connections VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--ha-connections")? {
        state.flags.ha_connections = Some(parse_u32(&value, "--ha-connections")?);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --max-edge-addr-retries VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--max-edge-addr-retries")? {
        state.flags.max_edge_addr_retries = Some(parse_u32(&value, "--max-edge-addr-retries")?);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --rpc-timeout VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--rpc-timeout")? {
        state.flags.rpc_timeout = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --heartbeat-interval VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--heartbeat-interval")? {
        state.flags.heartbeat_interval = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --heartbeat-count VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--heartbeat-count")? {
        state.flags.heartbeat_count = Some(parse_u32(&value, "--heartbeat-count")?);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --write-stream-timeout VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--write-stream-timeout")? {
        state.flags.write_stream_timeout = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --max-active-flows VALUE
    if let Some(value) = try_string_flag(arg, args, "--max-active-flows")? {
        state.flags.max_active_flows = Some(parse_u64(&value, "--max-active-flows")?);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --management-hostname VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--management-hostname")? {
        state.flags.management_hostname = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --api-url VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--api-url")? {
        state.flags.api_url = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --trace-output VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--trace-output")? {
        state.flags.trace_output = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --unix-socket VALUE
    if let Some(value) = try_string_flag(arg, args, "--unix-socket")? {
        state.flags.unix_socket = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --http-host-header VALUE
    if let Some(value) = try_string_flag(arg, args, "--http-host-header")? {
        state.flags.http_host_header = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --origin-server-name VALUE
    if let Some(value) = try_string_flag(arg, args, "--origin-server-name")? {
        state.flags.origin_server_name = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --origin-ca-pool / --cacert VALUE (hidden)
    if let Some(value) =
        try_string_flag(arg, args, "--origin-ca-pool")?.or(try_string_flag(arg, args, "--cacert")?)
    {
        state.flags.origin_ca_pool = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --icmpv4-src VALUE
    if let Some(value) = try_string_flag(arg, args, "--icmpv4-src")? {
        state.flags.icmpv4_src = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --icmpv6-src VALUE
    if let Some(value) = try_string_flag(arg, args, "--icmpv6-src")? {
        state.flags.icmpv6_src = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --proxy-address VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--proxy-address")? {
        state.flags.proxy_address = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --proxy-port VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--proxy-port")? {
        state.flags.proxy_port = Some(
            value
                .parse::<u16>()
                .map_err(|_| format!("invalid value for --proxy-port: {value}"))?,
        );
        state.any_flag_set = true;
        return Ok(true);
    }

    // --proxy-connect-timeout VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--proxy-connect-timeout")? {
        state.flags.proxy_connect_timeout = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --proxy-tls-timeout VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--proxy-tls-timeout")? {
        state.flags.proxy_tls_timeout = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --proxy-tcp-keepalive VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--proxy-tcp-keepalive")? {
        state.flags.proxy_tcp_keepalive = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --proxy-keepalive-connections VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--proxy-keepalive-connections")? {
        state.flags.proxy_keepalive_connections = Some(parse_u32(&value, "--proxy-keepalive-connections")?);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --proxy-keepalive-timeout VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--proxy-keepalive-timeout")? {
        state.flags.proxy_keepalive_timeout = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --service-op-ip VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--service-op-ip")? {
        state.flags.service_op_ip = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --quic-connection-level-flow-control-limit VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--quic-connection-level-flow-control-limit")? {
        state.flags.quic_conn_flow_control =
            Some(parse_u64(&value, "--quic-connection-level-flow-control-limit")?);
        state.any_flag_set = true;
        return Ok(true);
    }

    // --quic-stream-level-flow-control-limit VALUE (hidden)
    if let Some(value) = try_string_flag(arg, args, "--quic-stream-level-flow-control-limit")? {
        state.flags.quic_stream_flow_control =
            Some(parse_u64(&value, "--quic-stream-level-flow-control-limit")?);
        state.any_flag_set = true;
        return Ok(true);
    }

    // Deprecated credential flags (hidden)
    if let Some(value) = try_string_flag(arg, args, "--api-key")? {
        state.flags.api_key = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    if let Some(value) = try_string_flag(arg, args, "--api-email")? {
        state.flags.api_email = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    if let Some(value) = try_string_flag(arg, args, "--api-ca-key")? {
        state.flags.api_ca_key = Some(value);
        state.any_flag_set = true;
        return Ok(true);
    }

    // Bool flags
    let arg_str = arg.to_string_lossy();
    let bool_match = match arg_str.as_ref() {
        "--no-autoupdate" => {
            state.flags.no_autoupdate = true;
            true
        }
        "--hello-world" => {
            state.flags.hello_world = true;
            true
        }
        "--no-tls-verify" => {
            state.flags.no_tls_verify = true;
            true
        }
        "--no-chunked-encoding" => {
            state.flags.no_chunked_encoding = true;
            true
        }
        "--http2-origin" => {
            state.flags.http2_origin = true;
            true
        }
        "--post-quantum" | "-pq" => {
            state.flags.post_quantum = Some(true);
            true
        }
        "--quiet" | "-q" => {
            state.flags.quiet = true;
            true
        }
        "--is-autoupdated" => {
            state.flags.is_autoupdated = true;
            true
        }
        "--bastion" => {
            state.flags.bastion = true;
            true
        }
        "--socks5" => {
            state.flags.socks5 = true;
            true
        }
        "--proxy-no-happy-eyeballs" => {
            state.flags.proxy_no_happy_eyeballs = true;
            true
        }
        "--quic-disable-pmtu-discovery" => {
            state.flags.quic_disable_pmtu = true;
            true
        }
        "--no-update-service" => {
            state.flags.no_update_service = true;
            true
        }
        "--proxy-dns" => {
            state.flags.proxy_dns = true;
            true
        }
        _ => false,
    };

    if bool_match {
        state.any_flag_set = true;
        return Ok(true);
    }

    Ok(false)
}

/// Try to extract a string value from `--flag VALUE` or `--flag=VALUE`.
fn try_string_flag(
    arg: &OsStr,
    args: &mut impl Iterator<Item = OsString>,
    name: &str,
) -> Result<Option<String>, String> {
    if arg == OsStr::new(name) {
        let value = args
            .next()
            .ok_or_else(|| surface_contract::missing_flag_value_message(name))?;
        return Ok(Some(value.to_string_lossy().into_owned()));
    }

    if let Some(value) = parse_equals_flag(arg, name) {
        return Ok(Some(value.to_owned()));
    }

    Ok(None)
}

fn finalize_cli(state: ParseState) -> Cli {
    let ParseState {
        flags,
        command,
        help_requested,
        version_requested,
        short_version,
        any_flag_set,
        awaiting_subcommand: _,
    } = state;

    if help_requested {
        return Cli {
            command: Command::Help,
            flags,
        };
    }

    if version_requested {
        return Cli {
            command: Command::Version { short: short_version },
            flags,
        };
    }

    // --short / -s without --version is still version output.
    if short_version {
        return Cli {
            command: Command::Version { short: true },
            flags,
        };
    }

    let command = match command {
        Some(cmd) => cmd,
        None => {
            if any_flag_set || !flags.rest_args.is_empty() {
                // Flags present but no command word — implicit tunnel mode.
                // Go baseline: root action delegates to tunnel.TunnelCommand(c).
                Command::Tunnel(TunnelSubcommand::Bare)
            } else {
                // Truly empty invocation — service mode.
                // Go baseline: handleServiceMode() in main.go.
                Command::ServiceMode
            }
        }
    };

    Cli { command, flags }
}

fn parse_equals_flag<'a>(arg: &'a OsStr, name: &str) -> Option<&'a str> {
    let arg = arg.to_str()?;
    arg.strip_prefix(name)?.strip_prefix('=')
}

fn parse_u32(value: &str, flag_name: &str) -> Result<u32, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value for {flag_name}: {value}"))
}

fn parse_u64(value: &str, flag_name: &str) -> Result<u64, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value for {flag_name}: {value}"))
}

fn set_path_flag(slot: &mut Option<PathBuf>, value: String, flag_name: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(surface_contract::repeated_flag_message(flag_name));
    }

    *slot = Some(PathBuf::from(value));
    Ok(())
}

fn set_command(slot: &mut Option<Command>, command: Command) -> Result<(), String> {
    if let Some(existing) = slot
        && *existing != command
    {
        return Err(surface_contract::multiple_commands_message(existing, &command));
    }

    *slot = Some(command);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_args;
    use crate::types::{ServiceAction, TunnelSubcommand};
    use crate::{Command, surface_contract};
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn parse(parts: &[&str]) -> crate::Cli {
        let args = std::iter::once(OsString::from(surface_contract::PROGRAM_NAME))
            .chain(parts.iter().map(OsString::from))
            .collect::<Vec<_>>();
        parse_args(args).expect("arguments should parse")
    }

    #[test]
    fn empty_invocation_is_service_mode() {
        let cli = parse(&[]);
        assert_eq!(cli.command, Command::ServiceMode);
    }

    #[test]
    fn help_flag() {
        let cli = parse(&[surface_contract::HELP_FLAG]);
        assert_eq!(cli.command, Command::Help);
    }

    #[test]
    fn version_flag() {
        let cli = parse(&[surface_contract::VERSION_FLAG]);
        assert_eq!(cli.command, Command::Version { short: false });
    }

    #[test]
    fn config_flag_can_appear_before_command() {
        let cli = parse(&[
            surface_contract::CONFIG_FLAG,
            "/tmp/config.yml",
            surface_contract::VALIDATE_COMMAND,
        ]);

        assert_eq!(cli.command, Command::Validate);
        assert_eq!(cli.flags.config_path, Some(PathBuf::from("/tmp/config.yml")));
    }

    #[test]
    fn config_flag_can_appear_after_command() {
        let config_eq = format!("{}=/tmp/config.yml", surface_contract::CONFIG_FLAG);
        let cli = parse(&[
            surface_contract::TUNNEL_COMMAND,
            surface_contract::TUNNEL_RUN,
            &config_eq,
        ]);

        assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Run));
        assert_eq!(cli.flags.config_path, Some(PathBuf::from("/tmp/config.yml")));
    }

    #[test]
    fn tunnel_bare_invocation() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND]);
        assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Bare));
    }

    #[test]
    fn tunnel_run_subcommand() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, surface_contract::TUNNEL_RUN]);
        assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Run));
    }

    #[test]
    fn tunnel_create_subcommand() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, surface_contract::TUNNEL_CREATE]);
        assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Create));
    }

    #[test]
    fn tunnel_list_subcommand() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, surface_contract::TUNNEL_LIST]);
        assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::List));
    }

    #[test]
    fn bare_run_is_tunnel_run() {
        let cli = parse(&[surface_contract::RUN_COMMAND]);
        assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Run));
    }

    #[test]
    fn top_level_commands() {
        assert_eq!(
            parse(&[surface_contract::UPDATE_COMMAND]).command,
            Command::Update
        );
        assert_eq!(parse(&[surface_contract::LOGIN_COMMAND]).command, Command::Login);
        assert_eq!(
            parse(&[surface_contract::PROXY_DNS_COMMAND]).command,
            Command::ProxyDns
        );
        assert_eq!(
            parse(&[surface_contract::ACCESS_COMMAND]).command,
            Command::Access(crate::types::AccessSubcommand::Bare)
        );
        assert_eq!(
            parse(&[surface_contract::FORWARD_COMMAND]).command,
            Command::Access(crate::types::AccessSubcommand::Bare)
        );
        assert_eq!(
            parse(&[surface_contract::TAIL_COMMAND]).command,
            Command::Tail(crate::types::TailSubcommand::Bare)
        );
        assert_eq!(
            parse(&[surface_contract::MANAGEMENT_COMMAND]).command,
            Command::Management(crate::types::ManagementSubcommand::Bare)
        );
        assert_eq!(
            parse(&[surface_contract::VALIDATE_COMMAND]).command,
            Command::Validate
        );
    }

    #[test]
    fn service_install() {
        let cli = parse(&[
            surface_contract::SERVICE_COMMAND,
            surface_contract::SERVICE_INSTALL,
        ]);
        assert_eq!(cli.command, Command::Service(ServiceAction::Install));
    }

    #[test]
    fn service_uninstall() {
        let cli = parse(&[
            surface_contract::SERVICE_COMMAND,
            surface_contract::SERVICE_UNINSTALL,
        ]);
        assert_eq!(cli.command, Command::Service(ServiceAction::Uninstall));
    }

    #[test]
    fn flags_without_command_is_implicit_tunnel() {
        let cli = parse(&["--url", "http://localhost:8080"]);
        assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Bare));
        assert_eq!(cli.flags.url, Some("http://localhost:8080".to_owned()));
    }

    #[test]
    fn credentials_file_flag() {
        let cli = parse(&[
            surface_contract::TUNNEL_COMMAND,
            surface_contract::TUNNEL_RUN,
            "--credentials-file",
            "/etc/cred.json",
        ]);
        assert_eq!(cli.flags.credentials_file, Some(PathBuf::from("/etc/cred.json")));
    }

    #[test]
    fn token_flag() {
        let cli = parse(&[
            surface_contract::TUNNEL_COMMAND,
            surface_contract::TUNNEL_RUN,
            "--token",
            "abc123",
        ]);
        assert_eq!(cli.flags.token, Some("abc123".to_owned()));
    }

    #[test]
    fn unknown_flags_collected_as_rest_args() {
        let cli = parse(&[
            surface_contract::TUNNEL_COMMAND,
            surface_contract::TUNNEL_RUN,
            "--some-future-flag",
            "value",
        ]);
        assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Run));
        assert!(cli.flags.rest_args.contains(&"--some-future-flag".to_owned()));
        assert!(cli.flags.rest_args.contains(&"value".to_owned()));
    }

    // --- Version --short / -s --------------------------------------------------

    #[test]
    fn version_short_flag() {
        let cli = parse(&[surface_contract::VERSION_FLAG, "--short"]);
        assert_eq!(cli.command, Command::Version { short: true });
    }

    #[test]
    fn version_short_flag_s() {
        let cli = parse(&[surface_contract::VERSION_FLAG, "-s"]);
        assert_eq!(cli.command, Command::Version { short: true });
    }

    // --- Access sub-tree -------------------------------------------------------

    #[test]
    fn access_bare() {
        let cli = parse(&[surface_contract::ACCESS_COMMAND]);
        assert_eq!(cli.command, Command::Access(crate::types::AccessSubcommand::Bare));
    }

    #[test]
    fn access_login() {
        let cli = parse(&[surface_contract::ACCESS_COMMAND, "login"]);
        assert_eq!(
            cli.command,
            Command::Access(crate::types::AccessSubcommand::Login)
        );
    }

    #[test]
    fn access_tcp() {
        let cli = parse(&[surface_contract::ACCESS_COMMAND, "tcp"]);
        assert_eq!(cli.command, Command::Access(crate::types::AccessSubcommand::Tcp));
    }

    #[test]
    fn access_rdp_alias() {
        // rdp/ssh/smb are aliases for tcp in Go baseline.
        let cli = parse(&[surface_contract::ACCESS_COMMAND, "rdp"]);
        assert_eq!(cli.command, Command::Access(crate::types::AccessSubcommand::Tcp));
    }

    #[test]
    fn access_ssh_config() {
        let cli = parse(&[surface_contract::ACCESS_COMMAND, "ssh-config"]);
        assert_eq!(
            cli.command,
            Command::Access(crate::types::AccessSubcommand::SshConfig)
        );
    }

    #[test]
    fn forward_alias_is_access() {
        let cli = parse(&[surface_contract::FORWARD_COMMAND]);
        assert_eq!(cli.command, Command::Access(crate::types::AccessSubcommand::Bare));
    }

    // --- Tail sub-tree ---------------------------------------------------------

    #[test]
    fn tail_bare() {
        let cli = parse(&[surface_contract::TAIL_COMMAND]);
        assert_eq!(cli.command, Command::Tail(crate::types::TailSubcommand::Bare));
    }

    #[test]
    fn tail_token() {
        let cli = parse(&[surface_contract::TAIL_COMMAND, "token"]);
        assert_eq!(cli.command, Command::Tail(crate::types::TailSubcommand::Token));
    }

    // --- Management sub-tree ---------------------------------------------------

    #[test]
    fn management_bare() {
        let cli = parse(&[surface_contract::MANAGEMENT_COMMAND]);
        assert_eq!(
            cli.command,
            Command::Management(crate::types::ManagementSubcommand::Bare)
        );
    }

    #[test]
    fn management_token() {
        let cli = parse(&[surface_contract::MANAGEMENT_COMMAND, "token"]);
        assert_eq!(
            cli.command,
            Command::Management(crate::types::ManagementSubcommand::Token)
        );
    }

    // --- Route sub-tree --------------------------------------------------------

    #[test]
    fn tunnel_route_bare() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, "route"]);
        assert_eq!(
            cli.command,
            Command::Tunnel(TunnelSubcommand::Route(crate::types::RouteSubcommand::Bare))
        );
    }

    #[test]
    fn tunnel_route_dns() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, "route", "dns"]);
        assert_eq!(
            cli.command,
            Command::Tunnel(TunnelSubcommand::Route(crate::types::RouteSubcommand::Dns))
        );
    }

    #[test]
    fn tunnel_route_ip_add() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, "route", "ip", "add"]);
        assert_eq!(
            cli.command,
            Command::Tunnel(TunnelSubcommand::Route(crate::types::RouteSubcommand::Ip(
                crate::types::IpRouteSubcommand::Add
            )))
        );
    }

    #[test]
    fn tunnel_route_ip_show() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, "route", "ip", "show"]);
        assert_eq!(
            cli.command,
            Command::Tunnel(TunnelSubcommand::Route(crate::types::RouteSubcommand::Ip(
                crate::types::IpRouteSubcommand::Show
            )))
        );
    }

    // --- Vnet sub-tree ---------------------------------------------------------

    #[test]
    fn tunnel_vnet_bare() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, "vnet"]);
        assert_eq!(
            cli.command,
            Command::Tunnel(TunnelSubcommand::Vnet(crate::types::VnetSubcommand::Bare))
        );
    }

    #[test]
    fn tunnel_vnet_add() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, "vnet", "add"]);
        assert_eq!(
            cli.command,
            Command::Tunnel(TunnelSubcommand::Vnet(crate::types::VnetSubcommand::Add))
        );
    }

    #[test]
    fn tunnel_vnet_list() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, "vnet", "list"]);
        assert_eq!(
            cli.command,
            Command::Tunnel(TunnelSubcommand::Vnet(crate::types::VnetSubcommand::List))
        );
    }

    // --- Ingress sub-tree ------------------------------------------------------

    #[test]
    fn tunnel_ingress_bare() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, "ingress"]);
        assert_eq!(
            cli.command,
            Command::Tunnel(TunnelSubcommand::Ingress(crate::types::IngressSubcommand::Bare))
        );
    }

    #[test]
    fn tunnel_ingress_validate() {
        let cli = parse(&[
            surface_contract::TUNNEL_COMMAND,
            "ingress",
            surface_contract::INGRESS_VALIDATE,
        ]);
        assert_eq!(
            cli.command,
            Command::Tunnel(TunnelSubcommand::Ingress(
                crate::types::IngressSubcommand::Validate
            ))
        );
    }

    #[test]
    fn tunnel_ingress_rule() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, "ingress", "rule"]);
        assert_eq!(
            cli.command,
            Command::Tunnel(TunnelSubcommand::Ingress(crate::types::IngressSubcommand::Rule))
        );
    }

    // --- Extended flag parsing -------------------------------------------------

    #[test]
    fn region_flag() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, "--region", "us"]);
        assert_eq!(cli.flags.region, Some("us".to_owned()));
    }

    #[test]
    fn quiet_flag() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, "-q"]);
        assert!(cli.flags.quiet);
    }

    #[test]
    fn no_update_service_flag() {
        let cli = parse(&[
            surface_contract::SERVICE_COMMAND,
            surface_contract::SERVICE_INSTALL,
            "--no-update-service",
        ]);
        assert!(cli.flags.no_update_service);
    }

    #[test]
    fn proxy_dns_flag() {
        let cli = parse(&["--proxy-dns"]);
        assert!(cli.flags.proxy_dns);
    }

    #[test]
    fn retries_flag() {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, "--retries", "5"]);
        assert_eq!(cli.flags.retries, Some(5));
    }
}
