#![forbid(unsafe_code)]

use cloudflared_config as _;
use std::path::Path;

#[test]
fn first_slice_fixture_inventory_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/first-slice");

    for relative in [
        "README.md",
        "fixture-index.toml",
        "discovery-cases.toml",
        "credentials/sources.toml",
        "ingress-cli/cases.toml",
        "config-loading/valid/basic_named_tunnel.yaml",
        "config-loading/valid/unicode_ingress.yaml",
        "config-loading/invalid/missing_catch_all.yaml",
        "config-loading/invalid/invalid_wildcard.yaml",
        "config-loading/edge/no_ingress_minimal.yaml",
        "config-loading/edge/unknown_top_level_key.yaml",
        "golden/README.md",
    ] {
        assert!(
            root.join(relative).exists(),
            "missing first-slice parity fixture: {relative}"
        );
    }
}
