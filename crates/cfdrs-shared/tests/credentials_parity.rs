#![allow(unused_crate_dependencies)]

use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use cfdrs_shared::{OriginCertToken, OriginCertUser};

#[path = "support/mod.rs"]
mod support;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cfdrs-shared-{name}-{unique}"));
    fs::create_dir_all(&path).expect("temp directory should be created");
    path
}

fn baseline_credential_fixture(path: &str) -> std::path::PathBuf {
    support::repo_root().join(path)
}

#[test]
fn valid_origin_cert_fixture_decodes() {
    let fixture = baseline_credential_fixture(
        "baseline-2026.2.0/old-impl/credentials/test-cloudflare-tunnel-cert-json.pem",
    );
    let token = OriginCertToken::from_pem_path(&fixture).expect("fixture should decode");

    assert_eq!(token.zone_id, "7b0a4d77dfb881c1a3b7d61ea9443e19");
    assert_eq!(token.account_id, "abcdabcdabcdabcd1234567890abcdef");
    assert_eq!(token.api_token, "test-service-key");
    assert_eq!(token.endpoint, None);
}

#[test]
fn valid_origin_cert_user_read_preserves_path() {
    let fixture = baseline_credential_fixture(
        "baseline-2026.2.0/old-impl/credentials/test-cloudflare-tunnel-cert-json.pem",
    );
    let user = OriginCertUser::read(&fixture).expect("fixture should read as user");

    assert_eq!(user.cert_path, fixture);
    assert_eq!(user.cert.account_id, "abcdabcdabcdabcd1234567890abcdef");
}

#[test]
fn missing_token_fixture_maps_to_origin_cert_category() {
    let fixture =
        baseline_credential_fixture("baseline-2026.2.0/old-impl/credentials/test-cert-no-token.pem");
    let error = OriginCertToken::from_pem_path(&fixture).expect_err("fixture should fail");

    assert_eq!(error.category().to_string(), "origin-cert-missing-token");
    assert_eq!(error.to_string(), "missing token in the certificate");
}

#[test]
fn unknown_block_fixture_maps_to_origin_cert_category() {
    let fixture =
        baseline_credential_fixture("baseline-2026.2.0/old-impl/credentials/test-cert-unknown-block.pem");
    let error = OriginCertToken::from_pem_path(&fixture).expect_err("fixture should fail");

    assert_eq!(error.category().to_string(), "origin-cert-unknown-block");
    assert_eq!(
        error.to_string(),
        "unknown block RSA PRIVATE KEY in the certificate"
    );
}

#[test]
fn harness_can_emit_origin_cert_reports() {
    let output_dir = temp_dir("rust-actual-origin-cert");
    let output = Command::new("python3")
        .arg(support::tool_path())
        .arg("emit-rust-actual")
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--fixture-id")
        .arg("origin-cert-json-token")
        .arg("--fixture-id")
        .arg("origin-cert-missing-token")
        .output()
        .expect("python3 should be available to run the shared-behavior parity harness");

    assert!(
        output.status.success(),
        "expected rust actual emission to succeed; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let success_artifact = output_dir.join("origin-cert-json-token.json");
    let success_payload: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&success_artifact).expect("success artifact should be readable"),
    )
    .expect("success artifact should be valid json");
    assert_eq!(success_payload["report_kind"], "credential-report.v1");
    assert_eq!(
        success_payload["payload"]["zone_id"],
        "7b0a4d77dfb881c1a3b7d61ea9443e19"
    );

    let error_artifact = output_dir.join("origin-cert-missing-token.json");
    let error_payload: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&error_artifact).expect("error artifact should be readable"),
    )
    .expect("error artifact should be valid json");
    assert_eq!(error_payload["report_kind"], "error-report.v1");
    assert_eq!(error_payload["payload"]["category"], "origin-cert-missing-token");

    fs::remove_dir_all(output_dir).expect("temp directory should be removable");
}
