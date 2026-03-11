use cloudflared_config::ConfigError;

use super::CliOutput;

pub(super) enum CliError {
    Usage(String),
    Config(ConfigError),
}

impl CliError {
    pub(super) fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    pub(super) fn config(error: ConfigError) -> Self {
        Self::Config(error)
    }

    pub(super) fn into_output(self) -> CliOutput {
        match self {
            Self::Usage(message) => CliOutput::usage_failure(format!(
                "error: {message}\nRun `cloudflared help` for the admitted command surface.\n"
            )),
            Self::Config(error) => CliOutput::failure(
                String::new(),
                format!(
                    "error: startup validation failed [{}]: {error}\n",
                    error.category()
                ),
                1,
            ),
        }
    }
}
