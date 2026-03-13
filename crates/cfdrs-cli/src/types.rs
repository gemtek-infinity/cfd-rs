use std::fmt;
use std::path::PathBuf;

use crate::surface_contract;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    Validate,
    Run,
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(surface_contract::command_label(*self))
    }
}

#[derive(Debug)]
pub struct Cli {
    pub command: Command,
    pub config_path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_display() {
        assert_eq!(Command::Help.to_string(), surface_contract::HELP_COMMAND);
        assert_eq!(Command::Version.to_string(), surface_contract::VERSION_COMMAND);
        assert_eq!(Command::Validate.to_string(), surface_contract::VALIDATE_COMMAND);
        assert_eq!(Command::Run.to_string(), surface_contract::RUN_COMMAND);
    }
}
