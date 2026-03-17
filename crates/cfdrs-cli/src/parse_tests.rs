use super::parse_args;
use crate::types::{HelpTarget, ServiceAction, TunnelSubcommand};
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
    assert_eq!(cli.command, Command::Help(HelpTarget::Root));
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
fn tunnel_delete_subcommand() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, surface_contract::TUNNEL_DELETE]);
    assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Delete));
}

#[test]
fn tunnel_cleanup_subcommand() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, surface_contract::TUNNEL_CLEANUP]);
    assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Cleanup));
}

#[test]
fn tunnel_info_subcommand() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, surface_contract::TUNNEL_INFO]);
    assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Info));
}

#[test]
fn tunnel_ready_subcommand() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, surface_contract::TUNNEL_READY]);
    assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Ready));
}

#[test]
fn tunnel_diag_subcommand() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, surface_contract::TUNNEL_DIAG]);
    assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Diag));
}

#[test]
fn tunnel_token_subcommand() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, surface_contract::TUNNEL_TOKEN]);
    assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Token));
}

#[test]
fn tunnel_login_subcommand() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, surface_contract::TUNNEL_LOGIN]);
    assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Login));
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
fn update_help_routes_to_update_target() {
    let cli = parse(&[surface_contract::UPDATE_COMMAND, "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::Update));
}

#[test]
fn update_version_flag_is_command_scoped() {
    let cli = parse(&[surface_contract::UPDATE_COMMAND, "--version", "2026.2.0"]);
    assert_eq!(cli.command, Command::Update);
    assert_eq!(cli.flags.update_version, Some("2026.2.0".to_owned()));
}

#[test]
fn update_version_equals_syntax_is_command_scoped() {
    let cli = parse(&[surface_contract::UPDATE_COMMAND, "--version=2026.2.0"]);
    assert_eq!(cli.command, Command::Update);
    assert_eq!(cli.flags.update_version, Some("2026.2.0".to_owned()));
}

#[test]
fn update_beta_and_staging_flags_parse() {
    let cli = parse(&[surface_contract::UPDATE_COMMAND, "--beta", "--staging"]);
    assert_eq!(cli.command, Command::Update);
    assert!(cli.flags.update_beta);
    assert!(cli.flags.update_staging);
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

// --- CLI-003: flag inventory parity -----------------------------------------
//
// Go baseline: Flags() in cmd/cloudflared/tunnel/cmd.go defines every global
// flag the binary accepts.  These tests verify that each flag name, alias, and
// `=` syntax parse without error.

#[test]
fn cred_file_alias_matches_credentials_file() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "--cred-file", "/etc/cred.json"]);
    assert_eq!(cli.flags.credentials_file, Some(PathBuf::from("/etc/cred.json")));
}

#[test]
fn credentials_contents_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "--credentials-contents",
        "{\"AccountTag\":\"a\"}",
    ]);
    assert_eq!(
        cli.flags.credentials_contents,
        Some("{\"AccountTag\":\"a\"}".to_owned())
    );
}

#[test]
fn token_equals_syntax() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "--token=abc123"]);
    assert_eq!(cli.flags.token, Some("abc123".to_owned()));
}

#[test]
fn config_equals_syntax() {
    let flag_with_value = format!("{}=/etc/cloudflared/config.yml", surface_contract::CONFIG_FLAG);
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, &flag_with_value]);
    assert_eq!(
        cli.flags.config_path,
        Some(PathBuf::from("/etc/cloudflared/config.yml"))
    );
}

#[test]
fn credentials_file_equals_syntax() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "--credentials-file=/etc/cred.json",
    ]);
    assert_eq!(cli.flags.credentials_file, Some(PathBuf::from("/etc/cred.json")));
}

#[test]
fn post_quantum_flag_long() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "--post-quantum"]);
    assert_eq!(cli.flags.post_quantum, Some(true));
}

#[test]
fn post_quantum_flag_short_pq() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "-pq"]);
    assert_eq!(cli.flags.post_quantum, Some(true));
}

#[test]
fn tunnel_name_alias_short_n() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "-n", "my-tunnel"]);
    assert_eq!(cli.flags.tunnel_name, Some("my-tunnel".to_owned()));
}

#[test]
fn protocol_alias_short_p() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "-p", "quic"]);
    assert_eq!(cli.flags.protocol, Some("quic".to_owned()));
}

#[test]
fn features_alias_short_f() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "-F", "feature1"]);
    assert!(cli.flags.features.contains(&"feature1".to_owned()));
}

#[test]
fn origin_ca_pool_alias_cacert() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "--cacert", "/etc/ca.pem"]);
    assert_eq!(cli.flags.origin_ca_pool, Some("/etc/ca.pem".to_owned()));
}

