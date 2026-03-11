#![allow(unused_crate_dependencies)]

use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use cloudflared_config::{
    ConfigError, ConfigSource, DiscoveryDefaults, DiscoveryRequest, IngressService, discover_config,
    load_normalized_config,
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
fn discovery_prefers_first_existing_candidate() {
    let root = temp_dir("fixture-discovery");
    let first = root.join("home/.cloudflared/config.yml");
    let second = root.join("home/.cloudflare-warp/config.yaml");
    fs::create_dir_all(first.parent().expect("first parent should exist"))
        .expect("first parent should be created");
    fs::create_dir_all(second.parent().expect("second parent should exist"))
        .expect("second parent should be created");
    fs::write(&first, "logDirectory: /var/log/cloudflared\n").expect("first config should be written");
    fs::write(&second, "logDirectory: /var/log/cloudflared\n").expect("second config should be written");

    let request = DiscoveryRequest {
        explicit_config: None,
        defaults: DiscoveryDefaults {
            config_filenames: vec!["config.yml".to_owned(), "config.yaml".to_owned()],
            search_directories: vec![root.join("home/.cloudflared"), root.join("home/.cloudflare-warp")],
            primary_config_path: root.join("usr/local/etc/cloudflared/config.yml"),
            primary_log_directory: root.join("var/log/cloudflared"),
        },
    };

    let outcome = discover_config(&request).expect("discovery should succeed");
    assert_eq!(outcome.path, first);

    fs::remove_dir_all(root).expect("temp directory should be removable");
}

#[test]
fn yaml_loading_normalizes_unicode_fixture() {
    let fixture = support::fixtures_root().join("yaml-config/valid/unicode_ingress.yaml");
    let normalized = load_normalized_config(
        &fixture,
        ConfigSource::DiscoveredPath("yaml-config/valid/unicode_ingress.yaml".into()),
    )
    .expect("fixture should normalize");

    assert_eq!(normalized.ingress.len(), 2);
    assert_eq!(
        normalized.ingress[0].matcher.hostname.as_deref(),
        Some("môô.cloudflare.com")
    );
    assert_eq!(
        normalized.ingress[0].matcher.punycode_hostname.as_deref(),
        Some("xn--m-xgaa.cloudflare.com")
    );
}

#[test]
fn invalid_yaml_fixture_maps_to_error_category() {
    let fixture = support::fixtures_root().join("invalid-input/ingress/missing_catch_all.yaml");
    let error = load_normalized_config(
        &fixture,
        ConfigSource::DiscoveredPath("invalid-input/ingress/missing_catch_all.yaml".into()),
    )
    .expect_err("fixture should fail validation");

    assert!(matches!(error, ConfigError::IngressLastRuleNotCatchAll));
    assert_eq!(error.category().to_string(), "ingress-last-rule-not-catch-all");
}

#[test]
fn no_ingress_fixture_emits_default_503_contract() {
    let fixture = support::fixtures_root().join("yaml-config/edge/no_ingress_minimal.yaml");
    let normalized = load_normalized_config(
        &fixture,
        ConfigSource::DiscoveredPath("yaml-config/edge/no_ingress_minimal.yaml".into()),
    )
    .expect("fixture should normalize");

    assert_eq!(normalized.ingress.len(), 1);
    assert_eq!(normalized.ingress[0].service, IngressService::HttpStatus(503));
}

#[test]
fn harness_can_emit_targeted_rust_actual_artifact() {
    let output_dir = temp_dir("rust-actual");
    let output = Command::new("python3")
        .arg(support::tool_path())
        .arg("emit-rust-actual")
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--fixture-id")
        .arg("config-basic-named-tunnel")
        .output()
        .expect("python3 should be available to run the first-slice parity harness");

    assert!(
        output.status.success(),
        "expected rust actual emission to succeed; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_dir.join("config-basic-named-tunnel.json").exists());

    fs::remove_dir_all(output_dir).expect("temp directory should be removable");
}
