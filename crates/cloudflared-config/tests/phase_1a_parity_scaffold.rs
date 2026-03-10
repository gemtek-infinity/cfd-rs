#![allow(unused_crate_dependencies)]

use cloudflared_config as _;
use std::process::Command;

#[path = "support/mod.rs"]
mod support;

#[test]
fn harness_runner_exists() {
    let tool = support::tool_path();
    assert!(tool.exists(), "missing harness runner at {}", tool.display());
}

#[test]
fn go_truth_capture_gate_is_real() {
    let output = Command::new("python3")
        .arg(support::tool_path())
        .arg("check-go-truth")
        .output()
        .expect("python3 should be available to run the first-slice parity harness");

    assert!(
        output.status.success(),
        "expected Go truth check to pass once truth outputs are captured; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn rust_parity_compare_entrypoint_is_real_for_matching_subset() {
    let output = Command::new("python3")
        .arg(support::tool_path())
        .arg("compare")
        .arg("--require-go-truth")
        .arg("--require-rust-actual")
        .arg("--fixture-id")
        .arg("discover-home-cloudflared")
        .arg("--fixture-id")
        .arg("origin-cert-json-token")
        .arg("--fixture-id")
        .arg("cli-origin-no-origin")
        .output()
        .expect("python3 should be available to run the first-slice parity harness");

    assert!(
        output.status.success(),
        "expected compare mode to pass for a matching subset; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn full_first_slice_compare_is_green() {
    let output = Command::new("python3")
        .arg(support::tool_path())
        .arg("compare")
        .arg("--require-go-truth")
        .arg("--require-rust-actual")
        .output()
        .expect("python3 should be available to run the first-slice parity harness");

    assert!(
        output.status.success(),
        "expected full accepted first-slice compare to pass; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
