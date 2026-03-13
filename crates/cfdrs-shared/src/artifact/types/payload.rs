use std::fmt;

use serde::Serialize;
use serde_json::Value;

use crate::config::error::ErrorCategory;
use crate::config::ingress::OriginRequestConfig;
use crate::config::raw_config::WarpRoutingConfig;

pub const SCHEMA_VERSION: u32 = 1;

/// The kind of report in an artifact envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    Discovery,
    NormalizedConfig,
    Error,
    Credential,
    Ingress,
}

impl fmt::Display for ReportKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Discovery => "discovery-report.v1",
            Self::NormalizedConfig => "normalized-config.v1",
            Self::Error => "error-report.v1",
            Self::Credential => "credential-report.v1",
            Self::Ingress => "ingress-report.v1",
        };
        f.write_str(label)
    }
}

impl Serialize for ReportKind {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// The kind of discovery action taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiscoveryActionKind {
    UseExisting,
    CreateDefaultConfig,
}

impl fmt::Display for DiscoveryActionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UseExisting => f.write_str("use-existing"),
            Self::CreateDefaultConfig => f.write_str("create-default-config"),
        }
    }
}

/// The kind of config source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    ExplicitPath,
    DiscoveredPath,
    AutoCreatedPath,
    FlagSingleOrigin,
}

impl fmt::Display for SourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExplicitPath => f.write_str("explicit-path"),
            Self::DiscoveredPath => f.write_str("discovered-path"),
            Self::AutoCreatedPath => f.write_str("auto-created-path"),
            Self::FlagSingleOrigin => f.write_str("flag-single-origin"),
        }
    }
}

/// The kind of ingress service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IngressServiceKind {
    Http,
    TcpOverWebsocket,
    UnixSocket,
    UnixSocketTls,
    HttpStatus,
    HelloWorld,
    Bastion,
    SocksProxy,
    NamedToken,
}

impl fmt::Display for IngressServiceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http => f.write_str("http"),
            Self::TcpOverWebsocket => f.write_str("tcp-over-websocket"),
            Self::UnixSocket => f.write_str("unix-socket"),
            Self::UnixSocketTls => f.write_str("unix-socket-tls"),
            Self::HttpStatus => f.write_str("http-status"),
            Self::HelloWorld => f.write_str("hello-world"),
            Self::Bastion => f.write_str("bastion"),
            Self::SocksProxy => f.write_str("socks-proxy"),
            Self::NamedToken => f.write_str("named-token"),
        }
    }
}

/// The kind of origin cert locator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OriginCertLocatorKind {
    ConfiguredPath,
    DefaultSearchPath,
}

impl fmt::Display for OriginCertLocatorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfiguredPath => f.write_str("configured-path"),
            Self::DefaultSearchPath => f.write_str("default-search-path"),
        }
    }
}

/// The kind of credential report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CredentialKind {
    OriginCertPem,
}

impl fmt::Display for CredentialKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OriginCertPem => f.write_str("origin-cert-pem"),
        }
    }
}

/// The kind of normalization warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WarningKind {
    UnknownTopLevelKeys,
}

impl fmt::Display for WarningKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTopLevelKeys => f.write_str("unknown-top-level-keys"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactEnvelope {
    pub schema_version: u32,
    pub fixture_id: String,
    pub producer: &'static str,
    pub report_kind: ReportKind,
    pub comparison: String,
    pub source_refs: Vec<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryReportPayload {
    pub action: DiscoveryActionKind,
    pub source_kind: SourceKind,
    pub resolved_path: String,
    pub created_paths: Vec<String>,
    pub written_config: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorReportPayload {
    pub category: ErrorCategory,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CredentialReportPayload {
    pub kind: CredentialKind,
    pub source_path: String,
    pub zone_id: String,
    pub account_id: String,
    pub api_token: String,
    pub endpoint: Option<String>,
    pub is_fed_endpoint: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngressReportPayload {
    pub source_kind: SourceKind,
    pub rule_count: usize,
    pub catch_all_rule_index: usize,
    pub defaults: OriginRequestConfig,
    pub rules: Vec<IngressRulePayload>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NormalizedConfigPayload {
    pub source_kind: SourceKind,
    pub source_path: String,
    pub tunnel: Option<TunnelReferencePayload>,
    pub credentials: CredentialSurfacePayload,
    pub ingress: Vec<IngressRulePayload>,
    pub origin_request: OriginRequestConfig,
    pub warp_routing: WarpRoutingConfig,
    pub log_directory: Option<String>,
    pub warnings: Option<Vec<WarningPayload>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TunnelReferencePayload {
    pub raw: String,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CredentialSurfacePayload {
    pub credentials_file: Option<String>,
    pub origin_cert: Option<OriginCertLocatorPayload>,
    pub tunnel: Option<TunnelReferencePayload>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OriginCertLocatorPayload {
    pub kind: OriginCertLocatorKind,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngressRulePayload {
    pub hostname: Option<String>,
    pub punycode_hostname: Option<String>,
    pub path: Option<String>,
    pub service: IngressServicePayload,
    pub origin_request: OriginRequestConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngressServicePayload {
    pub kind: IngressServiceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WarningPayload {
    pub kind: WarningKind,
    pub keys: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_kind_display_and_serialize() {
        assert_eq!(ReportKind::Discovery.to_string(), "discovery-report.v1");
        assert_eq!(ReportKind::NormalizedConfig.to_string(), "normalized-config.v1");
        assert_eq!(ReportKind::Error.to_string(), "error-report.v1");
        assert_eq!(ReportKind::Credential.to_string(), "credential-report.v1");
        assert_eq!(ReportKind::Ingress.to_string(), "ingress-report.v1");

        let json = serde_json::to_string(&ReportKind::NormalizedConfig).expect("serialize");
        assert_eq!(json, "\"normalized-config.v1\"");
    }

    #[test]
    fn source_kind_display_and_serialize() {
        assert_eq!(SourceKind::ExplicitPath.to_string(), "explicit-path");
        assert_eq!(SourceKind::FlagSingleOrigin.to_string(), "flag-single-origin");

        let json = serde_json::to_string(&SourceKind::DiscoveredPath).expect("serialize");
        assert_eq!(json, "\"discovered-path\"");
    }

    #[test]
    fn ingress_service_kind_display_and_serialize() {
        assert_eq!(IngressServiceKind::Http.to_string(), "http");
        assert_eq!(
            IngressServiceKind::TcpOverWebsocket.to_string(),
            "tcp-over-websocket"
        );
        assert_eq!(IngressServiceKind::HttpStatus.to_string(), "http-status");

        let json = serde_json::to_string(&IngressServiceKind::SocksProxy).expect("serialize");
        assert_eq!(json, "\"socks-proxy\"");
    }

    #[test]
    fn credential_kind_display() {
        assert_eq!(CredentialKind::OriginCertPem.to_string(), "origin-cert-pem");
    }

    #[test]
    fn warning_kind_display() {
        assert_eq!(
            WarningKind::UnknownTopLevelKeys.to_string(),
            "unknown-top-level-keys"
        );
    }
}
