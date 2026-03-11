use serde::Serialize;
use serde_json::Value;

use crate::ingress::OriginRequestConfig;
use crate::raw_config::WarpRoutingConfig;

pub const SCHEMA_VERSION: u32 = 1;

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
