use std::fmt;
use std::path::PathBuf;

use crate::surface_contract;

/// Which command context was active when help was requested.
///
/// Go baseline: urfave/cli auto-routes `--help` and `help` within the
/// active command's scope, showing the subcommand help template when a
/// command has subcommands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpTarget {
    /// Root-level help — `cloudflared --help` or `cloudflared help`.
    Root,
    /// `cloudflared update --help`
    Update,
    /// `cloudflared tunnel --help` or `cloudflared help tunnel`.
    Tunnel,
    /// `cloudflared access --help` or `cloudflared help access`.
    Access,
    /// `cloudflared management --help` or `cloudflared help management`.
    Management,
    // --- Per-subcommand targets ---
    ManagementToken,
    TunnelCreate,
    TunnelList,
    TunnelRun,
    TunnelDelete,
    TunnelCleanup,
    TunnelToken,
    TunnelInfo,
    TunnelReady,
    TunnelDiag,
    TunnelLogin,
    TunnelRoute,
    TunnelRouteDns,
    TunnelRouteLb,
    TunnelRouteIp,
    TunnelVnet,
    TunnelIngress,
}

/// Top-level command parsed from the CLI invocation.
///
/// Maps to the frozen Go baseline `commands()` registry and root `action()`:
/// - empty invocation (no args, no flags) enters `ServiceMode`
/// - `cloudflared <flags-only>` enters implicit `Tunnel(Bare)` (Go root action)
/// - explicit command words dispatch to their respective variants
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `cloudflared` with no args and no flags — enters service mode.
    /// Go baseline: `handleServiceMode()` in `main.go`.
    ServiceMode,

    /// `cloudflared help` or `--help` / `-h`, optionally scoped to a command.
    Help(HelpTarget),

    /// `cloudflared version` or `--version` / `-v` / `-V`
    /// When `short` is true, output version number only (`--short` / `-s`).
    Version { short: bool },

    /// `cloudflared update [--beta] [--force] [--staging] [--version VER]`
    Update,

    /// `cloudflared tunnel <subcmd>`
    Tunnel(TunnelSubcommand),

    /// `cloudflared login` — compat alias for `tunnel login`
    Login,

    /// `cloudflared proxy-dns` — removed feature, error message only
    ProxyDns,

    /// `cloudflared access <subcmd>` (alias `cloudflared forward`)
    Access(AccessSubcommand),

    /// `cloudflared tail [TUNNEL-ID]`
    Tail(TailSubcommand),

    /// `cloudflared management`
    Management(ManagementSubcommand),

    /// `cloudflared service install|uninstall`
    Service(ServiceAction),

    /// `cloudflared validate` — transitional alpha command
    Validate,
}

/// Tunnel subcommands from `tunnel/cmd.go` `Commands()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunnelSubcommand {
    /// `tunnel run [TUNNEL]`
    Run,
    /// `tunnel create NAME`
    Create,
    /// `tunnel list`
    List,
    /// `tunnel delete TUNNEL`
    Delete,
    /// `tunnel cleanup TUNNEL [--connector-id ID]`
    Cleanup,
    /// `tunnel token TUNNEL`
    Token,
    /// `tunnel info TUNNEL`
    Info,
    /// `tunnel ready`
    Ready,
    /// `tunnel diag`
    Diag,
    /// `tunnel route dns|lb|ip ...`
    Route(RouteSubcommand),
    /// `tunnel vnet add|list|delete|update ...`
    Vnet(VnetSubcommand),
    /// `tunnel ingress validate|rule [URL]`
    Ingress(IngressSubcommand),
    /// `tunnel login [--fedramp]`
    Login,
    /// `tunnel proxy-dns` — removed feature
    ProxyDns,
    /// `tunnel db-connect` — removed feature
    DbConnect,
    /// Bare `tunnel` invocation (no subcommand, flags only)
    Bare,
}

/// `tunnel route` sub-subcommands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteSubcommand {
    Dns,
    Lb,
    Ip(IpRouteSubcommand),
    /// Bare `tunnel route` with no sub-subcommand.
    Bare,
}

/// `tunnel route ip` sub-sub-subcommands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpRouteSubcommand {
    Add,
    Show,
    Delete,
    Get,
    /// Bare `tunnel route ip` with no action.
    Bare,
}

/// `tunnel vnet` sub-subcommands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VnetSubcommand {
    Add,
    List,
    Delete,
    Update,
    /// Bare `tunnel vnet` with no sub-subcommand.
    Bare,
}

