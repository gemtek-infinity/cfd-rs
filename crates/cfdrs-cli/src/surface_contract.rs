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

// --- Error message templates ---

const USAGE_GUIDANCE_TEMPLATE: &str =
    "error: {message}\nRun `cloudflared help` for the admitted command surface.\n";
const CONFIG_ERROR_TEMPLATE: &str = "error: startup validation failed [{category}]: {error}\n";
const MISSING_FLAG_VALUE_TEMPLATE: &str = "missing value for {flag}";
const REPEATED_FLAG_TEMPLATE: &str = "{flag} may only be provided once";
const UNKNOWN_FLAG_TEMPLATE: &str = "unknown flag: {flag}";
const UNKNOWN_ARGUMENT_TEMPLATE: &str = "unknown command or argument: {value}";
const MULTIPLE_COMMANDS_TEMPLATE: &str = "multiple commands were provided: {existing} and {next}";
const STUB_NOT_IMPLEMENTED_TEMPLATE: &str = "error: `cloudflared {command}` is not yet implemented in the \
                                             Rust rewrite.\nThis command exists in the Go baseline and will \
                                             be implemented in a future milestone.\n";

/// Build time injected at compile time via `CFDRS_BUILD_TIME`, or `"unknown"`
/// when not set.
///
/// Go baseline uses linker `-ldflags` to set `BuildTime`; the default is
/// `"unknown"`.
const BUILD_TIME: &str = match option_env!("CFDRS_BUILD_TIME") {
    Some(t) => t,
    None => "unknown",
};

/// Build type injected at compile time via `CFDRS_BUILD_TYPE`, or `""` when
/// not set.
///
/// Go baseline: `BuildType` defaults to `""`.  When non-empty,
/// `GetBuildTypeMsg()` returns `" with {BuildType}"` (e.g. `" with FIPS"`).
const BUILD_TYPE: &str = match option_env!("CFDRS_BUILD_TYPE") {
    Some(t) => t,
    None => "",
};

/// Returns Go-baseline `GetBuildTypeMsg()` equivalent: empty when
/// `BUILD_TYPE` is empty, `" with {BUILD_TYPE}"` otherwise.
fn build_type_msg() -> String {
    if BUILD_TYPE.is_empty() {
        String::new()
    } else {
        format!(" with {BUILD_TYPE}")
    }
}

// --- Removed feature messages ---

pub const PROXY_DNS_REMOVED_MSG: &str = "dns-proxy feature is no longer supported\n";

/// Go baseline: `log.Error().Msg(...)` in `proxydns/cmd.go` emits this
/// structured log line before returning the short error.  Includes version and
/// migration URL.
pub const PROXY_DNS_REMOVED_LOG_MSG: &str =
    "DNS Proxy is no longer supported since version 2026.2.0 \
     (https://developers.cloudflare.com/changelog/2025-11-11-cloudflared-proxy-dns/). \
     As an alternative consider using \
     https://developers.cloudflare.com/1.1.1.1/encryption/dns-over-https/dns-over-https-client/";

pub const DB_CONNECT_REMOVED_MSG: &str = "db-connect command is no longer supported by cloudflared. Consult \
                                          Cloudflare Tunnel documentation for possible alternative \
                                          solutions.\n";
pub const CLASSIC_TUNNEL_DEPRECATED_MSG: &str =
    "Classic tunnels have been deprecated, please use Named Tunnels. \
     (https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/install-and-setup/tunnel-guide/)\n";

// --- Tunnel run validation messages (CLI-032) ---

/// Go baseline: `cliutil.UsageError(...)` in runCommand(), subcommands.go line
/// 754. NArg() > 1 → "accepts only one argument".
pub const TUNNEL_RUN_NARG_ERROR_MSG: &str =
    "\"cloudflared tunnel run\" accepts only one argument, the ID or name of the tunnel to run.";

/// Go baseline: `cliutil.UsageError(...)` in runCommand(), subcommands.go line
/// 778. ParseToken(tokenStr) fails → "Provided Tunnel token is not valid."
pub const TUNNEL_TOKEN_INVALID_MSG: &str = "Provided Tunnel token is not valid.";

/// Go baseline: `cliutil.UsageError(...)` in runCommand(), subcommands.go line
/// 769. os.ReadFile(tokenFile) fails → "Failed to read token file: <err>".
pub const TUNNEL_TOKEN_FILE_READ_ERROR_PREFIX: &str = "Failed to read token file: ";

