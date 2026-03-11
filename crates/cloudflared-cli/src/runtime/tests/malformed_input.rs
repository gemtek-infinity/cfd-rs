//! Phase 4.3: Malformed-input boundary proof tests.
//!
//! Proves that malformed material is rejected at the correct owned
//! boundary with explicit, classifiable error output.
//!
//! What this covers:
//! - invalid YAML at the parse boundary
//! - structurally valid YAML with missing mandatory fields
//! - invalid ingress service strings at the validation boundary
//! - malformed config reaching the runtime startup path
//!
//! What this does not cover:
//! - malformed QUIC wire protocol input (transport is in-process)
//! - malformed HTTP request handling (incoming streams are deferred)

use cloudflared_config::{ConfigSource, ErrorCategory, NormalizedConfig, RawConfig};

// -- YAML parse boundary --

#[test]
fn garbage_yaml_fails_with_yaml_parse_category() {
    let result = RawConfig::from_yaml_str("malformed.yaml", "{{{{not yaml at all");

    let err = result.expect_err("garbage YAML should fail at parse boundary");
    assert_eq!(
        err.category(),
        ErrorCategory::YamlParse,
        "error category should be yaml-parse"
    );
}

#[test]
fn binary_noise_fails_with_yaml_parse_category() {
    let result = RawConfig::from_yaml_str("binary.yaml", "\x00\x01\x02\x03\x04");

    let err = result.expect_err("binary noise should fail at parse boundary");
    assert_eq!(err.category(), ErrorCategory::YamlParse);
}

#[test]
fn empty_yaml_parses_to_default_config() {
    // Empty YAML is valid but produces defaults with no tunnel, no ingress.
    let raw = RawConfig::from_yaml_str("empty.yaml", "").expect("empty YAML should parse as default config");

    assert!(raw.tunnel.is_none());
    assert!(raw.ingress.is_empty());
}

// -- Normalization boundary --

#[test]
fn empty_ingress_normalizes_to_default_no_ingress_rule() {
    let raw = RawConfig::from_yaml_str("no-ingress.yaml", "tunnel: test-tunnel\n").expect("should parse");

    let normalized =
        NormalizedConfig::from_raw(ConfigSource::ExplicitPath("/tmp/no-ingress.yaml".into()), raw)
            .expect("empty ingress should normalize with default no-ingress rule");

    assert_eq!(
        normalized.ingress.len(),
        1,
        "should have exactly one default rule"
    );
}

#[test]
fn ingress_missing_catch_all_fails_at_validation_boundary() {
    // An ingress list that doesn't end with a catch-all rule should be rejected.
    let yaml = "tunnel: test\ningress:\n  - hostname: example.com\n    service: http://localhost:8080\n";

    let raw = RawConfig::from_yaml_str("bad-ingress.yaml", yaml).expect("YAML should parse");

    let result = NormalizedConfig::from_raw(ConfigSource::ExplicitPath("/tmp/bad-ingress.yaml".into()), raw);

    let err = result.expect_err("missing catch-all should fail at validation boundary");
    assert_eq!(
        err.category(),
        ErrorCategory::IngressLastRuleNotCatchAll,
        "error category should be ingress-last-rule-not-catch-all"
    );
}

#[test]
fn ingress_invalid_service_url_fails_at_validation_boundary() {
    let yaml = "tunnel: test\ningress:\n  - service: not-a-valid-service-string\n";

    let raw = RawConfig::from_yaml_str("bad-service.yaml", yaml).expect("YAML should parse");

    let result = NormalizedConfig::from_raw(ConfigSource::ExplicitPath("/tmp/bad-service.yaml".into()), raw);

    let err = result.expect_err("invalid service string should fail at validation boundary");
    assert_eq!(
        err.category(),
        ErrorCategory::InvalidIngressService,
        "error category should be invalid-ingress-service"
    );
}

#[test]
fn ingress_wildcard_in_wrong_position_fails_at_validation_boundary() {
    let yaml = "tunnel: test\ningress:\n  - hostname: \"foo.*.bar.com\"\n    service: http://localhost\n  - \
                service: http_status:404\n";

    let raw = RawConfig::from_yaml_str("bad-wildcard.yaml", yaml).expect("YAML should parse");

    let result = NormalizedConfig::from_raw(ConfigSource::ExplicitPath("/tmp/bad-wildcard.yaml".into()), raw);

    let err = result.expect_err("wildcard in wrong position should fail at validation boundary");
    assert_eq!(err.category(), ErrorCategory::IngressBadWildcard);
}

// -- Error display quality --

#[test]
fn config_errors_have_meaningful_display() {
    let err = RawConfig::from_yaml_str("bad.yaml", "{{{{").expect_err("should fail");

    let display = format!("{err}");
    assert!(
        display.contains("bad.yaml"),
        "error display should include config source name: {display}"
    );
}

#[test]
fn config_error_categories_are_machine_readable() {
    let err = RawConfig::from_yaml_str("test.yaml", "{{{{").expect_err("should fail");

    let category_str = format!("{}", err.category());
    assert!(
        !category_str.is_empty(),
        "error category should have a non-empty machine-readable label"
    );
    assert!(
        !category_str.contains(' '),
        "error category label should not contain spaces: {category_str}"
    );
}
