use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("no config file could be resolved")]
    NoConfigFile,

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

    #[error("failed to create directory {path}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to create file {path}")]
    CreateFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write file {path}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("cannot decode empty certificate")]
    OriginCertEmpty,

    #[error("invalid PEM encoding in the certificate")]
    OriginCertInvalidPem,

    #[error("unknown block {block_type} in the certificate")]
    OriginCertUnknownBlock { block_type: String },

    #[error("found multiple tokens in the certificate")]
    OriginCertMultipleTokens,

    #[error("missing token in the certificate")]
    OriginCertMissingToken,

    #[error(
        "Origin certificate needs to be refreshed before creating new tunnels.\nDelete {path} and run \
         \"cloudflared login\" to obtain a new cert."
    )]
    OriginCertNeedsRefresh { path: PathBuf },

    #[error(
        "No ingress rules were defined in provided config (if any) nor from the provided flags, cloudflared \
         will return 503 for all incoming HTTP requests"
    )]
    NoIngressRulesFlags,

    #[error("the last ingress rule must match all URLs")]
    IngressLastRuleNotCatchAll,

    #[error("hostname wildcard must appear only at the start")]
    IngressBadWildcard,

    #[error("hostname cannot contain a port")]
    IngressHostnameContainsPort,

    #[error("rule #{index} is a catch-all before the final rule")]
    IngressCatchAllNotLast { index: usize, hostname: String },

    #[error("invalid ingress service {value}: {reason}")]
    InvalidIngressService { value: String, reason: String },

    #[error("{message}")]
    InvariantViolation { message: String },

    #[error("{operation} is deferred beyond phase 1B.2")]
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

    pub fn create_directory(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::CreateDirectory {
            path: path.into(),
            source,
        }
    }

    pub fn create_file(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::CreateFile {
            path: path.into(),
            source,
        }
    }

    pub fn write_file(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::WriteFile {
            path: path.into(),
            source,
        }
    }

    pub fn origin_cert_unknown_block(block_type: impl Into<String>) -> Self {
        Self::OriginCertUnknownBlock {
            block_type: block_type.into(),
        }
    }

    pub fn origin_cert_invalid_pem(_detail: impl Into<String>) -> Self {
        Self::OriginCertInvalidPem
    }

    pub fn origin_cert_needs_refresh(path: impl Into<PathBuf>) -> Self {
        Self::OriginCertNeedsRefresh { path: path.into() }
    }

    pub fn invalid_ingress_service(value: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidIngressService {
            value: value.into(),
            reason: reason.into(),
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

    pub fn category(&self) -> &'static str {
        match self {
            Self::NoConfigFile => "no-config-file",
            Self::Io { .. } => "io",
            Self::Yaml { .. } => "yaml-parse",
            Self::JsonParse { .. } => "json-parse",
            Self::JsonSerialize { .. } => "json-serialize",
            Self::InvalidUrl { .. } => "invalid-url",
            Self::InvalidUuid { .. } => "invalid-uuid",
            Self::CreateDirectory { .. } => "create-directory",
            Self::CreateFile { .. } => "create-file",
            Self::WriteFile { .. } => "write-file",
            Self::OriginCertEmpty => "origin-cert-empty",
            Self::OriginCertInvalidPem => "origin-cert-invalid-pem",
            Self::OriginCertUnknownBlock { .. } => "origin-cert-unknown-block",
            Self::OriginCertMultipleTokens => "origin-cert-multiple-tokens",
            Self::OriginCertMissingToken => "origin-cert-missing-token",
            Self::OriginCertNeedsRefresh { .. } => "origin-cert-needs-refresh",
            Self::NoIngressRulesFlags => "no-ingress-rules-flags",
            Self::IngressLastRuleNotCatchAll => "ingress-last-rule-not-catch-all",
            Self::IngressBadWildcard => "ingress-bad-wildcard",
            Self::IngressHostnameContainsPort => "ingress-hostname-contains-port",
            Self::IngressCatchAllNotLast { .. } => "ingress-catch-all-not-last",
            Self::InvalidIngressService { .. } => "invalid-ingress-service",
            Self::InvariantViolation { .. } => "invariant-violation",
            Self::Deferred { .. } => "deferred",
        }
    }
}
