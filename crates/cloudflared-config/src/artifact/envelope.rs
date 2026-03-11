use std::path::Path;

use super::types::{
    ArtifactEnvelope, CredentialReportPayload, ErrorReportPayload, FixtureSpec, IngressReportPayload,
    NormalizedConfigPayload, SCHEMA_VERSION,
};
use crate::credentials::OriginCertToken;
use crate::error::ConfigError;
use crate::ingress::NormalizedIngress;
use crate::normalized::NormalizedConfig;

pub fn discovery_envelope(
    fixture: &FixtureSpec,
    payload: super::types::DiscoveryReportPayload,
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