/// `tunnel ingress` sub-subcommands (hidden command).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngressSubcommand {
    Validate,
    Rule,
    /// Bare `tunnel ingress` with no sub-subcommand.
    Bare,
}

/// `access` subcommands from `access/cmd.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessSubcommand {
    Login,
    Curl,
    Token,
    /// `access tcp` (aliases: `rdp`, `ssh`, `smb`)
    Tcp,
    SshConfig,
    SshGen,
    /// Bare `access` with no subcommand.
    Bare,
}

/// `tail` subcommands from `tail/cmd.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TailSubcommand {
    /// Hidden `tail token` subcommand.
    Token,
    /// Bare `tail [TUNNEL-ID]` — the normal streaming invocation.
    Bare,
}

/// `management` subcommands from `management/cmd.go` (entirely hidden).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagementSubcommand {
    /// Hidden `management token` subcommand.
    Token,
    /// Bare `management` invocation.
    Bare,
}

/// Service install/uninstall actions from `linux_service.go`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceAction {
    Install,
    Uninstall,
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(surface_contract::command_label(self))
    }
}

impl Command {
    /// Build a human-readable label including sub-tree depth.
    /// Used for stub-not-implemented messages and diagnostics.
    pub fn full_label(&self) -> String {
        match self {
            Command::Access(sub) => access_full_label(sub),
            Command::Tunnel(sub) => tunnel_full_label(sub),
            Command::Tail(sub) => tail_full_label(sub),
            Command::Management(sub) => management_full_label(sub),
            Command::Service(action) => service_full_label(action),
            other => format!("{other}"),
        }
    }
}

fn access_full_label(sub: &AccessSubcommand) -> String {
    match sub {
        AccessSubcommand::Login => "access login".into(),
        AccessSubcommand::Curl => "access curl".into(),
        AccessSubcommand::Token => "access token".into(),
        AccessSubcommand::Tcp => "access tcp".into(),
        AccessSubcommand::SshConfig => "access ssh-config".into(),
        AccessSubcommand::SshGen => "access ssh-gen".into(),
        AccessSubcommand::Bare => "access".into(),
    }
}

fn tail_full_label(sub: &TailSubcommand) -> String {
    match sub {
        TailSubcommand::Token => "tail token".into(),
        TailSubcommand::Bare => "tail".into(),
    }
}

fn management_full_label(sub: &ManagementSubcommand) -> String {
    match sub {
        ManagementSubcommand::Token => "management token".into(),
        ManagementSubcommand::Bare => "management".into(),
    }
}

fn service_full_label(action: &ServiceAction) -> String {
    match action {
        ServiceAction::Install => "service install".into(),
        ServiceAction::Uninstall => "service uninstall".into(),
    }
}

fn tunnel_full_label(sub: &TunnelSubcommand) -> String {
    match sub {
        TunnelSubcommand::Route(r) => route_full_label(r),
        TunnelSubcommand::Vnet(v) => vnet_full_label(v),
        TunnelSubcommand::Ingress(i) => ingress_full_label(i),
        other => format!("tunnel {other}"),
    }
}

fn route_full_label(sub: &RouteSubcommand) -> String {
    match sub {
        RouteSubcommand::Dns => "tunnel route dns".into(),
        RouteSubcommand::Lb => "tunnel route lb".into(),
        RouteSubcommand::Ip(ip) => match ip {
            IpRouteSubcommand::Add => "tunnel route ip add".into(),
            IpRouteSubcommand::Show => "tunnel route ip show".into(),
            IpRouteSubcommand::Delete => "tunnel route ip delete".into(),
            IpRouteSubcommand::Get => "tunnel route ip get".into(),
            IpRouteSubcommand::Bare => "tunnel route ip".into(),
        },
        RouteSubcommand::Bare => "tunnel route".into(),
    }
}

fn vnet_full_label(sub: &VnetSubcommand) -> String {
    match sub {
        VnetSubcommand::Add => "tunnel vnet add".into(),
        VnetSubcommand::List => "tunnel vnet list".into(),
        VnetSubcommand::Delete => "tunnel vnet delete".into(),
        VnetSubcommand::Update => "tunnel vnet update".into(),
        VnetSubcommand::Bare => "tunnel vnet".into(),
    }
}

