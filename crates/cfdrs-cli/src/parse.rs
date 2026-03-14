use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use super::{Cli, Command, surface_contract};

#[derive(Default)]
struct ParseState {
    config_path: Option<PathBuf>,
    command: Option<Command>,
    help_requested: bool,
    version_requested: bool,
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
    if handle_config_flag(arg.as_os_str(), args, state)? {
        return Ok(());
    }

    handle_non_config_argument(arg, state)
}

fn handle_config_flag(
    arg: &OsStr,
    args: &mut impl Iterator<Item = OsString>,
    state: &mut ParseState,
) -> Result<bool, String> {
    if arg == OsStr::new(surface_contract::CONFIG_FLAG) {
        let value = args
            .next()
            .ok_or_else(|| surface_contract::missing_flag_value_message(surface_contract::CONFIG_FLAG))?;
        set_config_path(&mut state.config_path, PathBuf::from(value))?;
        return Ok(true);
    }

    if let Some(path) = parse_equals_flag(arg, surface_contract::CONFIG_FLAG) {
        set_config_path(&mut state.config_path, PathBuf::from(path))?;
        return Ok(true);
    }

    Ok(false)
}

fn handle_non_config_argument(arg: OsString, state: &mut ParseState) -> Result<(), String> {
    let token = arg.to_string_lossy();
    let token = token.as_ref();

    if surface_contract::is_help_token(token) {
        state.help_requested = true;
        return Ok(());
    }

    if surface_contract::is_version_token(token) {
        state.version_requested = true;
        return Ok(());
    }

    if let Some(command) = surface_contract::parse_command_token(token) {
        return set_command(&mut state.command, command);
    }

    if token.starts_with('-') {
        return Err(surface_contract::unknown_flag_message(token));
    }

    Err(surface_contract::unknown_argument_message(token))
}

fn finalize_cli(state: ParseState) -> Cli {
    let ParseState {
        config_path,
        command,
        help_requested,
        version_requested,
    } = state;

    if help_requested {
        return Cli {
            command: Command::Help,
            config_path,
        };
    }

    if version_requested {
        return Cli {
            command: Command::Version,
            config_path,
        };
    }

    Cli {
        command: command.unwrap_or(Command::Help),
        config_path,
    }
}

fn parse_equals_flag<'a>(arg: &'a OsStr, name: &str) -> Option<&'a str> {
    let arg = arg.to_str()?;
    arg.strip_prefix(name)?.strip_prefix('=')
}

fn set_config_path(slot: &mut Option<PathBuf>, path: PathBuf) -> Result<(), String> {
    if slot.is_some() {
        return Err(surface_contract::repeated_flag_message(
            surface_contract::CONFIG_FLAG,
        ));
    }

    *slot = Some(path);
    Ok(())
}

fn set_command(slot: &mut Option<Command>, command: Command) -> Result<(), String> {
    if let Some(existing) = slot
        && *existing != command
    {
        return Err(surface_contract::multiple_commands_message(*existing, command));
    }

    *slot = Some(command);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_args;
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
    fn config_flag_can_appear_before_command() {
        let cli = parse(&[
            surface_contract::CONFIG_FLAG,
            "/tmp/config.yml",
            surface_contract::VALIDATE_COMMAND,
        ]);

        assert_eq!(cli.command, Command::Validate);
        assert_eq!(cli.config_path, Some(PathBuf::from("/tmp/config.yml")));
    }

    #[test]
    fn config_flag_can_appear_after_command() {
        let cli = parse(&[surface_contract::RUN_COMMAND, "--config=/tmp/config.yml"]);

        assert_eq!(cli.command, Command::Run);
        assert_eq!(cli.config_path, Some(PathBuf::from("/tmp/config.yml")));
    }
}
