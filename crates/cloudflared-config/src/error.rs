#![forbid(unsafe_code)]

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse YAML from {source_name}")]
    Yaml {
        source_name: String,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("failed to parse JSON for {subject}")]
    JsonParse {
        subject: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize JSON for {subject}")]
    JsonSerialize {
        subject: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid URL for {field}: {value}")]
    InvalidUrl {
        field: &'static str,
        value: String,
        #[source]
        source: url::ParseError,
    },
    #[error("invalid UUID for {field}: {value}")]
    InvalidUuid {
        field: &'static str,
        value: String,
        #[source]
        source: uuid::Error,
    },
    #[error("{message}")]
    InvariantViolation { message: String },
    #[error("{operation} is deferred beyond phase 1B.1")]
    Deferred { operation: &'static str },
}

pub type Result<T> = std::result::Result<T, ConfigError>;

impl ConfigError {
    pub fn read(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn yaml(source_name: impl Into<String>, source: serde_yaml::Error) -> Self {
        Self::Yaml {
            source_name: source_name.into(),
            source,
        }
    }

    pub fn json_parse(subject: &'static str, source: serde_json::Error) -> Self {
        Self::JsonParse { subject, source }
    }

    pub fn json_serialize(subject: &'static str, source: serde_json::Error) -> Self {
        Self::JsonSerialize { subject, source }
    }

    pub fn invalid_url(field: &'static str, value: impl Into<String>, source: url::ParseError) -> Self {
        Self::InvalidUrl {
            field,
            value: value.into(),
            source,
        }
    }

    pub fn invalid_uuid(field: &'static str, value: impl Into<String>, source: uuid::Error) -> Self {
        Self::InvalidUuid {
            field,
            value: value.into(),
            source,
        }
    }

    pub fn invariant(message: impl Into<String>) -> Self {
        Self::InvariantViolation {
            message: message.into(),
        }
    }

    pub fn deferred(operation: &'static str) -> Self {
        Self::Deferred { operation }
    }
}
