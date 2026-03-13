use cfdrs_shared::ConfigError;

use super::CliOutput;

pub enum CliError {
    Usage(String),
    Config(ConfigError),
}

impl CliError {
    pub fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    pub fn config(error: ConfigError) -> Self {
        Self::Config(error)
    }

    pub fn into_output(self) -> CliOutput {
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
