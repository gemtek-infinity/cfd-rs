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
                    state.command = Some(Command::Tunnel(sub));
                    state.awaiting_subcommand = None;
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
            command: Command::Version,
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
        assert_eq!(cli.command, Command::Version);
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
            Command::Access
        );
        assert_eq!(
            parse(&[surface_contract::FORWARD_COMMAND]).command,
            Command::Access
        );
        assert_eq!(parse(&[surface_contract::TAIL_COMMAND]).command, Command::Tail);
        assert_eq!(
            parse(&[surface_contract::MANAGEMENT_COMMAND]).command,
            Command::Management
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
}
