#![allow(unused_crate_dependencies)]

use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use cloudflared_config::{
    ConfigError, ConfigSource, IngressService, find_matching_rule, load_normalized_config,
    parse_ingress_flags,
};

#[path = "support/mod.rs"]
mod support;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cloudflared-config-{name}-{unique}"));
    fs::create_dir_all(&path).expect("temp directory should be created");
    path
}

#[test]
fn basic_named_tunnel_fixture_matches_first_rule_then_catch_all() {
    let fixture = support::fixtures_root().join("yaml-config/valid/basic_named_tunnel.yaml");
    let normalized = load_normalized_config(
        &fixture,
        ConfigSource::DiscoveredPath("yaml-config/valid/basic_named_tunnel.yaml".into()),
    )
    .expect("fixture should normalize");

    assert_eq!(
        find_matching_rule(&normalized.ingress, "tunnel1.example.com", "/id"),
        Some(0)
    );
    assert_eq!(
        find_matching_rule(&normalized.ingress, "tunnel1.example.com:443", "/other"),
        Some(1)
    );
    assert_eq!(
        find_matching_rule(&normalized.ingress, "unknown.example.com", "/id"),
        Some(1)
    );
}

#[test]
fn unicode_ingress_fixture_matches_punycode_hostname() {
    let fixture = support::fixtures_root().join("yaml-config/valid/unicode_ingress.yaml");
    let normalized = load_normalized_config(
        &fixture,
        ConfigSource::DiscoveredPath("yaml-config/valid/unicode_ingress.yaml".into()),
    )
    .expect("fixture should normalize");

    assert_eq!(
        find_matching_rule(&normalized.ingress, "xn--m-xgaa.cloudflare.com", "/"),
        Some(0)
    );
}

#[test]
fn flag_origin_url_http_normalizes_to_http_service() {
    let ingress = parse_ingress_flags(&["--url=http://localhost:8080".to_owned()])
        .expect("flag origin should normalize");

    assert_eq!(ingress.rules.len(), 1);
    match &ingress.rules[0].service {
        IngressService::Http(url) => {
            assert_eq!(url.scheme(), "http");
            assert_eq!(url.host_str(), Some("localhost"));
            assert_eq!(url.port(), Some(8080));
        }
        other => panic!("expected HTTP service, found {other:?}"),
    }

    assert_eq!(
        ingress
            .defaults
            .keep_alive_timeout
            .as_ref()
            .map(|value| value.0.as_str()),
        Some("1m30s")
    );
    assert_eq!(ingress.defaults.proxy_port, Some(0));
    assert_eq!(ingress.defaults.bastion_mode, Some(false));
}

#[test]
fn flag_origin_no_origin_returns_expected_error_category() {
    let error = parse_ingress_flags(&[]).expect_err("missing flag origin should fail");

    assert!(matches!(error, ConfigError::NoIngressRulesFlags));
    assert_eq!(error.category(), "no-ingress-rules-flags");
}

#[test]
fn harness_can_emit_ingress_related_rust_actual_artifacts() {
    let output_dir = temp_dir("rust-actual-ingress");
    let output = Command::new("python3")
        .arg(support::tool_path())
        .arg("emit-rust-actual")
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--fixture-id")
        .arg("flag-origin-hello-world")
        .arg("--fixture-id")
        .arg("flag-origin-no-origin")
        .arg("--fixture-id")
        .arg("ordering-catch-all-last")
        .output()
        .expect("python3 should be available to run the first-slice parity harness");

    assert!(
        output.status.success(),
        "expected rust actual emission to succeed; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let flag_artifact = output_dir.join("flag-origin-hello-world.json");
    let flag_payload: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&flag_artifact).expect("flag artifact should be readable"))
            .expect("flag artifact should be valid json");
    assert_eq!(flag_payload["report_kind"], "ingress-report.v1");
    assert_eq!(
        flag_payload["payload"]["rules"][0]["service"]["kind"],
        "hello-world"
    );
    assert_eq!(flag_payload["payload"]["defaults"]["keepAliveTimeout"], "1m30s");
    assert_eq!(flag_payload["payload"]["defaults"]["proxyPort"], 0);
    assert_eq!(flag_payload["payload"]["defaults"]["bastionMode"], false);

    let flag_error_artifact = output_dir.join("flag-origin-no-origin.json");
    let flag_error_payload: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&flag_error_artifact).expect("flag error artifact should be readable"),
    )
    .expect("flag error artifact should be valid json");
    assert_eq!(flag_error_payload["report_kind"], "error-report.v1");
    assert_eq!(
        flag_error_payload["payload"]["category"],
        "no-ingress-rules-flags"
    );

    let ordering_artifact = output_dir.join("ordering-catch-all-last.json");
    let ordering_payload: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&ordering_artifact).expect("ordering artifact should be readable"),
    )
    .expect("ordering artifact should be valid json");
    assert_eq!(ordering_payload["report_kind"], "normalized-config.v1");
    assert_eq!(
        ordering_payload["payload"]["ingress"][1]["service"]["kind"],
        "http"
    );
    assert!(ordering_payload["payload"]["warnings"].is_null());
    assert_eq!(
        ordering_payload["payload"]["ingress"][0]["origin_request"]["keepAliveTimeout"],
        "1m30s"
    );
    assert_eq!(
        ordering_payload["payload"]["ingress"][0]["origin_request"]["proxyPort"],
        0
    );
    assert_eq!(
        ordering_payload["payload"]["ingress"][0]["origin_request"]["ipRules"]
            .as_array()
            .map(|rules| rules.len()),
        Some(2)
    );

    fs::remove_dir_all(output_dir).expect("temp directory should be removable");
}
