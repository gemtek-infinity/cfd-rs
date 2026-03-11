use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Help,
    Version,
    Validate,
    Run,
}

impl Command {
    pub(super) fn as_str(&self) -> &'static str {
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
