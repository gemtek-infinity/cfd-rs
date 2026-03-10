#![allow(unused_crate_dependencies)]

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cloudflared-cli-{name}-{unique}"));
    fs::create_dir_all(&path).expect("temp directory should be created");
    path
}

fn write_config(root: &std::path::Path) -> PathBuf {
    let path = root.join("config.yml");
    fs::write(
        &path,
        "tunnel: phase-3-1\ningress:\n  - hostname: tunnel.example.com\n    service: https://localhost:8080\n  - service: http_status:503\n",
    )
    .expect("config fixture should be written");
    path
}

fn run_cloudflared(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cloudflared"))
        .args(args)
        .output()
        .expect("cloudflared binary should run")
}

#[test]
fn help_lists_only_admitted_phase_3_1_surface() {
    let output = run_cloudflared(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("cloudflared [--config FILEPATH] validate"));
    assert!(stdout.contains("cloudflared [--config FILEPATH] run"));
    assert!(stdout.contains("HOME"));
    assert!(!stdout.contains("tunnel"));
}

#[test]
fn version_prints_workspace_version() {
    let output = run_cloudflared(&["version"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(stdout.trim(), "cloudflared 2026.2.0-alpha.202603");
}

#[test]
fn validate_reports_admitted_startup_surface() {
    let root = temp_dir("validate");
    let config = write_config(&root);

    let output = run_cloudflared(&["validate", "--config", config.to_str().expect("utf-8 path")]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("OK: admitted alpha startup surface validated"));
    assert!(stdout.contains("config-source: explicit"));
    assert!(stdout.contains(&format!("config-path: {}", config.display())));
    assert!(stdout.contains("ingress-rules: 2"));
    assert!(stdout.contains("warnings: none"));

    fs::remove_dir_all(root).expect("temp directory should be removable");
}

#[test]
fn run_exits_nonzero_at_deferred_runtime_boundary() {
    let root = temp_dir("run");
    let config = write_config(&root);

    let output = run_cloudflared(&["--config", config.to_str().expect("utf-8 path"), "run"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout.contains("Resolved admitted alpha startup surface"));
    assert!(stdout.contains("config-source: explicit"));
    assert!(stderr.contains("deferred to Big Phase 3.2"));

    fs::remove_dir_all(root).expect("temp directory should be removable");
}

#[test]
fn unknown_flags_fail_as_usage_errors() {
    let output = run_cloudflared(&["--bogus"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("error: unknown flag: --bogus"));
    assert!(stderr.contains("cloudflared help"));
}
