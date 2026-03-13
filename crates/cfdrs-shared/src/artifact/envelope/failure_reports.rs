use super::envelope_with_payload;
use crate::config::credentials::OriginCertToken;
use crate::config::error::ConfigError;
use crate::config::ingress::NormalizedIngress;

use super::super::types::{
    ArtifactEnvelope, CredentialReportPayload, ErrorReportPayload, FixtureSpec, IngressReportPayload,
    ReportKind, SourceKind,
};

pub fn error_envelope(
    fixture: &FixtureSpec,
    error: &ConfigError,
) -> Result<ArtifactEnvelope, serde_json::Error> {
    envelope_with_payload(
        fixture,
        ReportKind::Error,
        serde_json::to_value(ErrorReportPayload {
            category: error.category(),
            message: error.to_string(),
        })?,
    )
}

pub fn credential_envelope(
    fixture: &FixtureSpec,
    source_path: &str,
    token: &OriginCertToken,
) -> Result<ArtifactEnvelope, serde_json::Error> {
    envelope_with_payload(
        fixture,
        ReportKind::Credential,
        serde_json::to_value(CredentialReportPayload::from_origin_cert(source_path, token))?,
    )
}

pub fn ingress_envelope(
    fixture: &FixtureSpec,
    source_kind: SourceKind,
    normalized: &NormalizedIngress,
) -> Result<ArtifactEnvelope, serde_json::Error> {
    envelope_with_payload(
        fixture,
        ReportKind::Ingress,
        serde_json::to_value(IngressReportPayload::from_ingress(source_kind, normalized))?,
    )
}
