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
        "tunnel: 11111111-1111-1111-1111-111111111111\ningress:\n  - hostname: tunnel.example.com\n    service: https://localhost:8080\n  - service: http_status:503\n",
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
fn help_lists_admitted_surface() {
    let output = run_cloudflared(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());

    // Standard help layout sections (matching Go baseline urfave/cli output).
    assert!(stdout.contains("NAME:"));
    assert!(stdout.contains("USAGE:"));
    assert!(stdout.contains("VERSION:"));
    assert!(stdout.contains("DESCRIPTION:"));
    assert!(stdout.contains("COMMANDS:"));
    assert!(stdout.contains("GLOBAL OPTIONS:"));

    // Program identity.
    assert!(stdout.contains("cloudflared - Cloudflare's command-line tool and agent"));

    // All command families from the Go baseline are listed.
    assert!(stdout.contains("update"));
    assert!(stdout.contains("tunnel"));
    assert!(stdout.contains("access"));
    assert!(stdout.contains("tail"));
    assert!(stdout.contains("service"));
    assert!(stdout.contains("help"));

    // Category headings from Go baseline.
    assert!(stdout.contains("Access:"), "missing Access: category");
    assert!(stdout.contains("Tunnel:"), "missing Tunnel: category");

    // COPYRIGHT section from Go baseline.
    assert!(stdout.contains("COPYRIGHT:"), "missing COPYRIGHT section");

    // Key global options (Go baseline app-level flags, not tunnel-level).
    assert!(stdout.contains("--output"));
    assert!(stdout.contains("--credentials-file"));
    assert!(stdout.contains("--help"));
    assert!(stdout.contains("--version"));
}

