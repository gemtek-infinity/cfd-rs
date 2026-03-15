use crate::types::Command;

// --- Program identity ---

pub const PROGRAM_NAME: &str = "cloudflared";

// --- Command names (frozen Go baseline `commands()` in main.go) ---

pub const HELP_COMMAND: &str = "help";
pub const VERSION_COMMAND: &str = "version";
pub const UPDATE_COMMAND: &str = "update";
pub const TUNNEL_COMMAND: &str = "tunnel";
pub const LOGIN_COMMAND: &str = "login";
pub const PROXY_DNS_COMMAND: &str = "proxy-dns";
pub const ACCESS_COMMAND: &str = "access";
pub const FORWARD_COMMAND: &str = "forward";
pub const TAIL_COMMAND: &str = "tail";
pub const MANAGEMENT_COMMAND: &str = "management";
pub const SERVICE_COMMAND: &str = "service";
pub const VALIDATE_COMMAND: &str = "validate";
pub const RUN_COMMAND: &str = "run";
pub const SERVICE_MODE_LABEL: &str = "(service-mode)";

// --- Tunnel subcommand names ---

pub const TUNNEL_RUN: &str = "run";
pub const TUNNEL_CREATE: &str = "create";
pub const TUNNEL_LIST: &str = "list";
pub const TUNNEL_DELETE: &str = "delete";
pub const TUNNEL_CLEANUP: &str = "cleanup";
pub const TUNNEL_TOKEN: &str = "token";
pub const TUNNEL_INFO: &str = "info";
pub const TUNNEL_READY: &str = "ready";
pub const TUNNEL_DIAG: &str = "diag";
pub const TUNNEL_ROUTE: &str = "route";
pub const TUNNEL_VNET: &str = "vnet";
pub const TUNNEL_INGRESS: &str = "ingress";
pub const TUNNEL_LOGIN: &str = "login";
pub const TUNNEL_PROXY_DNS: &str = "proxy-dns";
pub const TUNNEL_DB_CONNECT: &str = "db-connect";

// --- Service subcommand names ---

pub const SERVICE_INSTALL: &str = "install";
pub const SERVICE_UNINSTALL: &str = "uninstall";

// --- Access subcommand names ---

pub const ACCESS_LOGIN: &str = "login";
pub const ACCESS_CURL: &str = "curl";
pub const ACCESS_TOKEN: &str = "token";
pub const ACCESS_TCP: &str = "tcp";
pub const ACCESS_RDP: &str = "rdp";
pub const ACCESS_SSH: &str = "ssh";
pub const ACCESS_SMB: &str = "smb";
pub const ACCESS_SSH_CONFIG: &str = "ssh-config";
pub const ACCESS_SSH_GEN: &str = "ssh-gen";

// --- Tail subcommand names ---

pub const TAIL_TOKEN: &str = "token";

// --- Management subcommand names ---

pub const MANAGEMENT_TOKEN: &str = "token";

// --- Route sub-subcommand names ---

pub const ROUTE_DNS: &str = "dns";
pub const ROUTE_LB: &str = "lb";
pub const ROUTE_IP: &str = "ip";

// --- Route IP sub-sub-subcommand names ---

pub const IP_ADD: &str = "add";
pub const IP_SHOW: &str = "show";
pub const IP_LIST: &str = "list";
pub const IP_DELETE: &str = "delete";
pub const IP_GET: &str = "get";

// --- Vnet sub-subcommand names ---

pub const VNET_ADD: &str = "add";
pub const VNET_LIST: &str = "list";
pub const VNET_DELETE: &str = "delete";
pub const VNET_UPDATE: &str = "update";

// --- Ingress sub-subcommand names ---

pub const INGRESS_VALIDATE: &str = "validate";
pub const INGRESS_RULE: &str = "rule";

// --- Flag names ---