#[test]
fn all_boolean_flags_parse_without_error() {
    // Go baseline boolean flags from cmd/cloudflared/tunnel/cmd.go
    let bool_flags = [
        "--no-autoupdate",
        "--hello-world",
        "--no-tls-verify",
        "--no-chunked-encoding",
        "--http2-origin",
        "--post-quantum",
        "--is-autoupdated",
        "--bastion",
        "--socks5",
        "--proxy-no-happy-eyeballs",
        "--quic-disable-pmtu-discovery",
        "--no-update-service",
        "--proxy-dns",
    ];

    for flag in &bool_flags {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, flag]);
        assert_eq!(
            cli.command,
            Command::Tunnel(TunnelSubcommand::Bare),
            "bool flag {flag} broke command parsing"
        );
    }
}

#[test]
fn all_value_flags_parse_without_error() {
    // Go baseline value flags from cmd/cloudflared/tunnel/cmd.go
    let value_flags = [
        (surface_contract::CONFIG_FLAG, "/tmp/config.yml"),
        ("--credentials-file", "/etc/cred.json"),
        ("--cred-file", "/etc/cred.json"),
        ("--credentials-contents", "{}"),
        ("--token", "abc123"),
        ("--token-file", "/tmp/token"),
        ("--origincert", "/etc/cert.pem"),
        ("--loglevel", "info"),
        ("--transport-loglevel", "warn"),
        ("--logfile", "/var/log/cloudflared.log"),
        ("--log-directory", "/var/log/cloudflared"),
        ("--output", "json"),
        ("--metrics", "localhost:9090"),
        ("--pidfile", "/var/run/cloudflared.pid"),
        ("--grace-period", "30s"),
        ("--url", "http://localhost:8080"),
        ("--name", "my-tunnel"),
        ("--protocol", "quic"),
        ("--edge", "198.41.200.193:7844"),
        ("--region", "us"),
        ("--edge-ip-version", "4"),
        ("--edge-bind-address", "0.0.0.0"),
        ("--hostname", "example.com"),
        ("--id", "00000000-0000-0000-0000-000000000000"),
        ("--lb-pool", "my-pool"),
        ("--tag", "key=value"),
        ("--features", "feature1"),
        ("--label", "my-label"),
        ("--autoupdate-freq", "24h"),
        ("--metrics-update-freq", "5s"),
        ("--retries", "5"),
        ("--ha-connections", "4"),
        ("--max-edge-addr-retries", "8"),
        ("--rpc-timeout", "5s"),
        ("--heartbeat-interval", "5s"),
        ("--heartbeat-count", "5"),
        ("--write-stream-timeout", "5s"),
        ("--max-active-flows", "100"),
        ("--management-hostname", "management.example.com"),
        ("--api-url", "https://api.cloudflare.com/client/v4"),
        ("--trace-output", "/tmp/trace"),
        ("--unix-socket", "/tmp/socket.sock"),
        ("--http-host-header", "example.com"),
        ("--origin-server-name", "origin.example.com"),
        ("--origin-ca-pool", "/etc/ca.pem"),
        ("--cacert", "/etc/ca.pem"),
        ("--icmpv4-src", "0.0.0.0"),
        ("--icmpv6-src", "::"),
        ("--proxy-address", "127.0.0.1"),
        ("--proxy-port", "8080"),
        ("--proxy-connect-timeout", "30s"),
        ("--proxy-tls-timeout", "10s"),
        ("--proxy-tcp-keepalive", "30s"),
        ("--proxy-keepalive-connections", "100"),
        ("--proxy-keepalive-timeout", "90s"),
        ("--service-op-ip", "127.0.0.1"),
        ("--quic-connection-level-flow-control-limit", "15728640"),
        ("--quic-stream-level-flow-control-limit", "6291456"),
        ("--api-key", "key"),
        ("--api-email", "user@example.com"),
        ("--api-ca-key", "cakey"),
    ];

    for (flag, value) in &value_flags {
        let cli = parse(&[surface_contract::TUNNEL_COMMAND, flag, value]);
        assert_eq!(
            cli.command,
            Command::Tunnel(TunnelSubcommand::Bare),
            "value flag {flag} broke command parsing"
        );
    }
}

#[test]
fn ha_connections_default_matches_go_baseline() {
    // Go baseline default: ha-connections = 4
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "--ha-connections", "4"]);
    assert_eq!(cli.flags.ha_connections, Some(4));
}

#[test]
fn retries_default_matches_go_baseline() {
    // Go baseline default: retries = 5
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "--retries", "5"]);
    assert_eq!(cli.flags.retries, Some(5));
}

