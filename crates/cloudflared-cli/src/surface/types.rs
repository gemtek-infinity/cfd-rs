use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Help,
    Version,
    Validate,
    Run,
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Help => "help",
            Self::Version => "version",
            Self::Validate => "validate",
            Self::Run => "run",
        };
        f.write_str(label)
    }
}

#[derive(Debug)]
pub(crate) struct Cli {
    pub(crate) command: Command,
    pub(crate) config_path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_display() {
        assert_eq!(Command::Help.to_string(), "help");
        assert_eq!(Command::Version.to_string(), "version");
        assert_eq!(Command::Validate.to_string(), "validate");
        assert_eq!(Command::Run.to_string(), "run");
    }
}
