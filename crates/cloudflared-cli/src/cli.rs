use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Help,
    Version,
    Validate,
    Run,
}

impl Command {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Help => "help",
            Self::Version => "version",
            Self::Validate => "validate",
            Self::Run => "run",
        }
    }
}

#[derive(Debug)]
pub(crate) struct Cli {
    pub(crate) command: Command,
    pub(crate) config_path: Option<PathBuf>,
}

pub(crate) fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Cli, String> {
    let mut args = args.into_iter();
    let _ = args.next();

    let mut config_path = None;
    let mut command = None;
    let mut help_requested = false;
    let mut version_requested = false;

    while let Some(arg) = args.next() {
        if arg == OsStr::new("--config") {
            let value = args
                .next()
                .ok_or_else(|| String::from("missing value for --config"))?;
            set_config_path(&mut config_path, PathBuf::from(value))?;
            continue;
        }

        if let Some(path) = parse_equals_flag(&arg, "--config") {
            set_config_path(&mut config_path, PathBuf::from(path))?;
            continue;
        }

        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" | "help" => {
                help_requested = true;
            }
            "--version" | "-V" | "version" => {
                version_requested = true;
            }
            "validate" => {
                set_command(&mut command, Command::Validate)?;
            }
            "run" => {
                set_command(&mut command, Command::Run)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag: {other}"));
            }
            other => {
                return Err(format!("unknown command or argument: {other}"));
            }
        }
    }

    if help_requested {
        return Ok(Cli {
            command: Command::Help,
            config_path,
        });
    }
    if version_requested {
        return Ok(Cli {
            command: Command::Version,
            config_path,
        });
    }

    Ok(Cli {
        command: command.unwrap_or(Command::Help),
        config_path,
    })
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
            "multiple commands were provided: {} and {}",
            existing.as_str(),
            command.as_str()
        ));
    }
    *slot = Some(command);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Command, parse_args};
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn parse(parts: &[&str]) -> super::Cli {
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
