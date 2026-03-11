use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use super::{Cli, Command};

#[derive(Default)]
struct ParseState {
    config_path: Option<PathBuf>,
    command: Option<Command>,
    help_requested: bool,
    version_requested: bool,
}

pub(crate) fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Cli, String> {
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
    if arg == OsStr::new("--config") {
        let value = args
            .next()
            .ok_or_else(|| String::from("missing value for --config"))?;
        set_config_path(&mut state.config_path, PathBuf::from(value))?;
        return Ok(true);
    }

    if let Some(path) = parse_equals_flag(arg, "--config") {
        set_config_path(&mut state.config_path, PathBuf::from(path))?;
        return Ok(true);
    }

    Ok(false)
}

fn handle_non_config_argument(arg: OsString, state: &mut ParseState) -> Result<(), String> {
    match arg.to_string_lossy().as_ref() {
        "--help" | "-h" | "help" => {
            state.help_requested = true;
            Ok(())
        }
        "--version" | "-V" | "version" => {
            state.version_requested = true;
            Ok(())
        }
        "validate" => set_command(&mut state.command, Command::Validate),
        "run" => set_command(&mut state.command, Command::Run),
        other if other.starts_with('-') => Err(format!("unknown flag: {other}")),
        other => Err(format!("unknown command or argument: {other}")),
    }
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
        return Err(String::from("--config may only be provided once"));
    }
    *slot = Some(path);
    Ok(())
}

fn set_command(slot: &mut Option<Command>, command: Command) -> Result<(), String> {
    if let Some(existing) = slot
        && *existing != command
    {
        return Err(format!(
            "multiple commands were provided: {existing} and {command}"
        ));
    }
    *slot = Some(command);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_args;
    use crate::surface::Command;
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn parse(parts: &[&str]) -> crate::surface::Cli {
        let args = std::iter::once(OsString::from("cloudflared"))
            .chain(parts.iter().map(OsString::from))
            .collect::<Vec<_>>();
        parse_args(args).expect("arguments should parse")
    }

    #[test]
    fn config_flag_can_appear_before_command() {
        let cli = parse(&["--config", "/tmp/config.yml", "validate"]);

        assert_eq!(cli.command, Command::Validate);
        assert_eq!(cli.config_path, Some(PathBuf::from("/tmp/config.yml")));
    }

    #[test]
    fn config_flag_can_appear_after_command() {
        let cli = parse(&["run", "--config=/tmp/config.yml"]);

        assert_eq!(cli.command, Command::Run);
        assert_eq!(cli.config_path, Some(PathBuf::from("/tmp/config.yml")));
    }
}
