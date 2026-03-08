#![forbid(unsafe_code)]

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
#[ignore = "Phase 1A establishes scaffolding only; Go truth capture is pending"]
fn go_truth_capture_gate_is_explicit() {
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
#[ignore = "Phase 1B will start emitting Rust-side reports for comparison"]
fn rust_parity_compare_entrypoint_is_reserved() {
    let output = Command::new("python3")
        .arg(support::tool_path())
        .arg("compare")
        .arg("--require-go-truth")
        .arg("--require-rust-actual")
        .output()
        .expect("python3 should be available to run the first-slice parity harness");

    assert!(
        output.status.success(),
        "expected compare mode to pass once Go truth and Rust actual reports exist; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
