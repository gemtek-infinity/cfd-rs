mod failure_reports;

use std::path::Path;

use super::types::{ArtifactEnvelope, FixtureSpec, NormalizedConfigPayload, SCHEMA_VERSION};
use crate::normalized::NormalizedConfig;

pub use self::failure_reports::{credential_envelope, error_envelope, ingress_envelope};

pub fn discovery_envelope(
    fixture: &FixtureSpec,
    payload: super::types::DiscoveryReportPayload,
) -> Result<ArtifactEnvelope, serde_json::Error> {
    envelope_with_payload(fixture, "discovery-report.v1", serde_json::to_value(payload)?)
}

pub fn normalized_config_envelope(
    fixture: &FixtureSpec,
    source_path: &Path,
    normalized: &NormalizedConfig,
) -> Result<ArtifactEnvelope, serde_json::Error> {
    envelope_with_payload(
        fixture,
        "normalized-config.v1",
        serde_json::to_value(NormalizedConfigPayload::from_normalized(source_path, normalized))?,
    )
}

pub(super) fn envelope_with_payload(
    fixture: &FixtureSpec,
    report_kind: &'static str,
    payload: serde_json::Value,
) -> Result<ArtifactEnvelope, serde_json::Error> {
    Ok(ArtifactEnvelope {
        schema_version: SCHEMA_VERSION,
        fixture_id: fixture.fixture_id.clone(),
        producer: "rust-actual",
        report_kind,
        comparison: fixture.comparison.clone(),
        source_refs: fixture.source_refs.clone(),
        payload,
    })
}
