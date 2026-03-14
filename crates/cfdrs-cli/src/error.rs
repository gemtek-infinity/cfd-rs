use cfdrs_shared::ConfigError;

use super::{CliOutput, surface_contract};

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
            Self::Usage(message) => CliOutput::usage_failure(surface_contract::usage_guidance(&message)),
            Self::Config(error) => CliOutput::failure(
                String::new(),
                surface_contract::config_error_message(&error.category().to_string(), &error.to_string()),
                1,
            ),
        }
    }
}