fn ingress_full_label(sub: &IngressSubcommand) -> String {
    match sub {
        IngressSubcommand::Validate => "tunnel ingress validate".into(),
        IngressSubcommand::Rule => "tunnel ingress rule".into(),
        IngressSubcommand::Bare => "tunnel ingress".into(),
    }
}

impl fmt::Display for TunnelSubcommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Run => surface_contract::TUNNEL_RUN,
            Self::Create => surface_contract::TUNNEL_CREATE,
            Self::List => surface_contract::TUNNEL_LIST,
            Self::Delete => surface_contract::TUNNEL_DELETE,
            Self::Cleanup => surface_contract::TUNNEL_CLEANUP,
            Self::Token => surface_contract::TUNNEL_TOKEN,
            Self::Info => surface_contract::TUNNEL_INFO,
            Self::Ready => surface_contract::TUNNEL_READY,
            Self::Diag => surface_contract::TUNNEL_DIAG,
            Self::Route(_) => surface_contract::TUNNEL_ROUTE,
            Self::Vnet(_) => surface_contract::TUNNEL_VNET,
            Self::Ingress(_) => surface_contract::TUNNEL_INGRESS,
            Self::Login => surface_contract::TUNNEL_LOGIN,
            Self::ProxyDns => surface_contract::TUNNEL_PROXY_DNS,
            Self::DbConnect => surface_contract::TUNNEL_DB_CONNECT,
            Self::Bare => "(bare)",
        };
        f.write_str(label)
    }
}

/// Parsed global flags from the command line.
///
/// Maps to the frozen Go baseline flags defined in `tunnel/cmd.go` `Flags()`.
/// Fields are `Option<T>` to distinguish "not provided" from "not supported
/// yet". The parser progressively fills these; execution code checks what it
/// needs.
#[derive(Debug, Default)]
pub struct GlobalFlags {
    // --- Cloudflare config ---
    pub config_path: Option<PathBuf>,
    pub origincert: Option<PathBuf>,
    pub no_autoupdate: bool,
    pub autoupdate_freq: Option<String>,
    pub metrics: Option<String>,
    pub pidfile: Option<PathBuf>,
    pub update_beta: bool,
    pub update_staging: bool,
    pub update_version: Option<String>,

    // --- Credentials ---
    pub credentials_file: Option<PathBuf>,
    pub credentials_contents: Option<String>,
    pub token: Option<String>,
    pub token_file: Option<PathBuf>,

    // --- Edge connection ---
    pub edge: Vec<String>,
    pub region: Option<String>,
    pub edge_ip_version: Option<String>,
    pub edge_bind_address: Option<String>,

    // --- Tunnel identity ---
    pub tunnel_name: Option<String>,
    pub hostname: Option<String>,
    pub tunnel_id: Option<String>,
    pub lb_pool: Option<String>,
    pub tag: Vec<String>,

    // --- Logging ---
    pub loglevel: Option<String>,
    pub transport_loglevel: Option<String>,
    pub logfile: Option<PathBuf>,
    pub log_directory: Option<PathBuf>,
    pub log_format_output: Option<String>,
    pub trace_output: Option<String>,

    // --- Tunnel behavior ---
    pub grace_period: Option<String>,
    pub protocol: Option<String>,
    pub retries: Option<u32>,
    pub ha_connections: Option<u32>,
    pub label: Option<String>,
    pub url: Option<String>,
    pub hello_world: bool,
    pub post_quantum: Option<bool>,
    pub management_diagnostics: Option<bool>,
    pub management_hostname: Option<String>,
    pub api_url: Option<String>,
    pub features: Vec<String>,
    pub is_autoupdated: bool,
    pub metrics_update_freq: Option<String>,
    pub max_edge_addr_retries: Option<u32>,
    pub rpc_timeout: Option<String>,
    pub heartbeat_interval: Option<String>,
    pub heartbeat_count: Option<u32>,
    pub write_stream_timeout: Option<String>,
    pub quic_disable_pmtu: bool,
    pub quic_conn_flow_control: Option<u64>,
    pub quic_stream_flow_control: Option<u64>,

    // --- Proxy/origin ---
    pub unix_socket: Option<String>,
    pub http_host_header: Option<String>,
    pub origin_server_name: Option<String>,
    pub origin_ca_pool: Option<String>,
    pub no_tls_verify: bool,
    pub no_chunked_encoding: bool,
    pub http2_origin: bool,
    pub bastion: bool,
    pub socks5: bool,
    pub proxy_address: Option<String>,
    pub proxy_port: Option<u16>,
    pub proxy_connect_timeout: Option<String>,
    pub proxy_tls_timeout: Option<String>,
    pub proxy_tcp_keepalive: Option<String>,
    pub proxy_no_happy_eyeballs: bool,
    pub proxy_keepalive_connections: Option<u32>,
    pub proxy_keepalive_timeout: Option<String>,
    pub service_op_ip: Option<String>,