pub const CONFIG_FLAG: &str = "--config";
pub const HELP_FLAG: &str = "--help";
pub const HELP_FLAG_SHORT: &str = "-h";
pub const VERSION_FLAG: &str = "--version";
pub const VERSION_FLAG_SHORT_LOWER: &str = "-v";
pub const VERSION_FLAG_SHORT_UPPER: &str = "-V";
pub const SHORT_FLAG: &str = "--short";
pub const SHORT_FLAG_SHORT: &str = "-s";

// --- Help text fragments (matching Go baseline urfave/cli output) ---

const APP_USAGE: &str = "Cloudflare's command-line tool and agent";
const APP_USAGE_TEXT: &str = "cloudflared [global options] [command] [command options]";

const APP_DESCRIPTION: &str = concat!(
    "cloudflared connects your machine or user identity to Cloudflare's global network.\n",
    "   You can use it to authenticate a session to reach an API behind Access, ",
    "route web traffic to this machine,\n",
    "   and configure access control.\n",
    "\n",
    "   See https://developers.cloudflare.com/cloudflare-one/connections/connect-apps ",
    "for more in-depth documentation.",
);

const CMD_UPDATE_USAGE: &str = "Update the agent if a new version exists";
const CMD_VERSION_USAGE: &str = "Print the version";
const CMD_TUNNEL_USAGE: &str = concat!(
    "Use Cloudflare Tunnel to expose private services to the Internet ",
    "or to Cloudflare connected private users.",
);
const CMD_PROXY_DNS_USAGE: &str = "dns-proxy feature is no longer supported";
const CMD_ACCESS_USAGE: &str = "access <subcommand>";
const CMD_TAIL_USAGE: &str = "Stream logs from a remote cloudflared";
#[allow(dead_code)] // Used when per-command help is implemented.
const CMD_MANAGEMENT_USAGE: &str = "Monitor cloudflared tunnels via management API";
const CMD_SERVICE_USAGE: &str = "Manages the cloudflared system service";
const CMD_HELP_USAGE: &str = "Shows a list of commands or help for one command";

// --- Transitional alpha ---

const CMD_VALIDATE_USAGE: &str = "Validate ingress configuration and report startup readiness";

// --- Error message templates ---

const USAGE_GUIDANCE_TEMPLATE: &str =
    "error: {message}\nRun `cloudflared help` for the admitted command surface.\n";
const CONFIG_ERROR_TEMPLATE: &str = "error: startup validation failed [{category}]: {error}\n";
const MISSING_FLAG_VALUE_TEMPLATE: &str = "missing value for {flag}";
const REPEATED_FLAG_TEMPLATE: &str = "{flag} may only be provided once";
const UNKNOWN_FLAG_TEMPLATE: &str = "unknown flag: {flag}";
const UNKNOWN_ARGUMENT_TEMPLATE: &str = "unknown command or argument: {value}";
const MULTIPLE_COMMANDS_TEMPLATE: &str = "multiple commands were provided: {existing} and {next}";
const VERSION_OUTPUT_TEMPLATE: &str = "{program} {version}\n";
const STUB_NOT_IMPLEMENTED_TEMPLATE: &str = "error: `cloudflared {command}` is not yet implemented in the \
                                             Rust rewrite.\nThis command exists in the Go baseline and will \
                                             be implemented in a future milestone.\n";

// --- Removed feature messages ---

pub const PROXY_DNS_REMOVED_MSG: &str = "dns-proxy feature is no longer supported\n";
pub const DB_CONNECT_REMOVED_MSG: &str = "error: the db-connect command has been removed.\n";
#[allow(dead_code)] // Used when classic tunnel detection is wired up.
pub const CLASSIC_TUNNEL_DEPRECATED_MSG: &str =
    "Classic tunnels have been deprecated, please use Named Tunnels. \
     (https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/install-and-setup/tunnel-guide/)\n";

// --- Public helpers ---