/// Go baseline: `cliutil.UsageError(...)` in runCommand(), subcommands.go line
/// 786. No token, no positional arg, no config tunnel ID.
pub const TUNNEL_RUN_IDENTITY_ERROR_MSG: &str = "\"cloudflared tunnel run\" requires the ID or name of the \
                                                 tunnel to run as the last command line argument or in the \
                                                 configuration file.";

// --- Tunnel run hostname warning (CLI-027) ---

/// Go baseline: `sc.log.Warn().Msg(...)` in runCommand(), subcommands.go line
/// 757. Hostname set but Named Tunnel is configured.
pub const TUNNEL_RUN_HOSTNAME_WARNING_MSG: &str =
    "The property `hostname` in your configuration is ignored because you configured a Named Tunnel in the \
     property `tunnel` to run. Make sure to provision the routing (e.g. via `cloudflared tunnel route \
     dns/lb`) or else your origin will not be reachable. You should remove the `hostname` property to avoid \
     this warning.";

/// Go baseline: WithErrorHandler in cliutil/errors.go appends
/// `\nSee 'cloudflared <command> --help'.` to every UsageError message.
pub fn tunnel_run_usage_error(message: &str) -> String {
    format!("{message}\nSee 'cloudflared tunnel run --help'.\n")
}

/// Go baseline: `tunnelCmdErrorMessage` in cmd/cloudflared/tunnel/cmd.go
pub const TUNNEL_CMD_ERROR_MSG: &str = "\
You did not specify any valid additional argument to the cloudflared tunnel command.

If you are trying to run a Quick Tunnel then you need to explicitly pass the --url flag.
Eg. cloudflared tunnel --url localhost:8080/.

