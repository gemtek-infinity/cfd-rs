use std::fmt;
use std::path::PathBuf;

use thiserror::Error;

/// Typed error category for structured reporting and artifact envelopes.
///
/// Replaces the previous `&'static str` return from `ConfigError::category()`
/// so that category values are exhaustively matched rather than compared
/// as free-form strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    NoConfigFile,
    Io,
    YamlParse,
    JsonParse,
    JsonSerialize,
    InvalidUrl,
    InvalidUuid,
    CreateDirectory,
    CreateFile,
    WriteFile,
    OriginCertEmpty,
    OriginCertInvalidPem,
    OriginCertUnknownBlock,
    OriginCertMultipleTokens,
    OriginCertMissingToken,
    OriginCertNeedsRefresh,
    NoIngressRulesFlags,
    IngressLastRuleNotCatchAll,
    IngressBadWildcard,
    IngressHostnameContainsPort,
    IngressCatchAllNotLast,
    InvalidIngressService,
    TokenDecode,
    InvariantViolation,
    Deferred,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::NoConfigFile => "no-config-file",
            Self::Io => "io",
            Self::YamlParse => "yaml-parse",
            Self::JsonParse => "json-parse",
            Self::JsonSerialize => "json-serialize",
            Self::InvalidUrl => "invalid-url",
            Self::InvalidUuid => "invalid-uuid",
            Self::CreateDirectory => "create-directory",
            Self::CreateFile => "create-file",
            Self::WriteFile => "write-file",
            Self::OriginCertEmpty => "origin-cert-empty",
            Self::OriginCertInvalidPem => "origin-cert-invalid-pem",
            Self::OriginCertUnknownBlock => "origin-cert-unknown-block",
            Self::OriginCertMultipleTokens => "origin-cert-multiple-tokens",
            Self::OriginCertMissingToken => "origin-cert-missing-token",
            Self::OriginCertNeedsRefresh => "origin-cert-needs-refresh",
            Self::NoIngressRulesFlags => "no-ingress-rules-flags",
            Self::IngressLastRuleNotCatchAll => "ingress-last-rule-not-catch-all",
            Self::IngressBadWildcard => "ingress-bad-wildcard",
            Self::IngressHostnameContainsPort => "ingress-hostname-contains-port",
            Self::IngressCatchAllNotLast => "ingress-catch-all-not-last",
            Self::InvalidIngressService => "invalid-ingress-service",
            Self::TokenDecode => "token-decode",
            Self::InvariantViolation => "invariant-violation",
            Self::Deferred => "deferred",
        };
        f.write_str(label)
    }
}

impl serde::Serialize for ErrorCategory {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

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

    #[error("failed to decode tunnel token: {reason}")]
    TokenDecode { reason: String },

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

    pub fn token_decode(reason: impl Into<String>) -> Self {
        Self::TokenDecode {
            reason: reason.into(),
        }
    }

    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::NoConfigFile => ErrorCategory::NoConfigFile,
            Self::Io { .. } => ErrorCategory::Io,
            Self::Yaml { .. } => ErrorCategory::YamlParse,
            Self::JsonParse { .. } => ErrorCategory::JsonParse,
            Self::JsonSerialize { .. } => ErrorCategory::JsonSerialize,
            Self::InvalidUrl { .. } => ErrorCategory::InvalidUrl,
            Self::InvalidUuid { .. } => ErrorCategory::InvalidUuid,
            Self::CreateDirectory { .. } => ErrorCategory::CreateDirectory,
            Self::CreateFile { .. } => ErrorCategory::CreateFile,
            Self::WriteFile { .. } => ErrorCategory::WriteFile,
            Self::OriginCertEmpty => ErrorCategory::OriginCertEmpty,
            Self::OriginCertInvalidPem => ErrorCategory::OriginCertInvalidPem,
            Self::OriginCertUnknownBlock { .. } => ErrorCategory::OriginCertUnknownBlock,
            Self::OriginCertMultipleTokens => ErrorCategory::OriginCertMultipleTokens,
            Self::OriginCertMissingToken => ErrorCategory::OriginCertMissingToken,
            Self::OriginCertNeedsRefresh { .. } => ErrorCategory::OriginCertNeedsRefresh,
            Self::NoIngressRulesFlags => ErrorCategory::NoIngressRulesFlags,
            Self::IngressLastRuleNotCatchAll => ErrorCategory::IngressLastRuleNotCatchAll,
            Self::IngressBadWildcard => ErrorCategory::IngressBadWildcard,
            Self::IngressHostnameContainsPort => ErrorCategory::IngressHostnameContainsPort,
            Self::IngressCatchAllNotLast { .. } => ErrorCategory::IngressCatchAllNotLast,
            Self::InvalidIngressService { .. } => ErrorCategory::InvalidIngressService,
            Self::TokenDecode { .. } => ErrorCategory::TokenDecode,
            Self::InvariantViolation { .. } => ErrorCategory::InvariantViolation,
            Self::Deferred { .. } => ErrorCategory::Deferred,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_category_display_outputs_kebab_case() {
        assert_eq!(ErrorCategory::NoConfigFile.to_string(), "no-config-file");
        assert_eq!(ErrorCategory::Io.to_string(), "io");
        assert_eq!(
            ErrorCategory::OriginCertInvalidPem.to_string(),
            "origin-cert-invalid-pem"
        );
        assert_eq!(
            ErrorCategory::NoIngressRulesFlags.to_string(),
            "no-ingress-rules-flags"
        );
        assert_eq!(ErrorCategory::Deferred.to_string(), "deferred");
    }

    #[test]
    fn error_category_serializes_as_kebab_case_string() {
        let json =
            serde_json::to_string(&ErrorCategory::IngressLastRuleNotCatchAll).expect("serialize category");
        assert_eq!(json, "\"ingress-last-rule-not-catch-all\"");
    }

    #[test]
    fn config_error_category_is_exhaustive() {
        let error = ConfigError::NoConfigFile;
        assert_eq!(error.category(), ErrorCategory::NoConfigFile);

        let error = ConfigError::NoIngressRulesFlags;
        assert_eq!(error.category(), ErrorCategory::NoIngressRulesFlags);
    }
}