pub fn command_label(command: &Command) -> &'static str {
    match command {
        Command::ServiceMode => SERVICE_MODE_LABEL,
        Command::Help => HELP_COMMAND,
        Command::Version { .. } => VERSION_COMMAND,
        Command::Update => UPDATE_COMMAND,
        Command::Tunnel(_) => TUNNEL_COMMAND,
        Command::Login => LOGIN_COMMAND,
        Command::ProxyDns => PROXY_DNS_COMMAND,
        Command::Access(_) => ACCESS_COMMAND,
        Command::Tail(_) => TAIL_COMMAND,
        Command::Management(_) => MANAGEMENT_COMMAND,
        Command::Service(_) => SERVICE_COMMAND,
        Command::Validate => VALIDATE_COMMAND,
    }
}

pub fn is_help_token(token: &str) -> bool {
    matches!(token, HELP_FLAG | HELP_FLAG_SHORT | HELP_COMMAND)
}

pub fn is_version_token(token: &str) -> bool {
    matches!(
        token,
        VERSION_FLAG | VERSION_FLAG_SHORT_LOWER | VERSION_FLAG_SHORT_UPPER | VERSION_COMMAND
    )
}

/// Recognize all top-level command words from the frozen Go baseline.
pub fn parse_command_token(token: &str) -> Option<Command> {
    use super::types::*;
    match token {
        UPDATE_COMMAND => Some(Command::Update),
        TUNNEL_COMMAND => Some(Command::Tunnel(TunnelSubcommand::Bare)),
        LOGIN_COMMAND => Some(Command::Login),
        PROXY_DNS_COMMAND => Some(Command::ProxyDns),
        ACCESS_COMMAND | FORWARD_COMMAND => Some(Command::Access(AccessSubcommand::Bare)),
        TAIL_COMMAND => Some(Command::Tail(TailSubcommand::Bare)),
        MANAGEMENT_COMMAND => Some(Command::Management(ManagementSubcommand::Bare)),
        SERVICE_COMMAND => Some(Command::Service(ServiceAction::Install)),
        VALIDATE_COMMAND => Some(Command::Validate),
        RUN_COMMAND => Some(Command::Tunnel(TunnelSubcommand::Run)),
        _ => None,
    }
}

/// Parse a tunnel subcommand word.
pub fn parse_tunnel_subcommand(token: &str) -> Option<super::types::TunnelSubcommand> {
    use super::types::*;
    match token {
        TUNNEL_RUN => Some(TunnelSubcommand::Run),
        TUNNEL_CREATE => Some(TunnelSubcommand::Create),
        TUNNEL_LIST => Some(TunnelSubcommand::List),
        TUNNEL_DELETE => Some(TunnelSubcommand::Delete),
        TUNNEL_CLEANUP => Some(TunnelSubcommand::Cleanup),
        TUNNEL_TOKEN => Some(TunnelSubcommand::Token),
        TUNNEL_INFO => Some(TunnelSubcommand::Info),
        TUNNEL_READY => Some(TunnelSubcommand::Ready),
        TUNNEL_DIAG => Some(TunnelSubcommand::Diag),
        TUNNEL_ROUTE => Some(TunnelSubcommand::Route(RouteSubcommand::Bare)),
        TUNNEL_VNET => Some(TunnelSubcommand::Vnet(VnetSubcommand::Bare)),
        TUNNEL_INGRESS => Some(TunnelSubcommand::Ingress(IngressSubcommand::Bare)),
        TUNNEL_LOGIN => Some(TunnelSubcommand::Login),
        TUNNEL_PROXY_DNS => Some(TunnelSubcommand::ProxyDns),
        TUNNEL_DB_CONNECT => Some(TunnelSubcommand::DbConnect),
        _ => None,
    }
}

/// Parse an access subcommand word.
pub fn parse_access_subcommand(token: &str) -> Option<super::types::AccessSubcommand> {
    use super::types::AccessSubcommand;
    match token {
        ACCESS_LOGIN => Some(AccessSubcommand::Login),
        ACCESS_CURL => Some(AccessSubcommand::Curl),
        ACCESS_TOKEN => Some(AccessSubcommand::Token),
        ACCESS_TCP | ACCESS_RDP | ACCESS_SSH | ACCESS_SMB => Some(AccessSubcommand::Tcp),
        ACCESS_SSH_CONFIG => Some(AccessSubcommand::SshConfig),
        ACCESS_SSH_GEN => Some(AccessSubcommand::SshGen),
        _ => None,
    }
}

