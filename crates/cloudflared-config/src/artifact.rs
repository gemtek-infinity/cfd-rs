use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::credentials::{CredentialSurface, OriginCertLocator, OriginCertToken, TunnelReference};
use crate::discovery::{ConfigSource, DiscoveryAction, DiscoveryOutcome};
use crate::error::ConfigError;
use crate::ingress::{IngressRule, IngressService, NormalizedIngress, OriginRequestConfig};
use crate::normalized::{NormalizationWarning, NormalizedConfig};
use crate::raw_config::WarpRoutingConfig;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
pub struct EmissionPlan {
    pub repo_root: PathBuf,
    pub fixture_root: PathBuf,
    pub output_dir: PathBuf,
    pub fixtures: Vec<FixtureSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FixtureSpec {
    pub fixture_id: String,
    pub category: String,
    pub comparison: String,
    pub input: String,
    pub source_refs: Vec<String>,
    #[serde(default)]
    pub discovery_case: Option<DiscoveryCase>,
    #[serde(default)]
    pub origin_cert_source: Option<String>,
    #[serde(default)]
    pub ordering_case: Option<OrderingCase>,
    #[serde(default)]
    pub flag_ingress_case: Option<FlagIngressCase>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscoveryCase {
    pub explicit_config: bool,
    pub present: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderingCase {
    pub input: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlagIngressCase {
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactEnvelope {
    pub schema_version: u32,
    pub fixture_id: String,
    pub producer: &'static str,
    pub report_kind: &'static str,
    pub comparison: String,
    pub source_refs: Vec<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryReportPayload {
    pub action: &'static str,
    pub source_kind: &'static str,
    pub resolved_path: String,
    pub created_paths: Vec<String>,
    pub written_config: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorReportPayload {
    pub category: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CredentialReportPayload {
    pub kind: &'static str,
    pub source_path: String,
    pub zone_id: String,
    pub account_id: String,
    pub api_token: String,
    pub endpoint: Option<String>,
    pub is_fed_endpoint: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngressReportPayload {
    pub source_kind: &'static str,
    pub rule_count: usize,
    pub catch_all_rule_index: usize,
    pub defaults: OriginRequestConfig,
    pub rules: Vec<IngressRulePayload>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NormalizedConfigPayload {
    pub source_kind: &'static str,
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
    pub kind: &'static str,
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
    pub kind: &'static str,
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
    pub kind: &'static str,
    pub keys: Vec<String>,
}

pub fn discovery_envelope(
    fixture: &FixtureSpec,
    payload: DiscoveryReportPayload,
) -> Result<ArtifactEnvelope, serde_json::Error> {
    Ok(ArtifactEnvelope {
        schema_version: SCHEMA_VERSION,
        fixture_id: fixture.fixture_id.clone(),
        producer: "rust-actual",
        report_kind: "discovery-report.v1",
        comparison: fixture.comparison.clone(),
        source_refs: fixture.source_refs.clone(),
        payload: serde_json::to_value(payload)?,
    })
}

pub fn normalized_config_envelope(
    fixture: &FixtureSpec,
    source_path: &Path,
    normalized: &NormalizedConfig,
) -> Result<ArtifactEnvelope, serde_json::Error> {
    Ok(ArtifactEnvelope {
        schema_version: SCHEMA_VERSION,
        fixture_id: fixture.fixture_id.clone(),
        producer: "rust-actual",
        report_kind: "normalized-config.v1",
        comparison: fixture.comparison.clone(),
        source_refs: fixture.source_refs.clone(),
        payload: serde_json::to_value(NormalizedConfigPayload::from_normalized(source_path, normalized))?,
    })
}

pub fn error_envelope(
    fixture: &FixtureSpec,
    error: &ConfigError,
) -> Result<ArtifactEnvelope, serde_json::Error> {
    Ok(ArtifactEnvelope {
        schema_version: SCHEMA_VERSION,
        fixture_id: fixture.fixture_id.clone(),
        producer: "rust-actual",
        report_kind: "error-report.v1",
        comparison: fixture.comparison.clone(),
        source_refs: fixture.source_refs.clone(),
        payload: serde_json::to_value(ErrorReportPayload {
            category: error.category(),
            message: error.to_string(),
        })?,
    })
}

pub fn credential_envelope(
    fixture: &FixtureSpec,
    source_path: &str,
    token: &OriginCertToken,
) -> Result<ArtifactEnvelope, serde_json::Error> {
    Ok(ArtifactEnvelope {
        schema_version: SCHEMA_VERSION,
        fixture_id: fixture.fixture_id.clone(),
        producer: "rust-actual",
        report_kind: "credential-report.v1",
        comparison: fixture.comparison.clone(),
        source_refs: fixture.source_refs.clone(),
        payload: serde_json::to_value(CredentialReportPayload::from_origin_cert(source_path, token))?,
    })
}

pub fn ingress_envelope(
    fixture: &FixtureSpec,
    source_kind: &'static str,
    normalized: &NormalizedIngress,
) -> Result<ArtifactEnvelope, serde_json::Error> {
    Ok(ArtifactEnvelope {
        schema_version: SCHEMA_VERSION,
        fixture_id: fixture.fixture_id.clone(),
        producer: "rust-actual",
        report_kind: "ingress-report.v1",
        comparison: fixture.comparison.clone(),
        source_refs: fixture.source_refs.clone(),
        payload: serde_json::to_value(IngressReportPayload::from_ingress(source_kind, normalized))?,
    })
}

impl DiscoveryReportPayload {
    pub fn from_outcome(outcome: &DiscoveryOutcome, sandbox_root: &Path) -> Self {
        Self {
            action: match outcome.action {
                DiscoveryAction::UseExisting => "use-existing",
                DiscoveryAction::CreateDefaultConfig => "create-default-config",
            },
            source_kind: match &outcome.source {
                ConfigSource::ExplicitPath(_) => "explicit-path",
                ConfigSource::DiscoveredPath(_) => "discovered-path",
                ConfigSource::AutoCreatedPath(_) => "auto-created-path",
            },
            resolved_path: display_path(&outcome.path, sandbox_root),
            created_paths: outcome
                .created_paths
                .iter()
                .map(|path| display_path(path, sandbox_root))
                .collect(),
            written_config: outcome.written_config.clone(),
        }
    }
}

impl NormalizedConfigPayload {
    pub fn from_normalized(source_path: &Path, normalized: &NormalizedConfig) -> Self {
        Self {
            source_kind: match normalized.source {
                ConfigSource::ExplicitPath(_) => "explicit-path",
                ConfigSource::DiscoveredPath(_) => "discovered-path",
                ConfigSource::AutoCreatedPath(_) => "auto-created-path",
            },
            source_path: source_path.display().to_string(),
            tunnel: normalized
                .tunnel
                .as_ref()
                .map(TunnelReferencePayload::from_tunnel),
            credentials: CredentialSurfacePayload::from_credentials(&normalized.credentials),
            ingress: normalized
                .ingress
                .iter()
                .map(IngressRulePayload::from_rule)
                .collect(),
            origin_request: normalized.origin_request.clone(),
            warp_routing: normalized.warp_routing.clone(),
            log_directory: normalized
                .log_directory
                .as_ref()
                .map(|path| path.display().to_string()),
            warnings: if normalized.warnings.is_empty() {
                None
            } else {
                Some(
                    normalized
                        .warnings
                        .iter()
                        .map(WarningPayload::from_warning)
                        .collect(),
                )
            },
        }
    }
}

impl TunnelReferencePayload {
    fn from_tunnel(tunnel: &TunnelReference) -> Self {
        Self {
            raw: tunnel.raw.clone(),
            uuid: tunnel.uuid.map(|value| value.to_string()),
        }
    }
}

impl CredentialSurfacePayload {
    fn from_credentials(credentials: &CredentialSurface) -> Self {
        Self {
            credentials_file: credentials
                .credentials_file
                .as_ref()
                .map(|path| path.display().to_string()),
            origin_cert: credentials
                .origin_cert
                .as_ref()
                .map(OriginCertLocatorPayload::from_locator),
            tunnel: credentials
                .tunnel
                .as_ref()
                .map(TunnelReferencePayload::from_tunnel),
        }
    }
}

impl OriginCertLocatorPayload {
    fn from_locator(locator: &OriginCertLocator) -> Self {
        match locator {
            OriginCertLocator::ConfiguredPath(path) => Self {
                kind: "configured-path",
                path: path.display().to_string(),
            },
            OriginCertLocator::DefaultSearchPath(path) => Self {
                kind: "default-search-path",
                path: path.display().to_string(),
            },
        }
    }
}

impl IngressRulePayload {
    fn from_rule(rule: &IngressRule) -> Self {
        Self {
            hostname: rule.matcher.hostname.clone(),
            punycode_hostname: rule.matcher.punycode_hostname.clone(),
            path: rule.matcher.path.clone(),
            service: IngressServicePayload::from_service(&rule.service),
            origin_request: rule.origin_request.clone(),
        }
    }
}

impl IngressServicePayload {
    fn from_service(service: &IngressService) -> Self {
        match service {
            IngressService::Http(uri) => Self {
                kind: "http",
                uri: Some(display_origin_url(uri)),
                path: None,
                name: None,
                status_code: None,
            },
            IngressService::TcpOverWebsocket(uri) => Self {
                kind: "tcp-over-websocket",
                uri: Some(display_origin_url(uri)),
                path: None,
                name: None,
                status_code: None,
            },
            IngressService::UnixSocket(path) => Self {
                kind: "unix-socket",
                uri: None,
                path: Some(path.display().to_string()),
                name: None,
                status_code: None,
            },
            IngressService::UnixSocketTls(path) => Self {
                kind: "unix-socket-tls",
                uri: None,
                path: Some(path.display().to_string()),
                name: None,
                status_code: None,
            },
            IngressService::HttpStatus(status_code) => Self {
                kind: "http-status",
                uri: None,
                path: None,
                name: None,
                status_code: Some(*status_code),
            },
            IngressService::HelloWorld => Self {
                kind: "hello-world",
                uri: None,
                path: None,
                name: None,
                status_code: None,
            },
            IngressService::Bastion => Self {
                kind: "bastion",
                uri: None,
                path: None,
                name: None,
                status_code: None,
            },
            IngressService::SocksProxy => Self {
                kind: "socks-proxy",
                uri: None,
                path: None,
                name: None,
                status_code: None,
            },
            IngressService::NamedToken(name) => Self {
                kind: "named-token",
                uri: None,
                path: None,
                name: Some(name.clone()),
                status_code: None,
            },
        }
    }
}

impl WarningPayload {
    fn from_warning(warning: &NormalizationWarning) -> Self {
        match warning {
            NormalizationWarning::UnknownTopLevelKeys(keys) => Self {
                kind: "unknown-top-level-keys",
                keys: keys.clone(),
            },
        }
    }
}

impl CredentialReportPayload {
    fn from_origin_cert(source_path: &str, token: &OriginCertToken) -> Self {
        Self {
            kind: "origin-cert-pem",
            source_path: source_path.to_owned(),
            zone_id: token.zone_id.clone(),
            account_id: token.account_id.clone(),
            api_token: token.api_token.clone(),
            endpoint: token.endpoint.clone(),
            is_fed_endpoint: token.is_fed_endpoint(),
        }
    }
}

impl IngressReportPayload {
    fn from_ingress(source_kind: &'static str, normalized: &NormalizedIngress) -> Self {
        Self {
            source_kind,
            rule_count: normalized.rules.len(),
            catch_all_rule_index: normalized.rules.len().saturating_sub(1),
            defaults: normalized.defaults.clone(),
            rules: normalized
                .rules
                .iter()
                .map(IngressRulePayload::from_rule)
                .collect(),
        }
    }
}

fn display_path(path: &Path, sandbox_root: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(sandbox_root) {
        format!("/{}", relative.display())
    } else {
        path.display().to_string()
    }
}

fn display_origin_url(url: &url::Url) -> String {
    let rendered = url.to_string();
    if url.path() == "/" && url.query().is_none() && url.fragment().is_none() {
        rendered.trim_end_matches('/').to_owned()
    } else {
        rendered
    }
}