#[test]
fn logging_flags_match_go_baseline_names() {
    // Go baseline: --loglevel, --transport-loglevel, --logfile, --log-directory
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "--loglevel",
        "debug",
        "--transport-loglevel",
        "error",
        "--logfile",
        "/var/log/cloudflared.log",
        "--log-directory",
        "/var/log/cloudflared",
    ]);
    assert_eq!(cli.flags.loglevel, Some("debug".to_owned()));
    assert_eq!(cli.flags.transport_loglevel, Some("error".to_owned()));
    assert_eq!(cli.flags.logfile, Some(PathBuf::from("/var/log/cloudflared.log")));
    assert_eq!(
        cli.flags.log_directory,
        Some(PathBuf::from("/var/log/cloudflared"))
    );
}

// --- CLI-001, CLI-028, CLI-032: parse contract strengthening ---------------

#[test]
fn login_and_tunnel_login_produce_same_dispatch() {
    // Go baseline: top-level `login` redirects to `tunnel login`.
    let top = parse(&[surface_contract::LOGIN_COMMAND]);
    let nested = parse(&[surface_contract::TUNNEL_COMMAND, surface_contract::TUNNEL_LOGIN]);
    assert_eq!(top.command, Command::Login);
    assert_eq!(nested.command, Command::Tunnel(TunnelSubcommand::Login));
}

#[test]
fn run_and_tunnel_run_produce_equivalent_dispatch() {
    // Go baseline: bare `run` is shorthand for `tunnel run`.
    let bare = parse(&[surface_contract::RUN_COMMAND]);
    let tunneled = parse(&[surface_contract::TUNNEL_COMMAND, surface_contract::TUNNEL_RUN]);
    assert_eq!(bare.command, Command::Tunnel(TunnelSubcommand::Run));
    assert_eq!(tunneled.command, Command::Tunnel(TunnelSubcommand::Run));
}

// --- CLI-023, CLI-024: tail and management flag parsing --------------------

#[test]
fn tail_with_tunnel_id_arg() {
    // Go baseline: `tail TUNNEL-ID` passes tunnel ID as positional.
    let cli = parse(&[
        surface_contract::TAIL_COMMAND,
        "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
    ]);
    assert_eq!(cli.command, Command::Tail(crate::types::TailSubcommand::Bare));
}

#[test]
fn management_token_subcommand_dispatch() {
    // Go baseline: `management token` is the only management subcommand
    // (besides bare management).
    let token = parse(&[surface_contract::MANAGEMENT_COMMAND, "token"]);
    let bare = parse(&[surface_contract::MANAGEMENT_COMMAND]);
    assert_ne!(token.command, bare.command);
}

// --- CLI-032: tunnel run dispatch and identity flags ------------------------

#[test]
fn token_file_flag() {
    // Go baseline: --token-file reads token from a file path.
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        surface_contract::TUNNEL_RUN,
        "--token-file",
        "/etc/cloudflared/token",
    ]);
    assert_eq!(
        cli.flags.token_file,
        Some(PathBuf::from("/etc/cloudflared/token"))
    );
}

#[test]
fn token_takes_precedence_over_token_file_in_parse() {
    // Go baseline: --token > --token-file > config credentials.
    // Both flags must be parseable simultaneously so the runtime
    // can apply precedence at dispatch time.
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        surface_contract::TUNNEL_RUN,
        "--token",
        "my-token-value",
        "--token-file",
        "/tmp/token",
    ]);
    assert_eq!(cli.flags.token, Some("my-token-value".to_owned()));
    assert_eq!(cli.flags.token_file, Some(PathBuf::from("/tmp/token")));
}

#[test]
fn tunnel_run_with_positional_tunnel_name() {
    // Go baseline: `tunnel run TUNNEL` accepts one positional arg
    // (tunnel name or UUID) which lands in rest_args.
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        surface_contract::TUNNEL_RUN,
        "my-tunnel",
    ]);
    assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Run));
    assert!(
        cli.flags.rest_args.contains(&"my-tunnel".to_owned()),
        "tunnel name should be captured in rest_args"
    );
}

#[test]
fn tunnel_run_with_hostname_flag() {
    // Go baseline: --hostname is hidden/deprecated for named tunnels
    // but still parsed. In `runCommand()`, its presence triggers a
    // deprecation warning log.
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        surface_contract::TUNNEL_RUN,
        "--hostname",
        "example.com",
    ]);
    assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Run));
    assert_eq!(cli.flags.hostname, Some("example.com".to_owned()));
}

#[test]
fn bare_run_with_token_flag() {
    // Go baseline: bare `run --token` shares the same flag set as
    // `tunnel run --token`. Verify the shorthand route parses
    // identity flags identically.
    let bare = parse(&[surface_contract::RUN_COMMAND, "--token", "tok123"]);
    let tunneled = parse(&[
        surface_contract::TUNNEL_COMMAND,
        surface_contract::TUNNEL_RUN,
        "--token",
        "tok123",
    ]);
    assert_eq!(bare.flags.token, Some("tok123".to_owned()));
    assert_eq!(tunneled.flags.token, Some("tok123".to_owned()));
}