Please note that Quick Tunnels are meant to be ephemeral and should only be used for testing purposes.
For production usage, we recommend creating Named Tunnels. \
(https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/install-and-setup/tunnel-guide/)
";

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
    // Go baseline: `{Version} (built {BuildTime}{GetBuildTypeMsg()})`
    format!(
        "{program_name} version {} (built {BUILD_TIME}{})\n",
        env!("CARGO_PKG_VERSION"),
        build_type_msg(),
    )
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

    // VERSION section — Go baseline includes build time and build type here too
    text.push_str("VERSION:\n");
    text.push_str(&format!(
        "   {} (built {}{})\n\n",
        env!("CARGO_PKG_VERSION"),
        BUILD_TIME,
        build_type_msg(),
    ));

    // DESCRIPTION section
    text.push_str("DESCRIPTION:\n");
    text.push_str(&format!("   {APP_DESCRIPTION}\n\n"));

    // COMMANDS section — Go baseline groups commands with category headings
    // via urfave/cli VisibleCategories.  Uncategorized commands first (in
    // insertion order), then named categories sorted alphabetically.
    // Column alignment: all usage text starts at the same column.
    // Uncategorized (3-space indent): name padded to 19 chars → col 22.
    // Categorized   (5-space indent): name padded to 17 chars → col 22.
    text.push_str("COMMANDS:\n");
    text.push_str(&format!("   {UPDATE_COMMAND:<19}{CMD_UPDATE_USAGE}\n"));
    text.push_str(&format!("   {VERSION_COMMAND:<19}{CMD_VERSION_USAGE}\n"));
    text.push_str(&format!("   {PROXY_DNS_COMMAND:<19}{CMD_PROXY_DNS_USAGE}\n"));
    text.push_str(&format!("   {TAIL_COMMAND:<19}{CMD_TAIL_USAGE}\n"));
    text.push_str(&format!("   {SERVICE_COMMAND:<19}{CMD_SERVICE_USAGE}\n"));
    let help_name = format!("{HELP_COMMAND}, h");
    text.push_str(&format!("   {help_name:<19}{CMD_HELP_USAGE}\n"));
    text.push_str("   Access:\n");
    let access_name = format!("{ACCESS_COMMAND}, {FORWARD_COMMAND}");
    text.push_str(&format!("     {access_name:<17}{CMD_ACCESS_USAGE}\n"));
    text.push_str("   Tunnel:\n");
    text.push_str(&format!("     {TUNNEL_COMMAND:<17}{CMD_TUNNEL_USAGE}\n\n"));

    // GLOBAL OPTIONS section
    text.push_str("GLOBAL OPTIONS:\n");
    text.push_str(&format!(
        "   {CONFIG_FLAG} value          Path to a configuration file (default: search standard paths)\n"
    ));
    text.push_str(
        "   --credentials-file value, --cred-file value  Filepath at which to read/write the tunnel \
         credentials [$TUNNEL_CRED_FILE]\n",
    );
    text.push_str(
        "   --token value              Token provided to associate this connector to a specific tunnel \
         [$TUNNEL_TOKEN]\n",
    );
    text.push_str(
        "   --origincert value         Path to the certificate for authenticating with Cloudflare \
         [$TUNNEL_ORIGIN_CERT]\n",
    );
    text.push_str("   --loglevel value           Application log level [$TUNNEL_LOGLEVEL]\n");
    text.push_str("   --logfile value            Save application log to this file [$TUNNEL_LOGFILE]\n");
    text.push_str(
        "   --log-directory value      Save application logs to this directory [$TUNNEL_LOGDIRECTORY]\n",
    );
    text.push_str(&format!(
        "   {HELP_FLAG}, {HELP_FLAG_SHORT}                 show help\n"
    ));
    text.push_str(&format!(
        "   {VERSION_FLAG}, {VERSION_FLAG_SHORT_LOWER}, {VERSION_FLAG_SHORT_UPPER}          \
         {CMD_VERSION_USAGE}\n\n"
    ));

    // COPYRIGHT section — matches Go baseline
    text.push_str("COPYRIGHT:\n");
    text.push_str(
        "   (c) 2026 Cloudflare Inc.\n   \
         Your installation of cloudflared software constitutes a symbol of your signature indicating that \
         you accept\n   \
         the terms of the Apache License Version 2.0 \
         (https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/license),\n   \
         Terms (https://www.cloudflare.com/terms/) and Privacy Policy \
         (https://www.cloudflare.com/privacypolicy/).\n",
    );

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- CLI-005: version command format parity ---

    #[test]
    fn version_output_matches_go_baseline_format() {
        let output = render_version_output(PROGRAM_NAME);

        // Go baseline: `cloudflared version DEV (built unknown)`
        // urfave/cli prints: `{app.Name} version {app.Version}`
        // where app.Version = `{Version} (built {BuildTime}{BuildTypeMsg})`
        assert!(
            output.starts_with("cloudflared version "),
            "must start with 'cloudflared version ': {output:?}"
        );
        assert!(output.contains("(built "), "must contain '(built ': {output:?}");
        assert!(output.ends_with(")\n"), "must end with ')\\n': {output:?}");
    }

    #[test]
    fn version_output_contains_cargo_pkg_version() {
        let output = render_version_output(PROGRAM_NAME);
        let version = env!("CARGO_PKG_VERSION");

        assert!(
            output.contains(version),
            "must contain CARGO_PKG_VERSION '{version}': {output:?}"
        );
    }

    #[test]
    fn version_output_default_build_time_is_unknown() {
        // When CFDRS_BUILD_TIME is not set (default), BUILD_TIME is "unknown"
        // matching Go's `BuildTime = "unknown"` default.
        let output = render_version_output(PROGRAM_NAME);

        assert!(
            output.contains("(built unknown)"),
            "default build time must be 'unknown': {output:?}"
        );
    }

    #[test]
    fn build_type_msg_empty_when_not_set() {
        // Go baseline: `GetBuildTypeMsg()` returns "" when BuildType=="".
        // CFDRS_BUILD_TYPE is not set during tests, so build_type_msg() returns "".
        assert_eq!(build_type_msg(), "");
    }

    #[test]
    fn build_type_constant_matches_go_default() {
        // Go baseline: `BuildType = ""` — empty by default, set to "FIPS"
        // by the build system for FIPS builds.  Rust equivalent: CFDRS_BUILD_TYPE
        // env var at compile time.
        assert!(
            BUILD_TYPE.is_empty(),
            "BUILD_TYPE must be empty in default test builds"
        );
    }

    #[test]
    fn short_version_outputs_version_number_only() {
        let output = render_short_version();
        let version = env!("CARGO_PKG_VERSION");

        assert_eq!(output, format!("{version}\n"));
    }

    #[test]
    fn short_version_tokens_match_go_baseline() {
        assert!(is_short_version_token("--short"));
        assert!(is_short_version_token("-s"));
        assert!(!is_short_version_token("--version"));
        assert!(!is_short_version_token("-v"));
    }

    #[test]
    fn version_flag_constants_match_go_baseline() {
        // Go: `cli.VersionFlag` has Name:"version", Aliases:["v","V"]
        assert_eq!(VERSION_FLAG, "--version");
        assert_eq!(VERSION_FLAG_SHORT_LOWER, "-v");
        assert_eq!(VERSION_FLAG_SHORT_UPPER, "-V");
    }

    // --- CLI-002: help text format parity ---

    #[test]
    fn help_text_contains_all_go_baseline_sections() {
        let help = render_help_text(PROGRAM_NAME);

        assert!(help.contains("NAME:"), "missing NAME section");
        assert!(help.contains("USAGE:"), "missing USAGE section");
        assert!(help.contains("VERSION:"), "missing VERSION section");
        assert!(help.contains("DESCRIPTION:"), "missing DESCRIPTION section");
        assert!(help.contains("COMMANDS:"), "missing COMMANDS section");
        assert!(help.contains("GLOBAL OPTIONS:"), "missing GLOBAL OPTIONS section");
        assert!(help.contains("COPYRIGHT:"), "missing COPYRIGHT section");
    }

    #[test]
    fn help_text_has_category_headings() {
        let help = render_help_text(PROGRAM_NAME);

        assert!(help.contains("   Access:\n"), "missing Access: category heading");
        assert!(help.contains("   Tunnel:\n"), "missing Tunnel: category heading");
    }

    #[test]
    fn help_text_lists_forward_alias() {
        let help = render_help_text(PROGRAM_NAME);

        // Go baseline: `access, forward  access <subcommand>`
        assert!(
            help.contains("access, forward"),
            "missing forward alias next to access"
        );
    }

    #[test]
    fn help_text_version_section_includes_build_time() {
        let help = render_help_text(PROGRAM_NAME);

        assert!(
            help.contains("(built "),
            "VERSION section should include build time"
        );
    }

    #[test]
    fn help_text_copyright_section_matches_go_baseline() {
        let help = render_help_text(PROGRAM_NAME);

        assert!(
            help.contains("Cloudflare Inc."),
            "missing Cloudflare Inc. in COPYRIGHT"
        );
        assert!(
            help.contains("Apache License Version 2.0"),
            "missing license name in COPYRIGHT"
        );
    }

    #[test]
    fn help_text_credentials_file_shows_alias() {
        let help = render_help_text(PROGRAM_NAME);

        // Go baseline: `--credentials-file value, --cred-file value`
        assert!(help.contains("--cred-file"), "help should show --cred-file alias");
    }

    #[test]
    fn help_text_lists_all_go_baseline_commands() {
        let help = render_help_text(PROGRAM_NAME);

        assert!(help.contains(UPDATE_COMMAND), "missing update");
        assert!(help.contains(VERSION_COMMAND), "missing version");
        assert!(help.contains(TUNNEL_COMMAND), "missing tunnel");
        assert!(help.contains(PROXY_DNS_COMMAND), "missing proxy-dns");
        assert!(help.contains(ACCESS_COMMAND), "missing access");
        assert!(help.contains(TAIL_COMMAND), "missing tail");
        assert!(help.contains(SERVICE_COMMAND), "missing service");
        assert!(help.contains(HELP_COMMAND), "missing help");
    }

    #[test]
    fn help_text_contains_go_baseline_global_flags() {
        let help = render_help_text(PROGRAM_NAME);

        assert!(help.contains("--config"), "missing --config");
        assert!(help.contains("--credentials-file"), "missing --credentials-file");
        assert!(help.contains("--token"), "missing --token");
        assert!(help.contains("--origincert"), "missing --origincert");
        assert!(help.contains("--loglevel"), "missing --loglevel");
        assert!(help.contains("--logfile"), "missing --logfile");
        assert!(help.contains("--log-directory"), "missing --log-directory");
        assert!(help.contains("--help"), "missing --help");
        assert!(help.contains("--version"), "missing --version");
    }

    #[test]
    fn help_text_contains_env_var_annotations() {
        let help = render_help_text(PROGRAM_NAME);

        assert!(
            help.contains("TUNNEL_CRED_FILE"),
            "missing TUNNEL_CRED_FILE env annotation"
        );
        assert!(
            help.contains("TUNNEL_TOKEN"),
            "missing TUNNEL_TOKEN env annotation"
        );
        assert!(
            help.contains("TUNNEL_ORIGIN_CERT"),
            "missing TUNNEL_ORIGIN_CERT env annotation"
        );
        assert!(
            help.contains("TUNNEL_LOGLEVEL"),
            "missing TUNNEL_LOGLEVEL env annotation"
        );
        assert!(
            help.contains("TUNNEL_LOGFILE"),
            "missing TUNNEL_LOGFILE env annotation"
        );
        assert!(
            help.contains("TUNNEL_LOGDIRECTORY"),
            "missing TUNNEL_LOGDIRECTORY env annotation"
        );
    }

    #[test]
    fn program_name_matches_go_baseline() {
        assert_eq!(PROGRAM_NAME, "cloudflared");
    }

    // --- CLI-029: help formatting contract ---

    #[test]
    fn help_text_commands_section_snapshot() {
        // Full snapshot of the COMMANDS section matching Go urfave/cli
        // VisibleCategories ordering: uncategorized first (insertion order),
        // then named categories sorted alphabetically (Access, Tunnel).
        // management is Hidden (Go: `Hidden: true`), not shown in root help.
        let help = render_help_text(PROGRAM_NAME);

        let commands_start = help.find("COMMANDS:\n").expect("missing COMMANDS section");
        let commands_end = help
            .find("\nGLOBAL OPTIONS:\n")
            .expect("missing GLOBAL OPTIONS section");
        let commands_section = &help[commands_start..commands_end + 1];

        let expected = "\
COMMANDS:\n\
\x20\x20\x20update             Update the agent if a new version exists\n\
\x20\x20\x20version            Print the version\n\
\x20\x20\x20proxy-dns          dns-proxy feature is no longer supported\n\
\x20\x20\x20tail               Stream logs from a remote cloudflared\n\
\x20\x20\x20service            Manages the cloudflared system service\n\
\x20\x20\x20help, h            Shows a list of commands or help for one command\n\
\x20\x20\x20Access:\n\
\x20\x20\x20\x20\x20access, forward  access <subcommand>\n\
\x20\x20\x20Tunnel:\n\
\x20\x20\x20\x20\x20tunnel           Use Cloudflare Tunnel to expose private services to the Internet or to Cloudflare connected private users.\n\
\n";

        assert_eq!(
            commands_section, expected,
            "COMMANDS section snapshot mismatch.\nGot:\n{commands_section}\nExpected:\n{expected}"
        );
    }

    #[test]
    fn help_text_management_not_shown() {
        // Go baseline: management command has `Hidden: true`, not visible
        // in root help output.
        let help = render_help_text(PROGRAM_NAME);

        assert!(
            !help.contains("management"),
            "management must not appear in root help (Hidden in Go baseline)"
        );
    }

    #[test]
    fn help_text_commands_column_alignment() {
        // All command names (with indent) must produce usage text starting
        // at the same column position.
        let help = render_help_text(PROGRAM_NAME);

        let commands_start = help.find("COMMANDS:\n").expect("COMMANDS section");
        let commands_end = help.find("\nGLOBAL OPTIONS:\n").expect("GLOBAL OPTIONS section");
        let commands_section = &help[commands_start..commands_end];

        // Collect the column where usage text starts for each command line
        // (skip section headers like "COMMANDS:", "   Access:", "   Tunnel:")
        let mut usage_columns = Vec::new();
        for line in commands_section.lines() {
            // Skip heading lines and category headers
            if line.starts_with("COMMANDS:") || line.trim().ends_with(':') || line.is_empty() {
                continue;
            }
            // Find the position of the first non-space character after the
            // name portion. Command names end where trailing spaces begin
            // before the usage text.
            let trimmed = line.trim_start();
            if trimmed.is_empty() {
                continue;
            }
            // Find the column where usage text starts by looking for 2+ spaces
            // after the command name
            if trimmed.contains("  ") {
                let indent = line.len() - trimmed.len();
                let after_name_gap = trimmed.find("  ").expect("multi-space gap");
                let usage_col = indent
                    + after_name_gap
                    + (trimmed[after_name_gap..].len() - trimmed[after_name_gap..].trim_start().len());
                usage_columns.push(usage_col);
            }
        }

        assert!(!usage_columns.is_empty(), "should have found command lines");
        let first = usage_columns[0];
        for (i, col) in usage_columns.iter().enumerate() {
            assert_eq!(
                *col, first,
                "command line {i} usage starts at column {col}, expected {first}"
            );
        }
    }

    // --- Removed feature messages ---

    #[test]
    fn proxy_dns_removed_message_matches_go_baseline() {
        assert!(PROXY_DNS_REMOVED_MSG.contains("dns-proxy feature is no longer supported"));
    }

    #[test]
    fn proxy_dns_removed_log_message_matches_go_baseline() {
        // Go: log.Error().Msg("DNS Proxy is no longer supported since version 2026.2.0
        // ...")
        assert!(
            PROXY_DNS_REMOVED_LOG_MSG.contains("DNS Proxy is no longer supported since version 2026.2.0")
        );
        assert!(PROXY_DNS_REMOVED_LOG_MSG.contains("cloudflared-proxy-dns"));
        assert!(PROXY_DNS_REMOVED_LOG_MSG.contains("dns-over-https-client"));
    }

    #[test]
    fn db_connect_removed_message_matches_go_baseline() {
        // Go: cliutil.RemovedCommand("db-connect") produces exact text
        assert!(DB_CONNECT_REMOVED_MSG.contains("db-connect command is no longer supported"));
        assert!(DB_CONNECT_REMOVED_MSG.contains("Consult Cloudflare Tunnel documentation"));
    }

    #[test]
    fn classic_tunnel_deprecated_message_matches_go_baseline() {
        assert!(CLASSIC_TUNNEL_DEPRECATED_MSG.contains("Classic tunnels have been deprecated"));
        assert!(CLASSIC_TUNNEL_DEPRECATED_MSG.contains("Named Tunnels"));
    }

    #[test]
    fn tunnel_cmd_error_message_matches_go_baseline() {
        assert!(TUNNEL_CMD_ERROR_MSG.contains("You did not specify any valid additional argument"));
        assert!(TUNNEL_CMD_ERROR_MSG.contains("--url"));
        assert!(TUNNEL_CMD_ERROR_MSG.contains("Quick Tunnels"));
        assert!(TUNNEL_CMD_ERROR_MSG.contains("Named Tunnels"));
    }

    // --- CLI-032: tunnel run validation messages ---

    #[test]
    fn tunnel_run_narg_error_matches_go_baseline() {
        // Go: cliutil.UsageError(`"cloudflared tunnel run" accepts only one argument,
        // ...`)
        assert!(TUNNEL_RUN_NARG_ERROR_MSG.contains("accepts only one argument"));
        assert!(TUNNEL_RUN_NARG_ERROR_MSG.contains("ID or name"));
    }

    #[test]
    fn tunnel_token_invalid_matches_go_baseline() {
        // Go: cliutil.UsageError("Provided Tunnel token is not valid.")
        assert_eq!(TUNNEL_TOKEN_INVALID_MSG, "Provided Tunnel token is not valid.");
    }

    #[test]
    fn tunnel_run_identity_error_matches_go_baseline() {
        // Go: cliutil.UsageError(`"cloudflared tunnel run" requires the ID or name
        // ...`)
        assert!(TUNNEL_RUN_IDENTITY_ERROR_MSG.contains("requires the ID or name"));
        assert!(TUNNEL_RUN_IDENTITY_ERROR_MSG.contains("last command line argument"));
        assert!(TUNNEL_RUN_IDENTITY_ERROR_MSG.contains("configuration file"));
    }

    #[test]
    fn tunnel_run_usage_error_appends_help_suffix() {
        // Go: WithErrorHandler appends "\nSee 'cloudflared tunnel run --help'."
        let msg = tunnel_run_usage_error("test error");
        assert!(msg.contains("test error"));
        assert!(msg.contains("See 'cloudflared tunnel run --help'."));
    }

    // --- CLI-027: tunnel run hostname warning ---

    #[test]
    fn tunnel_run_hostname_warning_matches_go_baseline() {
        // Go: sc.log.Warn().Msg("The property `hostname` in your configuration is
        // ignored ...")
        assert!(TUNNEL_RUN_HOSTNAME_WARNING_MSG.contains("hostname"));
        assert!(TUNNEL_RUN_HOSTNAME_WARNING_MSG.contains("Named Tunnel"));
        assert!(TUNNEL_RUN_HOSTNAME_WARNING_MSG.contains("provision the routing"));
    }
}