/// Parse a tail subcommand word.
pub fn parse_tail_subcommand(token: &str) -> Option<super::types::TailSubcommand> {
    use super::types::TailSubcommand;
    match token {
        TAIL_TOKEN => Some(TailSubcommand::Token),
        _ => None,
    }
}

/// Parse a management subcommand word.
pub fn parse_management_subcommand(token: &str) -> Option<super::types::ManagementSubcommand> {
    use super::types::ManagementSubcommand;
    match token {
        MANAGEMENT_TOKEN => Some(ManagementSubcommand::Token),
        _ => None,
    }
}

/// Parse a route sub-subcommand word.
pub fn parse_route_subcommand(token: &str) -> Option<super::types::RouteSubcommand> {
    use super::types::RouteSubcommand;
    match token {
        ROUTE_DNS => Some(RouteSubcommand::Dns),
        ROUTE_LB => Some(RouteSubcommand::Lb),
        ROUTE_IP => Some(RouteSubcommand::Ip(super::types::IpRouteSubcommand::Bare)),
        _ => None,
    }
}

/// Parse a route-ip sub-sub-subcommand word.
pub fn parse_ip_route_subcommand(token: &str) -> Option<super::types::IpRouteSubcommand> {
    use super::types::IpRouteSubcommand;
    match token {
        IP_ADD => Some(IpRouteSubcommand::Add),
        IP_SHOW | IP_LIST => Some(IpRouteSubcommand::Show),
        IP_DELETE => Some(IpRouteSubcommand::Delete),
        IP_GET => Some(IpRouteSubcommand::Get),
        _ => None,
    }
}

/// Parse a vnet sub-subcommand word.
pub fn parse_vnet_subcommand(token: &str) -> Option<super::types::VnetSubcommand> {
    use super::types::VnetSubcommand;
    match token {
        VNET_ADD => Some(VnetSubcommand::Add),
        VNET_LIST => Some(VnetSubcommand::List),
        VNET_DELETE => Some(VnetSubcommand::Delete),
        VNET_UPDATE => Some(VnetSubcommand::Update),
        _ => None,
    }
}

/// Parse an ingress sub-subcommand word.
pub fn parse_ingress_subcommand(token: &str) -> Option<super::types::IngressSubcommand> {
    use super::types::IngressSubcommand;
    match token {
        INGRESS_VALIDATE => Some(IngressSubcommand::Validate),
        INGRESS_RULE => Some(IngressSubcommand::Rule),
        _ => None,
    }
}

/// Parse a service subcommand word.
pub fn parse_service_subcommand(token: &str) -> Option<super::types::ServiceAction> {
    use super::types::ServiceAction;
    match token {
        SERVICE_INSTALL => Some(ServiceAction::Install),
        SERVICE_UNINSTALL => Some(ServiceAction::Uninstall),
        _ => None,
    }
}

pub fn missing_flag_value_message(flag: &str) -> String {
    MISSING_FLAG_VALUE_TEMPLATE.replace("{flag}", flag)
}

pub fn repeated_flag_message(flag: &str) -> String {
    REPEATED_FLAG_TEMPLATE.replace("{flag}", flag)
}

pub fn unknown_flag_message(flag: &str) -> String {
    UNKNOWN_FLAG_TEMPLATE.replace("{flag}", flag)
}

pub fn unknown_argument_message(value: &str) -> String {
    UNKNOWN_ARGUMENT_TEMPLATE.replace("{value}", value)
}

pub fn multiple_commands_message(existing: &Command, next: &Command) -> String {
    MULTIPLE_COMMANDS_TEMPLATE
        .replace("{existing}", command_label(existing))
        .replace("{next}", command_label(next))
}

pub fn usage_guidance(message: &str) -> String {
    USAGE_GUIDANCE_TEMPLATE.replace("{message}", message)
}

pub fn config_error_message(category: &str, error: &str) -> String {
    CONFIG_ERROR_TEMPLATE
        .replace("{category}", category)
        .replace("{error}", error)
}