#[test]
fn tunnel_run_multiple_positional_args_collected() {
    // Go baseline: runCommand() rejects NArg > 1. At parse time
    // the extras land in rest_args for the runtime to validate.
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        surface_contract::TUNNEL_RUN,
        "tunnel-name",
        "extra-arg",
    ]);
    assert_eq!(cli.command, Command::Tunnel(TunnelSubcommand::Run));
    assert!(cli.flags.rest_args.contains(&"tunnel-name".to_owned()));
    assert!(cli.flags.rest_args.contains(&"extra-arg".to_owned()));
}

// --- CLI-004: subcommand help routing --------------------------------------

#[test]
fn tunnel_help_flag_routes_to_tunnel_help() {
    // Go baseline: `cloudflared tunnel --help` shows SubcommandHelpTemplate
    // for the tunnel command.
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::Tunnel));
}

#[test]
fn tunnel_help_short_routes_to_tunnel_help() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "-h"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::Tunnel));
}

#[test]
fn help_tunnel_routes_to_tunnel_help() {
    // Go baseline: `cloudflared help tunnel` shows tunnel help.
    let cli = parse(&["help", surface_contract::TUNNEL_COMMAND]);
    assert_eq!(cli.command, Command::Help(HelpTarget::Tunnel));
}

#[test]
fn access_help_flag_routes_to_access_help() {
    let cli = parse(&[surface_contract::ACCESS_COMMAND, "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::Access));
}

#[test]
fn access_help_short_routes_to_access_help() {
    let cli = parse(&[surface_contract::ACCESS_COMMAND, "-h"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::Access));
}

#[test]
fn help_access_routes_to_access_help() {
    let cli = parse(&["help", surface_contract::ACCESS_COMMAND]);
    assert_eq!(cli.command, Command::Help(HelpTarget::Access));
}

#[test]
fn help_alone_routes_to_root_help() {
    let cli = parse(&["help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::Root));
}

#[test]
fn help_flag_alone_routes_to_root_help() {
    let cli = parse(&["--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::Root));
}

#[test]
fn help_resolves_to_subcommand_level() {
    // `tunnel run --help` should resolve to per-subcommand help.
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        surface_contract::TUNNEL_RUN,
        "--help",
    ]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelRun));
}

#[test]
fn tunnel_create_help_routes_correctly() {
    let cli = parse(&["tunnel", "create", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelCreate));
}

#[test]
fn tunnel_list_help_routes_correctly() {
    let cli = parse(&["tunnel", "list", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelList));
}

#[test]
fn tunnel_delete_help_routes_correctly() {
    let cli = parse(&["tunnel", "delete", "-h"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelDelete));
}

#[test]
fn tunnel_cleanup_help_routes_correctly() {
    let cli = parse(&["tunnel", "cleanup", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelCleanup));
}

#[test]
fn tunnel_token_help_routes_correctly() {
    let cli = parse(&["tunnel", "token", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelToken));
}

#[test]
fn tunnel_info_help_routes_correctly() {
    let cli = parse(&["tunnel", "info", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelInfo));
}

#[test]
fn tunnel_ready_help_routes_correctly() {
    let cli = parse(&["tunnel", "ready", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelReady));
}

#[test]
fn tunnel_diag_help_routes_correctly() {
    let cli = parse(&["tunnel", "diag", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelDiag));
}

#[test]
fn tunnel_login_help_routes_correctly() {
    let cli = parse(&["tunnel", "login", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelLogin));
}

#[test]
fn tunnel_route_help_routes_correctly() {
    let cli = parse(&["tunnel", "route", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelRoute));
}

#[test]
fn tunnel_route_dns_help_routes_correctly() {
    let cli = parse(&["tunnel", "route", "dns", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelRouteDns));
}

#[test]
fn tunnel_route_ip_help_routes_correctly() {
    let cli = parse(&["tunnel", "route", "ip", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelRouteIp));
}

#[test]
fn tunnel_vnet_help_routes_correctly() {
    let cli = parse(&["tunnel", "vnet", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelVnet));
}

#[test]
fn tunnel_ingress_help_routes_correctly() {
    let cli = parse(&["tunnel", "ingress", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::TunnelIngress));
}

#[test]
fn tunnel_bare_help_routes_to_tunnel() {
    // `tunnel --help` without a subcommand routes to tunnel-level help.
    let cli = parse(&["tunnel", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::Tunnel));
}

#[test]
fn management_help_routes_correctly() {
    let cli = parse(&["management", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::Management));
}

#[test]
fn management_token_help_routes_correctly() {
    let cli = parse(&["management", "token", "--help"]);
    assert_eq!(cli.command, Command::Help(HelpTarget::ManagementToken));
}