    // --- ICMP ---
    pub icmpv4_src: Option<String>,
    pub icmpv6_src: Option<String>,
    pub max_active_flows: Option<u64>,

    // --- Deprecated (kept for compat) ---
    pub api_key: Option<String>,
    pub api_email: Option<String>,
    pub api_ca_key: Option<String>,

    // --- Service install ---
    pub no_update_service: bool,

    // --- Proxy DNS (removed) ---
    pub proxy_dns: bool,

    // --- Subcommand: list/info filters ---
    pub output_format: Option<String>,
    pub show_deleted: bool,
    pub name_prefix: Option<String>,
    pub exclude_name_prefix: Option<String>,
    pub filter_when: Option<String>,
    pub show_recently_disconnected: bool,
    pub sort_by: Option<String>,
    pub invert_sort: bool,

    // --- Subcommand: create ---
    pub tunnel_secret: Option<String>,

    // --- Subcommand: delete/cleanup ---
    pub force: bool,
    pub connector_id: Option<String>,

    // --- Subcommand: login ---
    pub login_url: Option<String>,
    pub callback_url: Option<String>,
    pub fedramp: bool,

    // --- Subcommand: route ---
    pub overwrite_dns: bool,
    pub vnet_id: Option<String>,

    // --- Subcommand: route ip show/list ---
    pub filter_is_deleted: bool,
    pub filter_tunnel_id: Option<String>,
    pub filter_network_subset: Option<String>,
    pub filter_network_superset: Option<String>,
    pub filter_comment_is: Option<String>,
    pub filter_vnet_id: Option<String>,

    // --- Subcommand: vnet ---
    pub vnet_default: bool,
    pub vnet_comment: Option<String>,
    pub vnet_is_default_filter: bool,

    // --- Subcommand: ingress ---
    pub ingress_json: Option<String>,

    // --- Subcommand: diag ---
    pub diag_container_id: Option<String>,
    pub diag_pod_id: Option<String>,
    pub no_diag_logs: bool,
    pub no_diag_metrics: bool,
    pub no_diag_system: bool,
    pub no_diag_runtime: bool,
    pub no_diag_network: bool,

    // --- Unrecognized but forwarded ---
    /// Flags and arguments not yet handled by the parser.
    /// Collected instead of erroring so that Go-compatible flags
    /// do not break invocation before the parser learns about them.
    pub rest_args: Vec<String>,
}

/// Parsed CLI invocation.
#[derive(Debug)]
pub struct Cli {
    pub command: Command,
    pub flags: GlobalFlags,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_display_top_level() {
        assert_eq!(
            Command::Help(HelpTarget::Root).to_string(),
            surface_contract::HELP_COMMAND
        );
        assert_eq!(
            Command::Version { short: false }.to_string(),
            surface_contract::VERSION_COMMAND
        );
        assert_eq!(Command::Validate.to_string(), surface_contract::VALIDATE_COMMAND);
        assert_eq!(Command::Update.to_string(), surface_contract::UPDATE_COMMAND);
        assert_eq!(
            Command::Tunnel(TunnelSubcommand::Run).to_string(),
            surface_contract::TUNNEL_COMMAND
        );
        assert_eq!(Command::Login.to_string(), surface_contract::LOGIN_COMMAND);
        assert_eq!(Command::ProxyDns.to_string(), surface_contract::PROXY_DNS_COMMAND);
        assert_eq!(
            Command::Access(AccessSubcommand::Bare).to_string(),
            surface_contract::ACCESS_COMMAND
        );
        assert_eq!(
            Command::Tail(TailSubcommand::Bare).to_string(),
            surface_contract::TAIL_COMMAND
        );
        assert_eq!(
            Command::Management(ManagementSubcommand::Bare).to_string(),
            surface_contract::MANAGEMENT_COMMAND
        );
        assert_eq!(
            Command::Service(ServiceAction::Install).to_string(),
            surface_contract::SERVICE_COMMAND
        );
        assert_eq!(
            Command::ServiceMode.to_string(),
            surface_contract::SERVICE_MODE_LABEL
        );
    }
}
