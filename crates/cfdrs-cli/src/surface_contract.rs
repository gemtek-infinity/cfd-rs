use crate::types::Command;

pub const PROGRAM_NAME: &str = "cloudflared";
pub const CONFIG_FLAG: &str = "--config";
pub const HELP_FLAG: &str = "--help";
pub const HELP_FLAG_SHORT: &str = "-h";
pub const HELP_COMMAND: &str = "help";
pub const VERSION_FLAG: &str = "--version";
pub const VERSION_FLAG_SHORT_LOWER: &str = "-v";
pub const VERSION_FLAG_SHORT_UPPER: &str = "-V";
pub const VERSION_COMMAND: &str = "version";
pub const VALIDATE_COMMAND: &str = "validate";
pub const RUN_COMMAND: &str = "run";

const HELP_DESCRIPTION: &str = "Linux production-alpha QUIC tunnel core with wire/protocol boundary, \
                                Pingora proxy seam, and narrow operability reporting";
const HELP_USAGE_HEADING: &str = "Usage:";
const HELP_COMMANDS_HEADING: &str = "Admitted commands:";
const HELP_FLAGS_HEADING: &str = "Admitted flags and defaults:";
const HELP_ENV_HEADING: &str = "Admitted environment:";
const HELP_OPERABILITY_HEADING: &str = "Admitted operability surface:";
const HELP_DEFERRED_HEADING: &str = "Deferred beyond current phase:";
const CONFIG_PATH_METAVAR: &str = "FILEPATH";
const HOME_ENV: &str = "HOME";
const HELP_VALIDATE_DESCRIPTION: &str =
    "Resolve config, load YAML, normalize ingress, and report startup readiness.";
const HELP_RUN_DESCRIPTION: &str =
    "Enter the runtime-owned QUIC transport core with wire/protocol boundary\n            and Pingora proxy \
     seam.\n            Emits narrow lifecycle, readiness, and failure visibility for the\n            \
     admitted alpha role. The admitted origin path is http_status only.\n            Broader origin support \
     and general proxy completeness remain later\n            slices.";
const HELP_VERSION_DESCRIPTION: &str = "Print the workspace version.";
const HELP_HELP_DESCRIPTION: &str = "Print this help text.";
const HELP_CONFIG_FLAG_DESCRIPTION: &str = "Use an explicit YAML config path.";
const HELP_DISCOVERY_DESCRIPTION: &str = "Search ~/.cloudflared, ~/.cloudflare-warp, ~/cloudflare-warp, \
                                          /etc/cloudflared, /usr/local/etc/cloudflared.";
const HELP_CREATE_DESCRIPTION: &str = "If no config exists, write /usr/local/etc/cloudflared/config.yml \
                                       with logDirectory: /var/log/cloudflared.";
const HELP_HOME_DESCRIPTION: &str = "Expands the leading ~ in default config search directories.";
const HELP_RUN_OUTPUT_DESCRIPTION: &str = "Reports runtime lifecycle, owner-scoped transport/protocol/proxy \
                                           state,\n            narrow readiness, and localized failure \
                                           visibility for the admitted path.";
const HELP_DEFERRED_DESCRIPTION: &str = "Broader origin support, registration RPC, incoming stream \
                                         handling,\n  certificate/key container handling beyond the active \
                                         path, packaging, and deployment tooling";
const USAGE_GUIDANCE_TEMPLATE: &str =
    "error: {message}\nRun `cloudflared help` for the admitted command surface.\n";
const CONFIG_ERROR_TEMPLATE: &str = "error: startup validation failed [{category}]: {error}\n";
const MISSING_FLAG_VALUE_TEMPLATE: &str = "missing value for {flag}";
const REPEATED_FLAG_TEMPLATE: &str = "{flag} may only be provided once";
const UNKNOWN_FLAG_TEMPLATE: &str = "unknown flag: {flag}";
const UNKNOWN_ARGUMENT_TEMPLATE: &str = "unknown command or argument: {value}";
const MULTIPLE_COMMANDS_TEMPLATE: &str = "multiple commands were provided: {existing} and {next}";
const VERSION_OUTPUT_TEMPLATE: &str = "{program} {version}\n";

pub fn command_label(command: Command) -> &'static str {
    match command {
        Command::Help => HELP_COMMAND,
        Command::Version => VERSION_COMMAND,
        Command::Validate => VALIDATE_COMMAND,
        Command::Run => RUN_COMMAND,
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

pub fn parse_command_token(token: &str) -> Option<Command> {
    match token {
        VALIDATE_COMMAND => Some(Command::Validate),
        RUN_COMMAND => Some(Command::Run),
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

pub fn multiple_commands_message(existing: Command, next: Command) -> String {
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

pub fn render_help_text(program_name: &str) -> String {
    let mut text = String::new();
    text.push_str(&render_version_output(program_name));
    text.push_str(HELP_DESCRIPTION);
    text.push_str("\n\n");
    text.push_str(HELP_USAGE_HEADING);
    text.push('\n');
    text.push_str(&format!(
        "  {program_name} [{CONFIG_FLAG} {CONFIG_PATH_METAVAR}] {VALIDATE_COMMAND}\n"
    ));
    text.push_str(&format!(
        "  {program_name} [{CONFIG_FLAG} {CONFIG_PATH_METAVAR}] {RUN_COMMAND}\n"
    ));
    text.push_str(&format!("  {program_name} {HELP_COMMAND}\n"));
    text.push_str(&format!("  {program_name} {VERSION_COMMAND}\n\n"));
    text.push_str(HELP_COMMANDS_HEADING);
    text.push('\n');
    text.push_str(&format!("  {VALIDATE_COMMAND:<8} {HELP_VALIDATE_DESCRIPTION}\n"));
    text.push_str(&format!("  {RUN_COMMAND:<8} {HELP_RUN_DESCRIPTION}\n"));
    text.push_str(&format!("  {VERSION_COMMAND:<8} {HELP_VERSION_DESCRIPTION}\n"));
    text.push_str(&format!("  {HELP_COMMAND:<8} {HELP_HELP_DESCRIPTION}\n\n"));
    text.push_str(HELP_FLAGS_HEADING);
    text.push('\n');
    text.push_str(&format!(
        "  {CONFIG_FLAG} {CONFIG_PATH_METAVAR}  {HELP_CONFIG_FLAG_DESCRIPTION}\n"
    ));
    text.push_str(&format!("  default discovery  {HELP_DISCOVERY_DESCRIPTION}\n"));
    text.push_str(&format!("  default create     {HELP_CREATE_DESCRIPTION}\n\n"));
    text.push_str(HELP_ENV_HEADING);
    text.push('\n');
    text.push_str(&format!("  {HOME_ENV}  {HELP_HOME_DESCRIPTION}\n\n"));
    text.push_str(HELP_OPERABILITY_HEADING);
    text.push('\n');
    text.push_str(&format!("  run output  {HELP_RUN_OUTPUT_DESCRIPTION}\n\n"));
    text.push_str(HELP_DEFERRED_HEADING);
    text.push('\n');
    text.push_str(HELP_DEFERRED_DESCRIPTION);
    text.push('\n');
    text
}
