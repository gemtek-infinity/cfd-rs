use super::envelope_with_payload;
use crate::credentials::OriginCertToken;
use crate::error::ConfigError;
use crate::ingress::NormalizedIngress;

use super::super::types::{
    ArtifactEnvelope, CredentialReportPayload, ErrorReportPayload, FixtureSpec, IngressReportPayload,
};

pub fn error_envelope(
    fixture: &FixtureSpec,
    error: &ConfigError,
) -> Result<ArtifactEnvelope, serde_json::Error> {
    envelope_with_payload(
        fixture,
        "error-report.v1",
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
        "credential-report.v1",
        serde_json::to_value(CredentialReportPayload::from_origin_cert(source_path, token))?,
    )
}

pub fn ingress_envelope(
    fixture: &FixtureSpec,
    source_kind: &'static str,
    normalized: &NormalizedIngress,
) -> Result<ArtifactEnvelope, serde_json::Error> {
    envelope_with_payload(
        fixture,
        "ingress-report.v1",
        serde_json::to_value(IngressReportPayload::from_ingress(source_kind, normalized))?,
    )
}
