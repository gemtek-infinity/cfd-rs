#![forbid(unsafe_code)]

use cloudflared_config as _;

#[path = "support/mod.rs"]
#[allow(dead_code)]
mod support;

#[test]
fn first_slice_fixture_inventory_exists() {
    let root = support::fixtures_root();

    for relative in [
        "README.md",
        "fixture-index.toml",
        "config-discovery/cases.toml",
        "yaml-config/valid/basic_named_tunnel.yaml",
        "yaml-config/valid/unicode_ingress.yaml",
        "yaml-config/edge/no_ingress_minimal.yaml",
        "yaml-config/edge/unknown_top_level_key.yaml",
        "credentials-origin-cert/sources.toml",
        "ingress-normalization/cases.toml",
        "ordering-defaulting/cases.toml",
        "invalid-input/ingress/missing_catch_all.yaml",
        "invalid-input/ingress/invalid_wildcard.yaml",
        "golden/README.md",
        "golden/schema/README.md",
        "golden/go-truth/README.md",
        "golden/rust-actual/README.md",
    ] {
        assert!(
            root.join(relative).exists(),
            "missing first-slice parity fixture: {relative}"
        );
    }
}

#[test]
fn fixture_index_ids_are_unique() {
    let ids = support::fixture_ids();
    let mut sorted = ids.clone();

    sorted.sort();
    sorted.dedup();

    assert_eq!(ids.len(), sorted.len(), "fixture ids must remain unique");
}

#[test]
fn fixture_index_entries_use_existing_inputs() {
    for entry in support::fixture_entries() {
        let input_path = root_input_path(&entry.input);

        assert!(
            input_path.exists(),
            "fixture {} points to missing input {}",
            entry.id,
            entry.input
        );
    }
}

fn root_input_path(relative: &str) -> std::path::PathBuf {
    support::fixtures_root().join(relative)
}
