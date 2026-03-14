use std::fmt;
use std::path::PathBuf;

use crate::surface_contract;

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

    /// `cloudflared help` or `--help` / `-h`
    Help,

    /// `cloudflared version` or `--version` / `-v` / `-V`
    Version,

    /// `cloudflared update [--beta] [--force] [--staging] [--version VER]`
    Update,

    /// `cloudflared tunnel <subcmd>`
    Tunnel(TunnelSubcommand),

    /// `cloudflared login` — compat alias for `tunnel login`
    Login,

    /// `cloudflared proxy-dns` — removed feature, error message only
    ProxyDns,

    /// `cloudflared access <subcmd>` (alias `cloudflared forward`)
    Access,

    /// `cloudflared tail [TUNNEL-ID]`
    Tail,

    /// `cloudflared management`
    Management,

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
    Route,
    /// `tunnel vnet add|list|delete|update ...`
    Vnet,
    /// `tunnel ingress validate|rule [URL]`
    Ingress,
    /// `tunnel login [--fedramp]`
    Login,
    /// `tunnel proxy-dns` — removed feature
    ProxyDns,
    /// `tunnel db-connect` — removed feature
    DbConnect,
    /// Bare `tunnel` invocation (no subcommand, flags only)
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
            Self::Route => surface_contract::TUNNEL_ROUTE,
            Self::Vnet => surface_contract::TUNNEL_VNET,
            Self::Ingress => surface_contract::TUNNEL_INGRESS,
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

    // --- Logging ---
    pub loglevel: Option<String>,
    pub transport_loglevel: Option<String>,
    pub logfile: Option<PathBuf>,
    pub log_directory: Option<PathBuf>,
    pub log_format_output: Option<String>,

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

    // --- Proxy/origin ---
    pub unix_socket: Option<String>,
    pub http_host_header: Option<String>,
    pub origin_server_name: Option<String>,
    pub origin_ca_pool: Option<String>,
    pub no_tls_verify: bool,
    pub no_chunked_encoding: bool,
    pub http2_origin: bool,

    // --- ICMP ---
    pub icmpv4_src: Option<String>,
    pub icmpv6_src: Option<String>,
    pub max_active_flows: Option<u64>,

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
        assert_eq!(Command::Help.to_string(), surface_contract::HELP_COMMAND);
        assert_eq!(Command::Version.to_string(), surface_contract::VERSION_COMMAND);
        assert_eq!(Command::Validate.to_string(), surface_contract::VALIDATE_COMMAND);
        assert_eq!(Command::Update.to_string(), surface_contract::UPDATE_COMMAND);
        assert_eq!(
            Command::Tunnel(TunnelSubcommand::Run).to_string(),
            surface_contract::TUNNEL_COMMAND
        );
        assert_eq!(Command::Login.to_string(), surface_contract::LOGIN_COMMAND);
        assert_eq!(Command::ProxyDns.to_string(), surface_contract::PROXY_DNS_COMMAND);
        assert_eq!(Command::Access.to_string(), surface_contract::ACCESS_COMMAND);
        assert_eq!(Command::Tail.to_string(), surface_contract::TAIL_COMMAND);
        assert_eq!(
            Command::Management.to_string(),
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