pub fn render_version_output(program_name: &str) -> String {
    VERSION_OUTPUT_TEMPLATE
        .replace("{program}", program_name)
        .replace("{version}", env!("CARGO_PKG_VERSION"))
}

pub fn render_short_version() -> String {
    format!("{}\n", env!("CARGO_PKG_VERSION"))
}

pub fn is_short_version_token(token: &str) -> bool {
    matches!(token, SHORT_FLAG | SHORT_FLAG_SHORT)
}

pub fn stub_not_implemented(command: &str) -> String {
    STUB_NOT_IMPLEMENTED_TEMPLATE.replace("{command}", command)
}

/// Render root help text matching Go baseline `cloudflared --help` layout.
pub fn render_help_text(program_name: &str) -> String {
    let mut text = String::with_capacity(2048);

    // NAME section
    text.push_str("NAME:\n");
    text.push_str(&format!("   {program_name} - {APP_USAGE}\n\n"));

    // USAGE section
    text.push_str("USAGE:\n");
    text.push_str(&format!("   {APP_USAGE_TEXT}\n\n"));

    // VERSION section
    text.push_str("VERSION:\n");
    text.push_str(&format!("   {}\n\n", env!("CARGO_PKG_VERSION")));

    // DESCRIPTION section
    text.push_str("DESCRIPTION:\n");
    text.push_str(&format!("   {APP_DESCRIPTION}\n\n"));

    // COMMANDS section
    text.push_str("COMMANDS:\n");
    text.push_str(&format!("   {UPDATE_COMMAND:<14}{CMD_UPDATE_USAGE}\n"));
    text.push_str(&format!("   {VERSION_COMMAND:<14}{CMD_VERSION_USAGE}\n"));
    text.push_str(&format!("   {TUNNEL_COMMAND:<14}{CMD_TUNNEL_USAGE}\n"));
    text.push_str(&format!("   {PROXY_DNS_COMMAND:<14}{CMD_PROXY_DNS_USAGE}\n"));
    text.push_str(&format!("   {ACCESS_COMMAND:<14}{CMD_ACCESS_USAGE}\n"));
    text.push_str(&format!("   {TAIL_COMMAND:<14}{CMD_TAIL_USAGE}\n"));
    text.push_str(&format!("   {SERVICE_COMMAND:<14}{CMD_SERVICE_USAGE}\n"));
    text.push_str(&format!("   {VALIDATE_COMMAND:<14}{CMD_VALIDATE_USAGE}\n"));
    text.push_str(&format!("   {HELP_COMMAND}, h{:<8}{CMD_HELP_USAGE}\n\n", ""));

    // GLOBAL OPTIONS section
    text.push_str("GLOBAL OPTIONS:\n");
    text.push_str(&format!(
        "   {CONFIG_FLAG} value          Path to a configuration file (default: search standard paths)\n"
    ));
    text.push_str(
        "   --credentials-file value   Filepath at which to read/write the tunnel credentials (env: \
         TUNNEL_CRED_FILE)\n",
    );
    text.push_str(
        "   --token value              Token provided to associate this connector to a specific tunnel \
         (env: TUNNEL_TOKEN)\n",
    );
    text.push_str(
        "   --origincert value         Path to the certificate for authenticating with Cloudflare (env: \
         TUNNEL_ORIGIN_CERT)\n",
    );
    text.push_str("   --loglevel value           Application log level (env: TUNNEL_LOGLEVEL)\n");
    text.push_str("   --logfile value            Save application log to this file\n");
    text.push_str("   --log-directory value      Save application logs to this directory\n");
    text.push_str(&format!(
        "   {HELP_FLAG}, {HELP_FLAG_SHORT}                 show help\n"
    ));
    text.push_str(&format!(
        "   {VERSION_FLAG}, {VERSION_FLAG_SHORT_LOWER}, {VERSION_FLAG_SHORT_UPPER}          \
         {CMD_VERSION_USAGE}\n"
    ));

    text
}