#[test]
fn version_prints_workspace_version() {
    let output = run_cloudflared(&["version"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    // Go baseline: `cloudflared version {Version} (built {BuildTime})`
    assert_eq!(
        stdout.trim(),
        "cloudflared version 2026.2.0-alpha.202603 (built unknown)"
    );
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
    assert!(stdout.contains("startup-readiness: admitted-for-runtime-handoff"));
    assert!(stdout.contains("warnings: none"));

    fs::remove_dir_all(root).expect("temp directory should be removable");
}

#[test]
fn run_exits_nonzero_when_quic_transport_inputs_are_missing() {
    let root = temp_dir("run");
    let config = write_config(&root);

    let output = run_cloudflared(&["--config", config.to_str().expect("utf-8 path"), "run"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout.contains("Resolved admitted alpha startup surface"));
    assert!(stdout.contains("config-source: explicit"));
    assert!(stdout.contains("startup-readiness: admitted-for-runtime-handoff"));
    assert!(stdout.contains("runtime-owner: initialized"));
    assert!(stdout.contains("config-ownership: runtime-owned"));
    assert!(stdout.contains("readiness-scope: narrow-alpha-control-plane-only"));
    assert!(stdout.contains("security-boundary: runtime-crypto-surface=transport-tls-only"));
    assert!(
        stdout
            .contains("security-boundary-claims: bounded-surface-only, not-whole-program, not-certification")
    );
    assert!(stdout.contains("security-deployment-contract: linux-gnu-glibc"));
    assert!(
        stdout.contains("proxy-seam: origin-proxy admitted"),
        "run output should report the admitted Pingora proxy seam"
    );
    assert!(stdout.contains("proxy-state: admitted"));
    assert!(stdout.contains("protocol-state: bridge-created"));
    assert!(stdout.contains("operability-status: lifecycle=failed readiness=failed"));
    assert!(stdout.contains("operability-metrics: restart-attempts=0 proxy-admissions=1"));
    assert!(stderr.contains("readiness-transition state=waiting-for-transport"));
    assert!(stderr.contains("failure-boundary owner=quic-tunnel-core class=fatal"));
    assert!(stderr.contains("quic tunnel core requires credentials-file or origincert"));

    // Phase 4.4: deployment evidence is emitted even on failure exit
    assert!(
        stdout.contains("deploy-contract: linux-x86_64-gnu-glibc"),
        "run output should contain deployment contract evidence"
    );
    assert!(
        stdout.contains("deploy-host-validation: passed"),
        "run output should confirm host validation passed"
    );
    assert!(
        stdout.contains("deploy-known-gaps:"),
        "run output should declare known deployment gaps"
    );
    assert!(
        stdout.contains("deploy-evidence-scope:"),
        "run output should declare deployment evidence scope"
    );

    fs::remove_dir_all(root).expect("temp directory should be removable");
}

// --- CLI-032: run command reconciliation ---

#[test]
fn tunnel_run_routes_same_as_bare_run() {
    // Go baseline: `cloudflared tunnel run` and `cloudflared run` both
    // dispatch to the same named-tunnel entry point.  Rust routes both
    // `run` and `tunnel run` to Command::Tunnel(TunnelSubcommand::Run).
    let root_run = temp_dir("run-bare");
    let config_run = write_config(&root_run);
    let run_output = run_cloudflared(&["--config", config_run.to_str().expect("utf-8 path"), "run"]);

    let root_tunnel = temp_dir("run-tunnel");
    let config_tunnel = write_config(&root_tunnel);
    let tunnel_output = run_cloudflared(&[
        "--config",
        config_tunnel.to_str().expect("utf-8 path"),
        "tunnel",
        "run",
    ]);

    let run_stdout = String::from_utf8_lossy(&run_output.stdout);
    let tunnel_stdout = String::from_utf8_lossy(&tunnel_output.stdout);

    // Both should reach the same runtime path
    assert_eq!(run_output.status.code(), tunnel_output.status.code());
    assert!(
        run_stdout.contains("runtime-owner: initialized"),
        "bare run should reach runtime: {run_stdout:?}"
    );
    assert!(
        tunnel_stdout.contains("runtime-owner: initialized"),
        "tunnel run should reach runtime: {tunnel_stdout:?}"
    );

    fs::remove_dir_all(root_run).expect("temp directory should be removable");
    fs::remove_dir_all(root_tunnel).expect("temp directory should be removable");
}

#[test]
fn unknown_flags_fail_as_usage_errors() {
    let output = run_cloudflared(&["--bogus"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("error: unknown flag: --bogus"));
    assert!(stderr.contains("cloudflared help"));
}

#[test]
fn unknown_command_fails_as_usage_error() {
    // Go baseline: `cloudflared bogus` falls through to TunnelCommand(c) and
    // produces "You did not specify any valid additional argument". Rust treats
    // unknown top-level tokens as explicit usage errors with exit code 2, which
    // is a deliberate improvement over Go's implicit fallthrough behavior.
    let output = run_cloudflared(&["bogus"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("unknown command or argument: bogus"));
    assert!(stderr.contains("cloudflared help"));
}

#[test]
fn usage_errors_go_to_stderr_not_stdout() {
    // Go baseline: urfave/cli writes "Incorrect Usage." to a.Writer (stdout).
    // Rust deliberately sends usage errors to stderr, following POSIX convention.
    let output = run_cloudflared(&["--bogus"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stdout.is_empty(), "usage errors should not appear on stdout");
    assert!(!stderr.is_empty(), "usage errors should appear on stderr");
}

// --- CLI-008: tunnel bare dispatch parity ---

#[test]
fn tunnel_bare_with_hostname_returns_classic_tunnel_deprecated_error() {
    // Go baseline: `--hostname` set → errDeprecatedClassicTunnel
    let output = run_cloudflared(&["tunnel", "--hostname", "example.com"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr.contains("Classic tunnels have been deprecated"),
        "stderr should contain the Go baseline classic tunnel deprecation message: {stderr:?}"
    );
    assert!(stderr.contains("Named Tunnels"));
}

#[test]
fn tunnel_bare_without_identity_returns_usage_error() {
    // Go baseline: no --name/--url/--hello-world/TunnelID → tunnelCmdErrorMessage
    let output = run_cloudflared(&["tunnel"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr.contains("You did not specify any valid additional argument"),
        "stderr should contain the Go baseline tunnel cmd error message: {stderr:?}"
    );
    assert!(
        stderr.contains("--url"),
        "stderr should mention --url flag as guidance: {stderr:?}"
    );
}

#[test]
fn tunnel_bare_with_config_tunnel_id_runs() {
    // Go baseline: config has TunnelID → delegates to tunnel run
    let root = temp_dir("tunnel-bare-config");
    let config = write_config(&root);

    let output = run_cloudflared(&["tunnel", "--config", config.to_str().expect("utf-8 path")]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should attempt to run (and fail because no credentials), not return the
    // "no valid argument" error.
    assert!(
        !stdout.contains("You did not specify any valid additional argument"),
        "should dispatch to run, not fallthrough error"
    );

    fs::remove_dir_all(root).expect("temp directory should be removable");
}

#[test]
fn tunnel_bare_with_name_flag_returns_stub() {
    // Go baseline: `--name` set → runAdhocNamedTunnel(sc, name, credFile)
    // Rust: dispatches correctly but returns stub (adhoc tunnel create + run
    // requires CDC REST API — CDC-033/034).
    let output = run_cloudflared(&["tunnel", "--name", "my-tunnel"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not produce the "no valid additional argument" fallthrough error.
    assert!(
        !stderr.contains("You did not specify any valid additional argument"),
        "--name should dispatch to adhoc branch, not fallthrough: {stderr:?}"
    );
}

#[test]
fn tunnel_bare_with_url_flag_returns_stub() {
    // Go baseline: `--url` or `--hello-world` → quick tunnel branch
    // Rust: dispatches correctly but returns stub (quick tunnel requires CDC API).
    let output = run_cloudflared(&["tunnel", "--url", "http://localhost:8080"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains("You did not specify any valid additional argument"),
        "--url should dispatch to quick tunnel branch, not fallthrough: {stderr:?}"
    );
}

// --- CLI-025: proxy-dns removed feature ---

#[test]
fn proxy_dns_top_level_returns_removed_error() {
    // Go baseline: top-level `proxy-dns` returns "dns-proxy feature is no longer
    // supported" with exit code 1 (errors.New → urfave/cli exit 1).
    let output = run_cloudflared(&["proxy-dns"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr.contains("dns-proxy feature is no longer supported"),
        "stderr should contain Go baseline removal message: {stderr:?}"
    );
    // Go baseline: log.Error().Msg("DNS Proxy is no longer supported since version
    // 2026.2.0 ...")
    assert!(
        stderr.contains("DNS Proxy is no longer supported since version 2026.2.0"),
        "stderr should contain Go baseline log.Error version message: {stderr:?}"
    );
}

#[test]
fn tunnel_proxy_dns_returns_removed_error() {
    // Go baseline: `tunnel proxy-dns` returns same removal message with exit 1
    let output = run_cloudflared(&["tunnel", "proxy-dns"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr.contains("dns-proxy feature is no longer supported"),
        "stderr should contain Go baseline removal message: {stderr:?}"
    );
}

// --- CLI-026: db-connect removed feature ---

#[test]
fn tunnel_db_connect_returns_removed_error() {
    // Go baseline: cliutil.RemovedCommand("db-connect") uses cli.Exit(..., -1)
    // which shells see as exit code 255 (unsigned byte truncation of -1).
    let output = run_cloudflared(&["tunnel", "db-connect"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(255),
        "Go baseline uses cli.Exit(-1) → 255"
    );
    assert!(
        stderr.contains("db-connect command is no longer supported"),
        "stderr should contain Go baseline removed-command message: {stderr:?}"
    );
    assert!(
        stderr.contains("Consult Cloudflare Tunnel documentation"),
        "stderr should contain documentation guidance: {stderr:?}"
    );
}

// --- CLI-028: login at root level ---

#[test]
fn login_at_root_is_recognized() {
    // Go baseline: `login` at root falls through to tunnel login behavior
    let output = run_cloudflared(&["login"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The command should be recognized (not "unknown command") and dispatch
    // to the stub (not yet implemented). It should NOT be treated as an error
    // for unknown commands.
    assert!(
        !stderr.contains("unknown command"),
        "login should be recognized as a valid command: {stderr:?}"
    );
}

#[test]
fn login_hidden_from_root_help() {
    // Go baseline: buildLoginSubcommand(true) registers root-level `login`
    // with Hidden: true — it must NOT appear in `cloudflared --help` output.
    let output = run_cloudflared(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Extract COMMANDS section only (login might appear in other contexts).
    let commands_start = stdout.find("COMMANDS:").expect("missing COMMANDS section");
    let commands_end = stdout
        .find("GLOBAL OPTIONS:")
        .expect("missing GLOBAL OPTIONS section");
    let commands_section = &stdout[commands_start..commands_end];

    // "login" appears inside "tunnel login" in tunnel help, but must NOT
    // be a standalone entry in root COMMANDS.
    assert!(
        !commands_section.contains("\n   login"),
        "login must be hidden from root help (Go: Hidden: true): {commands_section}"
    );
}

#[test]
fn login_and_tunnel_login_both_dispatch() {
    // Go baseline: top-level `login` and `tunnel login` invoke the same
    // action function.  Both should be recognized and dispatched (currently
    // to stubs since auth flow is not yet implemented).
    let root_login = run_cloudflared(&["login"]);
    let tunnel_login = run_cloudflared(&["tunnel", "login"]);

    let root_stderr = String::from_utf8_lossy(&root_login.stderr);
    let tunnel_stderr = String::from_utf8_lossy(&tunnel_login.stderr);

    // Neither should produce "unknown command" errors.
    assert!(
        !root_stderr.contains("unknown command"),
        "root login should be recognized: {root_stderr:?}"
    );
    assert!(
        !tunnel_stderr.contains("unknown command"),
        "tunnel login should be recognized: {tunnel_stderr:?}"
    );
}

// ---------------------------------------------------------------------------
// CLI-001: root invocation / service mode
// ---------------------------------------------------------------------------

#[test]
fn empty_invocation_returns_service_mode_error() {
    // Go baseline: `cloudflared` with zero args and zero flags enters
    // handleServiceMode() which starts a config-watcher daemon loop.
    // Until the watcher/reload infrastructure (HIS-041 through HIS-043)
    // is wired, this should return a clear guidance message.
    let output = run_cloudflared(&[]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "empty invocation should fail until service mode is implemented"
    );
    assert!(
        stderr.contains("service mode"),
        "error should mention service mode: {stderr:?}"
    );
    assert!(
        stderr.contains("tunnel run"),
        "error should suggest 'tunnel run' as an alternative: {stderr:?}"
    );
}
