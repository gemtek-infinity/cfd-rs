use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use tempfile::TempDir;

fn encoded_tunnel_token() -> String {
    TunnelToken {
        account_tag: "account".to_owned(),
        tunnel_secret: cfdrs_shared::TunnelSecret::from_bytes([1, 2, 3, 4]),
        tunnel_id: uuid::Uuid::nil(),
        endpoint: None,
    }
    .encode()
    .expect("token should encode")
}

fn service_install_cli() -> Cli {
    Cli {
        command: Command::Service(ServiceAction::Install),
        flags: GlobalFlags::default(),
    }
}

fn serve_once(status_line: &str, body: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let address = listener.local_addr().expect("local addr");
    let status_line = status_line.to_owned();
    let body = body.to_owned();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buffer = [0_u8; 1024];
        let _ = stream.read(&mut buffer);
        let response = format!(
            "HTTP/1.1 {status_line}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: \
             close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).expect("write response");
    });

    address.to_string()
}

struct MockDiagnosticServer {
    address: String,
    running: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}

impl MockDiagnosticServer {
    fn start(routes: std::collections::BTreeMap<String, (u16, String, &'static str)>) -> Self {
        Self::start_on("127.0.0.1:0", routes)
    }

    fn start_on(bind: &str, routes: std::collections::BTreeMap<String, (u16, String, &'static str)>) -> Self {
        let listener = TcpListener::bind(bind).expect("listener");
        listener.set_nonblocking(true).expect("set nonblocking");
        let address = listener.local_addr().expect("addr").to_string();
        let running = Arc::new(AtomicBool::new(true));
        let keep_running = Arc::clone(&running);

        let join = thread::spawn(move || {
            while keep_running.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut buffer = [0_u8; 4096];
                        let read = stream.read(&mut buffer).unwrap_or(0);
                        let request = String::from_utf8_lossy(&buffer[..read]);
                        let path = request
                            .lines()
                            .next()
                            .and_then(|line| line.split_whitespace().nth(1))
                            .unwrap_or("/");
                        let (status, body, content_type) =
                            routes
                                .get(path)
                                .cloned()
                                .unwrap_or((404, "not found".to_owned(), "text/plain"));
                        let response = format!(
                            "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nContent-Type: \
                             {content_type}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            address,
            running,
            join: Some(join),
        }
    }
}

impl Drop for MockDiagnosticServer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = std::net::TcpStream::connect(&self.address);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn diagnostic_routes(
    diag_configuration: &str,
) -> std::collections::BTreeMap<String, (u16, String, &'static str)> {
    std::collections::BTreeMap::from([
        (
            "/diag/tunnel".to_owned(),
            (
                200,
                "{\"tunnelID\":\"00000000-0000-0000-0000-000000000000\",\"connectorID\":\"\
                 11111111-1111-1111-1111-111111111111\"}"
                    .to_owned(),
                "application/json",
            ),
        ),
        (
            "/diag/system".to_owned(),
            (
                200,
                "{\"info\":{\"osSystem\":\"Linux\"},\"errors\":{}}".to_owned(),
                "application/json",
            ),
        ),
        (
            "/debug/pprof/goroutine".to_owned(),
            (501, "pprof deferred\n".to_owned(), "text/plain"),
        ),
        (
            "/debug/pprof/heap".to_owned(),
            (501, "pprof deferred\n".to_owned(), "text/plain"),
        ),
        (
            "/metrics".to_owned(),
            (200, "build_info 1\n".to_owned(), "text/plain"),
        ),
        (
            "/diag/configuration".to_owned(),
            (200, diag_configuration.to_owned(), "application/json"),
        ),
        (
            "/config".to_owned(),
            (
                200,
                "{\"version\":1,\"config\":{}}".to_owned(),
                "application/json",
            ),
        ),
    ])
}

fn extract_diagnostic_zip_path(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        line.strip_prefix("Diagnostic file written: ")
            .map(|path| path.trim().to_owned())
    })
}

fn diag_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn write_config(tempdir: &TempDir, contents: &str) -> String {
    let path = tempdir.path().join("config.yml");
    std::fs::write(&path, contents).expect("write config");
    path.display().to_string()
}

#[test]
fn service_install_token_prefers_rest_args() {
    let mut cli = service_install_cli();
    cli.flags.rest_args.push(encoded_tunnel_token());
    cli.flags.token = Some("ignored".to_owned());

    let token = resolve_service_install_token(&cli).expect("token should resolve");
    assert_eq!(token, Some(cli.flags.rest_args[0].clone()));
}

#[test]
fn service_install_token_can_be_loaded_from_file() {
    let mut cli = service_install_cli();
    let token = encoded_tunnel_token();
    let token_path = std::env::temp_dir().join("cfdrs-service-install-token.txt");
    std::fs::write(&token_path, format!("{token}\n")).expect("token file should be written");
    cli.flags.token_file = Some(token_path.clone());

    let resolved = resolve_service_install_token(&cli).expect("token should resolve from file");
    assert_eq!(resolved, Some(token));

    let _ = std::fs::remove_file(token_path);
}

#[test]
fn service_install_rejects_invalid_token() {
    let mut cli = service_install_cli();
    cli.flags.token = Some("not-a-valid-token".to_owned());

    let error = resolve_service_install_token(&cli).expect_err("invalid token should fail");
    assert_eq!(error.category().to_string(), "token-decode");
}

#[test]
fn copy_service_config_skips_service_path() {
    let result = copy_service_config_if_needed(Path::new(SERVICE_CONFIG_PATH));
    assert!(result.is_ok());
}

#[test]
fn runtime_command_label_for_service_install_stays_stable() {
    let label = full_command_label(&Command::Service(ServiceAction::Install));
    assert_eq!(label, "service install");
}

// --- CLI-032: tunnel run NArg validation ---

#[test]
fn tunnel_run_rejects_multiple_positional_args() {
    // Go baseline: c.NArg() > 1 → UsageError (exit -1 = 255)
    let output = execute(
        ["cloudflared", "tunnel", "run", "arg1", "arg2"]
            .into_iter()
            .map(OsString::from),
    );

    assert_eq!(
        output.exit_code, 255,
        "Go baseline exit code is -1 (255 unsigned)"
    );
    assert!(
        output.stderr.contains("accepts only one argument"),
        "stderr must contain NArg error: {:?}",
        output.stderr
    );
    assert!(
        output.stderr.contains("See 'cloudflared tunnel run --help'."),
        "stderr must contain help suffix: {:?}",
        output.stderr
    );
}

#[test]
fn tunnel_run_allows_single_positional_arg() {
    // Go baseline: c.NArg() == 1 is the tunnel name/ID — valid
    let output = execute(
        ["cloudflared", "tunnel", "run", "my-tunnel"]
            .into_iter()
            .map(OsString::from),
    );

    // Should NOT get the NArg error (may get config discovery error, that's fine)
    assert_ne!(
        output.exit_code, 255,
        "single positional arg must not trigger NArg rejection"
    );
    assert!(
        !output.stderr.contains("accepts only one argument"),
        "single positional arg must not trigger NArg error"
    );
}

#[test]
fn tunnel_run_allows_zero_positional_args() {
    // Go baseline: c.NArg() == 0 is valid (uses config tunnel ID or token)
    // — may exit 255 with identity error, but NOT with NArg error
    let output = execute(["cloudflared", "tunnel", "run"].into_iter().map(OsString::from));

    assert!(
        !output.stderr.contains("accepts only one argument"),
        "zero positional args must not trigger NArg error"
    );
}

// --- CLI-032: NArg validation for subcommands (cliutil.UsageError → exit 255)
// ---

/// Helper: execute with the given args and return the output.
fn exec(args: &[&str]) -> CliOutput {
    execute(args.iter().map(OsString::from))
}

struct TestUpdater {
    check: Option<cfdrs_his::updater::UpdateCheck>,
    check_error: Option<&'static str>,
    apply_error: Option<&'static str>,
}

impl cfdrs_his::updater::Updater for TestUpdater {
    fn check(&self) -> cfdrs_shared::Result<cfdrs_his::updater::UpdateCheck> {
        if let Some(message) = self.check_error {
            return Err(ConfigError::invariant(message));
        }

        Ok(self.check.clone().expect("test updater check result"))
    }

    fn apply(&self, _info: &cfdrs_his::updater::UpdateInfo) -> cfdrs_shared::Result<()> {
        if let Some(message) = self.apply_error {
            return Err(ConfigError::invariant(message));
        }

        Ok(())
    }
}

#[test]
fn update_command_reports_package_managed_skip_with_exit_zero() {
    let updater = TestUpdater {
        check: None,
        check_error: Some("should not check"),
        apply_error: None,
    };

    let out = execute_update_with_updater(&updater, true);
    assert_eq!(out.exit_code, 0);
    assert!(out.stderr.contains("installed by a package manager"));
}

#[test]
fn update_command_reports_no_update_with_exit_zero() {
    let updater = TestUpdater {
        check: Some(cfdrs_his::updater::UpdateCheck {
            update: None,
            user_message: Some("already current".to_owned()),
        }),
        check_error: None,
        apply_error: None,
    };

    let out = execute_update_with_updater(&updater, false);
    assert_eq!(out.exit_code, 0);
    assert!(out.stdout.contains("already current"));
    assert!(out.stdout.contains("cloudflared is up to date"));
}

#[test]
fn update_command_returns_exit_11_on_success() {
    let updater = TestUpdater {
        check: Some(cfdrs_his::updater::UpdateCheck {
            update: Some(cfdrs_his::updater::UpdateInfo {
                version: "2026.2.1".to_owned(),
                url: "https://example.com/cloudflared".to_owned(),
                checksum: "deadbeef".to_owned(),
                compressed: false,
                user_message: None,
            }),
            user_message: None,
        }),
        check_error: None,
        apply_error: None,
    };

    let out = execute_update_with_updater(&updater, false);
    assert_eq!(out.exit_code, 11);
    assert!(out.stdout.contains("updated to version 2026.2.1"));
}

#[test]
fn update_command_returns_exit_10_on_failure() {
    let updater = TestUpdater {
        check: Some(cfdrs_his::updater::UpdateCheck {
            update: Some(cfdrs_his::updater::UpdateInfo {
                version: "2026.2.1".to_owned(),
                url: "https://example.com/cloudflared".to_owned(),
                checksum: "deadbeef".to_owned(),
                compressed: false,
                user_message: None,
            }),
            user_message: None,
        }),
        check_error: None,
        apply_error: Some("download failed"),
    };

    let out = execute_update_with_updater(&updater, false);
    assert_eq!(out.exit_code, 10);
    assert!(
        out.stderr
            .contains("failed to update cloudflared: download failed")
    );
}

// tunnel create: NArg != 1 → 255
#[test]
fn tunnel_create_rejects_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "create"]);
    assert_eq!(out.exit_code, 255);
    assert!(out.stderr.contains(TUNNEL_CREATE_NARG_ERROR_MSG));
}

#[test]
fn tunnel_create_rejects_two_args() {
    let out = exec(&["cloudflared", "tunnel", "create", "a", "b"]);
    assert_eq!(out.exit_code, 255);
    assert!(out.stderr.contains(TUNNEL_CREATE_NARG_ERROR_MSG));
}

#[test]
fn tunnel_create_accepts_one_arg() {
    let out = exec(&["cloudflared", "tunnel", "create", "my-tunnel"]);
    assert_ne!(out.exit_code, 255);
}

// tunnel delete: NArg < 1 → 255
#[test]
fn tunnel_delete_rejects_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "delete"]);
    assert_eq!(out.exit_code, 255);
    assert!(out.stderr.contains(TUNNEL_DELETE_NARG_ERROR_MSG));
}

#[test]
fn tunnel_delete_accepts_one_arg() {
    let out = exec(&["cloudflared", "tunnel", "delete", "my-tunnel"]);
    assert_ne!(out.exit_code, 255);
}

#[test]
fn tunnel_delete_accepts_multiple_args() {
    let out = exec(&["cloudflared", "tunnel", "delete", "t1", "t2"]);
    assert_ne!(out.exit_code, 255);
}

// tunnel cleanup: NArg < 1 → 255
#[test]
fn tunnel_cleanup_rejects_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "cleanup"]);
    assert_eq!(out.exit_code, 255);
    assert!(out.stderr.contains(TUNNEL_CLEANUP_NARG_ERROR_MSG));
}

#[test]
fn tunnel_cleanup_accepts_one_arg() {
    let out = exec(&["cloudflared", "tunnel", "cleanup", "t1"]);
    assert_ne!(out.exit_code, 255);
}

// tunnel token: NArg != 1 → 255
#[test]
fn tunnel_token_rejects_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "token"]);
    assert_eq!(out.exit_code, 255);
    assert!(out.stderr.contains(TUNNEL_TOKEN_NARG_ERROR_MSG));
}

#[test]
fn tunnel_token_rejects_two_args() {
    let out = exec(&["cloudflared", "tunnel", "token", "a", "b"]);
    assert_eq!(out.exit_code, 255);
    assert!(out.stderr.contains(TUNNEL_TOKEN_NARG_ERROR_MSG));
}

#[test]
fn tunnel_token_accepts_one_arg() {
    let out = exec(&["cloudflared", "tunnel", "token", "my-tunnel"]);
    assert_ne!(out.exit_code, 255);
}

// tunnel info: NArg != 1 → 255
#[test]
fn tunnel_info_rejects_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "info"]);
    assert_eq!(out.exit_code, 255);
    assert!(out.stderr.contains(TUNNEL_INFO_NARG_ERROR_MSG));
}

#[test]
fn tunnel_info_accepts_one_arg() {
    let out = exec(&["cloudflared", "tunnel", "info", "my-tunnel"]);
    assert_ne!(out.exit_code, 255);
}

// tunnel route dns: NArg != 2 → 255
#[test]
fn route_dns_rejects_one_arg() {
    let out = exec(&["cloudflared", "tunnel", "route", "dns", "my-tunnel"]);
    assert_eq!(out.exit_code, 255);
    assert!(out.stderr.contains(ROUTE_DNS_NARG_ERROR_MSG));
}

#[test]
fn route_dns_accepts_two_args() {
    let out = exec(&[
        "cloudflared",
        "tunnel",
        "route",
        "dns",
        "my-tunnel",
        "example.com",
    ]);
    assert_ne!(out.exit_code, 255);
}

// tunnel route lb: NArg != 3 → 255
#[test]
fn route_lb_rejects_two_args() {
    let out = exec(&["cloudflared", "tunnel", "route", "lb", "my-tunnel", "example.com"]);
    assert_eq!(out.exit_code, 255);
    assert!(out.stderr.contains(ROUTE_LB_NARG_ERROR_MSG));
}

#[test]
fn route_lb_accepts_three_args() {
    let out = exec(&[
        "cloudflared",
        "tunnel",
        "route",
        "lb",
        "my-tunnel",
        "example.com",
        "my-pool",
    ]);
    assert_ne!(out.exit_code, 255);
}

// --- Route IP subcommands (Go baseline: errors.New → exit 1) ---

#[test]
fn route_ip_add_rejects_one_arg() {
    let out = exec(&["cloudflared", "tunnel", "route", "ip", "add", "cidr"]);
    assert_eq!(out.exit_code, 1);
    assert!(out.stderr.contains(ROUTE_IP_ADD_NARG_ERROR_MSG));
}

#[test]
fn route_ip_add_accepts_two_args() {
    let out = exec(&[
        "cloudflared",
        "tunnel",
        "route",
        "ip",
        "add",
        "10.0.0.0/8",
        "my-tunnel",
    ]);
    assert!(
        !out.stderr.contains(ROUTE_IP_ADD_NARG_ERROR_MSG),
        "two positional args should pass NArg check"
    );
}

#[test]
fn route_ip_delete_rejects_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "route", "ip", "delete"]);
    assert_eq!(out.exit_code, 1);
    assert!(out.stderr.contains(ROUTE_IP_DELETE_NARG_ERROR_MSG));
}

#[test]
fn route_ip_delete_accepts_one_arg() {
    let out = exec(&["cloudflared", "tunnel", "route", "ip", "delete", "10.0.0.0/8"]);
    // Should not get NArg error — will be a stub error, exit 1, but different
    // message
    assert!(
        !out.stderr.contains(ROUTE_IP_DELETE_NARG_ERROR_MSG),
        "one arg should pass NArg check"
    );
}

#[test]
fn route_ip_get_rejects_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "route", "ip", "get"]);
    assert_eq!(out.exit_code, 1);
    assert!(out.stderr.contains(ROUTE_IP_GET_NARG_ERROR_MSG));
}

#[test]
fn route_ip_get_accepts_one_arg() {
    let out = exec(&["cloudflared", "tunnel", "route", "ip", "get", "10.0.0.1"]);
    assert!(
        !out.stderr.contains(ROUTE_IP_GET_NARG_ERROR_MSG),
        "one arg should pass NArg check"
    );
}

// --- Vnet subcommands (Go baseline: errors.New → exit 1) ---

#[test]
fn vnet_add_rejects_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "vnet", "add"]);
    assert_eq!(out.exit_code, 1);
    assert!(out.stderr.contains(VNET_ADD_NARG_ERROR_MSG));
}

#[test]
fn vnet_add_accepts_one_arg() {
    let out = exec(&["cloudflared", "tunnel", "vnet", "add", "my-vnet"]);
    assert!(!out.stderr.contains(VNET_ADD_NARG_ERROR_MSG));
}

#[test]
fn vnet_delete_rejects_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "vnet", "delete"]);
    assert_eq!(out.exit_code, 1);
    assert!(out.stderr.contains(VNET_DELETE_NARG_ERROR_MSG));
}

#[test]
fn vnet_delete_accepts_one_arg() {
    let out = exec(&["cloudflared", "tunnel", "vnet", "delete", "my-vnet"]);
    assert!(!out.stderr.contains(VNET_DELETE_NARG_ERROR_MSG));
}

#[test]
fn vnet_update_rejects_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "vnet", "update"]);
    assert_eq!(out.exit_code, 1);
    assert!(out.stderr.contains(VNET_UPDATE_NARG_ERROR_MSG));
}

#[test]
fn vnet_update_rejects_two_args() {
    let out = exec(&["cloudflared", "tunnel", "vnet", "update", "a", "b"]);
    assert_eq!(out.exit_code, 1);
    assert!(out.stderr.contains(VNET_UPDATE_NARG_ERROR_MSG));
}

#[test]
fn vnet_update_accepts_one_arg() {
    let out = exec(&["cloudflared", "tunnel", "vnet", "update", "my-vnet"]);
    assert!(!out.stderr.contains(VNET_UPDATE_NARG_ERROR_MSG));
}

// --- Ingress rule: empty args → exit 1 ---

#[test]
fn ingress_rule_rejects_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "ingress", "rule"]);
    assert_eq!(out.exit_code, 1);
    assert!(out.stderr.contains(INGRESS_RULE_NARG_ERROR_MSG));
}

#[test]
fn ingress_rule_accepts_one_arg() {
    let out = exec(&[
        "cloudflared",
        "tunnel",
        "ingress",
        "rule",
        "http://localhost:8080",
    ]);
    assert!(!out.stderr.contains(INGRESS_RULE_NARG_ERROR_MSG));
}

#[test]
fn tunnel_ready_requires_metrics_flag() {
    let out = exec(&["cloudflared", "tunnel", "ready"]);
    assert_eq!(out.exit_code, 1);
    assert!(out.stderr.contains("--metrics has to be provided"));
}

#[test]
fn tunnel_ready_returns_success_on_200() {
    let metrics_addr = serve_once("200 OK", "");
    let out = exec(&["cloudflared", "tunnel", "ready", "--metrics", &metrics_addr]);
    assert_eq!(out.exit_code, 0);
    assert!(
        out.stderr.is_empty(),
        "ready success should not emit stderr: {:?}",
        out.stderr
    );
}

#[test]
fn tunnel_ready_reports_non_200_body() {
    let metrics_addr = serve_once("503 Service Unavailable", "not ready");
    let out = exec(&["cloudflared", "tunnel", "ready", "--metrics", &metrics_addr]);
    assert_eq!(out.exit_code, 1);
    assert!(
        out.stderr
            .contains("endpoint returned status code 503\nnot ready"),
        "non-200 ready must include status and body: {:?}",
        out.stderr,
    );
}

#[test]
fn tunnel_diag_creates_zip_and_reports_completion() {
    let _guard = diag_test_lock().lock().expect("diag lock");
    let log_path = std::env::temp_dir().join("cfdrs-cli-diag.log");
    std::fs::write(&log_path, "logs\n").expect("write log");
    let server = MockDiagnosticServer::start(diagnostic_routes(&format!(
        "{{\"uid\":\"1000\",\"logfile\":\"{}\"}}",
        log_path.display()
    )));

    let out = exec(&["cloudflared", "tunnel", "diag", "--metrics", &server.address]);
    assert_eq!(out.exit_code, 0);
    assert!(
        out.stdout.contains(&format!(
            "Selected server http://{} starting diagnostic...",
            server.address
        )),
        "diag should announce selected server: {:?}",
        out.stdout
    );
    assert!(
        out.stdout.contains("Diagnostic completed"),
        "diag should report completion: {:?}",
        out.stdout
    );

    let zip_path = extract_diagnostic_zip_path(&out.stdout).expect("zip path");
    let zip_path = std::env::current_dir().expect("cwd").join(zip_path);
    assert!(zip_path.exists(), "zip must exist: {:?}", out.stdout);

    let _ = std::fs::remove_file(zip_path);
    let _ = std::fs::remove_file(log_path);
}

#[test]
fn tunnel_diag_invalid_log_configuration_is_nonfatal() {
    let _guard = diag_test_lock().lock().expect("diag lock");
    let server = MockDiagnosticServer::start(diagnostic_routes("{\"uid\":\"1000\"}"));

    let out = exec(&["cloudflared", "tunnel", "diag", "--metrics", &server.address]);
    assert_eq!(out.exit_code, 0);
    assert!(
        out.stdout.contains("Couldn't extract logs from the instance."),
        "diag should surface invalid log configuration hint: {:?}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("Diagnostic completed with one or more errors"),
        "diag should remain nonfatal on log config mismatch: {:?}",
        out.stdout
    );

    if let Some(zip_path) = extract_diagnostic_zip_path(&out.stdout) {
        let _ = std::fs::remove_file(std::env::current_dir().expect("cwd").join(zip_path));
    }
}

#[test]
fn tunnel_diag_no_instances_found_is_success() {
    let _guard = diag_test_lock().lock().expect("diag lock");
    let out = exec(&["cloudflared", "tunnel", "diag"]);
    assert_eq!(out.exit_code, 0);
    assert!(out.stdout.contains("No instances found"), "{:?}", out.stdout);
}

#[test]
fn tunnel_diag_reports_multiple_instances() {
    let _guard = diag_test_lock().lock().expect("diag lock");
    let _first = MockDiagnosticServer::start_on(
        "127.0.0.1:20241",
        diagnostic_routes("{\"uid\":\"1000\",\"logfile\":\"/tmp/a.log\"}"),
    );
    let _second = MockDiagnosticServer::start_on(
        "127.0.0.1:20242",
        diagnostic_routes("{\"uid\":\"1000\",\"logfile\":\"/tmp/b.log\"}"),
    );

    let out = exec(&["cloudflared", "tunnel", "diag"]);
    assert_eq!(out.exit_code, 0);
    assert!(
        out.stdout.contains("Found multiple instances running:"),
        "{:?}",
        out.stdout
    );
    assert!(
        out.stdout.contains("metrics-address=localhost:20241"),
        "{:?}",
        out.stdout
    );
    assert!(
        out.stdout.contains("metrics-address=localhost:20242"),
        "{:?}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("To select one instance use the option --metrics"),
        "{:?}",
        out.stdout
    );
}

// --- Subcommands with NO NArg validation in Go baseline ---

#[test]
fn tunnel_list_accepts_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "list"]);
    assert_ne!(out.exit_code, 255, "list has no NArg constraint");
}

/// Login dispatch no longer hits `stub_not_implemented` — it runs the
/// real login flow which requires network access.  The NArg constraint
/// (exit 255) was already validated by `tunnel_login::tests`.  This test
/// exercises the real dispatch path and needs network access.
#[test]
#[ignore = "requires network access — run manually with --include-ignored"]
fn tunnel_login_accepts_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "login"]);
    assert_ne!(out.exit_code, 255, "login has no NArg constraint");
}

#[test]
fn route_ip_show_accepts_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "route", "ip", "show"]);
    assert_ne!(out.exit_code, 255, "route ip show has no NArg constraint");
}

#[test]
fn vnet_list_accepts_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "vnet", "list"]);
    assert_ne!(out.exit_code, 255, "vnet list has no NArg constraint");
}

#[test]
fn ingress_validate_accepts_zero_args() {
    let out = exec(&["cloudflared", "tunnel", "ingress", "validate"]);
    assert_ne!(out.exit_code, 255, "ingress validate has no NArg constraint");
}

#[test]
fn ingress_validate_with_json_reports_ok() {
    let out = exec(&[
        "cloudflared",
        "tunnel",
        "ingress",
        "validate",
        "--json",
        "{\"ingress\":[{\"hostname\":\"app.example.com\",\"service\":\"https://localhost:8080\"},{\"service\":\"http_status:404\"}]}",
    ]);
    assert_eq!(out.exit_code, 0);
    assert!(out.stdout.contains("Validating rules from cmdline flag --json"));
    assert!(out.stdout.contains("OK"));
}

#[test]
fn ingress_validate_rejects_url_flag_with_rules() {
    let temp = TempDir::new().expect("tempdir");
    let config_path = write_config(
        &temp,
        "ingress:\n  - hostname: app.example.com\n    service: https://localhost:8080\n  - service: \
         http_status:404\n",
    );
    let out = exec(&[
        "cloudflared",
        "tunnel",
        "--config",
        &config_path,
        "--url",
        "http://localhost:9000",
        "ingress",
        "validate",
    ]);
    assert_eq!(out.exit_code, 1);
    assert!(out.stdout.contains("Validating rules from"));
    assert!(out.stderr.contains("You can't set the --url flag"));
}

#[test]
fn ingress_validate_rejects_empty_config() {
    let temp = TempDir::new().expect("tempdir");
    let config_path = write_config(&temp, "");
    let out = exec(&[
        "cloudflared",
        "tunnel",
        "--config",
        &config_path,
        "ingress",
        "validate",
    ]);
    assert_eq!(out.exit_code, 1);
    assert!(out.stdout.contains("Validating rules from"));
    assert!(
        out.stderr
            .contains("Validation failed: The config file doesn't contain any ingress rules")
    );
}

#[test]
fn ingress_rule_reports_matching_rule_from_config() {
    let temp = TempDir::new().expect("tempdir");
    let config_path = write_config(
        &temp,
        "ingress:\n  - hostname: app.example.com\n    path: /health\n    service: https://localhost:8080\n  \
         - service: http_status:404\n",
    );
    let out = exec(&[
        "cloudflared",
        "tunnel",
        "--config",
        &config_path,
        "ingress",
        "rule",
        "https://app.example.com/health",
    ]);
    assert_eq!(out.exit_code, 0);
    assert!(out.stdout.contains("Using rules from"));
    assert!(out.stdout.contains("Matched rule #0"));
    assert!(out.stdout.contains("\thostname: app.example.com"));
    assert!(out.stdout.contains("\tpath: /health"));
    assert!(out.stdout.contains("\tservice: https://localhost:8080"));
}

#[test]
fn ingress_rule_rejects_empty_config() {
    let temp = TempDir::new().expect("tempdir");
    let config_path = write_config(&temp, "");
    let out = exec(&[
        "cloudflared",
        "tunnel",
        "--config",
        &config_path,
        "ingress",
        "rule",
        "https://app.example.com/health",
    ]);
    assert_eq!(out.exit_code, 1);
    assert!(out.stdout.contains("Using rules from"));
    assert!(
        out.stderr
            .contains("Validation failed: The config file doesn't contain any ingress rules")
    );
}

#[test]
fn ingress_rule_suggests_scheme_for_bare_hostname() {
    let out = exec(&["cloudflared", "tunnel", "ingress", "rule", "app.example.com"]);
    assert_eq!(out.exit_code, 1);
    assert!(
        out.stderr
            .contains("doesn't have a hostname, consider adding a scheme")
    );
}

// --- exit 255 help suffix verification ---

#[test]
fn usage_error_includes_help_suffix() {
    let out = exec(&["cloudflared", "tunnel", "create"]);
    assert_eq!(out.exit_code, 255);
    assert!(
        out.stderr.contains("See 'cloudflared tunnel create --help'."),
        "UsageError must include help suffix: {:?}",
        out.stderr
    );
}

// --- CLI-012 / CLI-032: tunnel run token precedence and identity
// validation ---

#[test]
fn tunnel_run_with_valid_token_does_not_reject() {
    // Go baseline: --token <valid> → runWithCredentials (no error from token
    // path)
    let token = encoded_tunnel_token();
    let out = exec(&["cloudflared", "tunnel", "run", "--token", &token]);
    // Should proceed past token validation into startup (may get config error,
    // not token error)
    assert!(
        !out.stderr.contains(TUNNEL_TOKEN_INVALID_MSG),
        "valid token must not trigger invalid-token error"
    );
}

#[test]
fn tunnel_run_with_invalid_token_exits_255() {
    // Go baseline: ParseToken fails → "Provided Tunnel token is not valid."
    // exit -1 (255)
    let out = exec(&["cloudflared", "tunnel", "run", "--token", "not-a-valid-token"]);
    assert_eq!(out.exit_code, 255);
    assert!(
        out.stderr.contains(TUNNEL_TOKEN_INVALID_MSG),
        "stderr must contain invalid token msg: {:?}",
        out.stderr
    );
    assert!(
        out.stderr.contains("See 'cloudflared tunnel run --help'."),
        "stderr must contain help suffix: {:?}",
        out.stderr
    );
}

#[test]
fn tunnel_run_with_empty_token_falls_through() {
    // Go baseline: tokenStr == "" → falls through to positional arg / config
    let out = exec(&["cloudflared", "tunnel", "run", "--token", ""]);
    // Empty token → treated as no token → falls through to identity check
    assert!(
        !out.stderr.contains(TUNNEL_TOKEN_INVALID_MSG),
        "empty token must not trigger invalid-token error"
    );
}

#[test]
fn tunnel_run_with_token_file_reads_token() {
    // Go baseline: --token-file → read file → use as tokenStr
    let token = encoded_tunnel_token();
    let token_path = std::env::temp_dir().join("cfdrs-run-token-file-test.txt");
    std::fs::write(&token_path, format!("{token}\n")).expect("write token file");

    let path_str = token_path.to_str().expect("path to str");
    let out = exec(&["cloudflared", "tunnel", "run", "--token-file", path_str]);

    // Valid token from file → should not get token error
    assert!(
        !out.stderr.contains(TUNNEL_TOKEN_INVALID_MSG),
        "token from file must not trigger invalid-token error"
    );
    let _ = std::fs::remove_file(token_path);
}

#[test]
fn tunnel_run_with_invalid_token_file_exits_255() {
    // Go baseline: os.ReadFile fails → "Failed to read token file: <err>"
    let out = exec(&[
        "cloudflared",
        "tunnel",
        "run",
        "--token-file",
        "/nonexistent/path/to/token",
    ]);
    assert_eq!(out.exit_code, 255);
    assert!(
        out.stderr.contains(TUNNEL_TOKEN_FILE_READ_ERROR_PREFIX),
        "stderr must contain token file read error prefix: {:?}",
        out.stderr
    );
}

#[test]
fn tunnel_run_token_flag_takes_precedence_over_token_file() {
    // Go baseline: --token is checked before --token-file
    let bad_file = std::env::temp_dir().join("cfdrs-run-token-precedence.txt");
    std::fs::write(&bad_file, "garbage-not-a-token\n").expect("write bad token file");

    let valid_token = encoded_tunnel_token();
    let path_str = bad_file.to_str().expect("path to str");
    let out = exec(&[
        "cloudflared",
        "tunnel",
        "run",
        "--token",
        &valid_token,
        "--token-file",
        path_str,
    ]);

    // --token is valid → should not get invalid-token error (--token-file is
    // ignored)
    assert!(
        !out.stderr.contains(TUNNEL_TOKEN_INVALID_MSG),
        "valid --token must take precedence over --token-file"
    );
    let _ = std::fs::remove_file(bad_file);
}

#[test]
fn tunnel_run_no_identity_exits_255() {
    // Go baseline: no token, no positional arg, no config tunnel ID →
    // "requires the ID or name" error, exit -1 (255)
    let out = exec(&["cloudflared", "tunnel", "run"]);
    assert_eq!(
        out.exit_code, 255,
        "no identity must exit 255, got: exit={} stderr={:?}",
        out.exit_code, out.stderr
    );
    assert!(
        out.stderr.contains("requires the ID or name"),
        "stderr must contain identity error: {:?}",
        out.stderr
    );
}

#[test]
fn bare_run_and_tunnel_run_both_dispatch() {
    // Go baseline: "cloudflared run" and "cloudflared tunnel run" produce
    // identical dispatch
    let out_bare = exec(&["cloudflared", "run"]);
    let out_tunnel = exec(&["cloudflared", "tunnel", "run"]);
    // Both should reach the same identity error (no token, no config)
    assert_eq!(out_bare.exit_code, out_tunnel.exit_code);
}

// --- CLI-001: service mode (bare invocation) ---
//
// Go baseline: handleServiceMode() in main.go — empty invocation enters
// service mode via FindOrCreateConfigPath() → AppManager → AppService.Run().
// Rust equivalent: ServiceMode → execute_startup_command() → resolve_startup()
// → execute_runtime_command() → ApplicationRuntime::run() with signal bridge,
// config watcher, and primary service (equivalent to Go AppManager pattern).

#[test]
fn service_mode_enters_config_discovery_not_help() {
    // Go baseline: empty invocation enters handleServiceMode(), never
    // prints help.  Without a discoverable config, Go calls
    // FindOrCreateConfigPath() which creates a default or fails.
    let out = exec(&["cloudflared"]);

    // Must not produce help output (Go handleServiceMode ≠ help).
    assert!(
        !out.stdout.contains("USAGE:"),
        "bare invocation must not produce help output"
    );
    assert!(
        !out.stdout.contains("COMMANDS:"),
        "bare invocation must not produce command listing"
    );
}

#[test]
fn service_mode_without_config_produces_config_error() {
    // Go baseline: handleServiceMode() calls FindOrCreateConfigPath().
    // Without a discoverable config file and no --config flag, the path
    // either creates a default or fails with a config-related error.
    // The key invariant: it does NOT produce a stub error or help.
    let out = exec(&["cloudflared"]);

    assert!(
        !out.stderr.contains("not yet implemented"),
        "service mode must not be a stub"
    );
    // Exit code 1 matches Go config-error path.
    assert_eq!(out.exit_code, 1, "config discovery failure should exit 1");
}

#[test]
fn service_mode_with_config_reaches_runtime() {
    // Go baseline: handleServiceMode() with a valid config enters
    // AppManager → AppService → actionLoop().
    // Rust equivalent: reaches ApplicationRuntime::run() which produces
    // deployment contract output.
    let config_path = std::env::temp_dir().join("cfdrs-cli001-runtime-test.yml");
    std::fs::write(
        &config_path,
        "tunnel: 00000000-0000-0000-0000-000000000000\ningress:\n  - service: http_status:503\n",
    )
    .expect("write test config");

    let path_str = config_path.to_str().expect("path to str");
    let out = exec(&["cloudflared", "--config", path_str]);

    // Must reach the runtime — stdout contains deployment contract output.
    assert!(
        out.stdout.contains("config-source:"),
        "bare invocation with config must reach runtime startup: {:?}",
        out.stdout,
    );
    assert!(
        out.stdout.contains("deploy-contract:"),
        "bare invocation with config must produce deployment contract output: {:?}",
        out.stdout,
    );
    let _ = std::fs::remove_file(config_path);
}

#[test]
fn service_mode_with_config_shows_ingress_rules() {
    // Go baseline: runtime logs show ingress configuration.
    // Rust: startup surface renders ingress rule count.
    let config_path = std::env::temp_dir().join("cfdrs-cli001-ingress-test.yml");
    std::fs::write(
        &config_path,
        "tunnel: 00000000-0000-0000-0000-000000000000\ningress:\n  - service: http_status:503\n",
    )
    .expect("write test config");

    let path_str = config_path.to_str().expect("path to str");
    let out = exec(&["cloudflared", "--config", path_str]);

    assert!(
        out.stdout.contains("ingress-rules: 1"),
        "runtime output must show ingress rule count: {:?}",
        out.stdout,
    );
    let _ = std::fs::remove_file(config_path);
}

#[test]
fn service_mode_and_tunnel_run_with_config_reach_same_runtime() {
    // Go baseline: handleServiceMode() and `tunnel run` with the same
    // config both reach the same runtime path.
    let config_path = std::env::temp_dir().join("cfdrs-cli001-equivalence-test.yml");
    std::fs::write(
        &config_path,
        "tunnel: 00000000-0000-0000-0000-000000000000\ningress:\n  - service: http_status:503\n",
    )
    .expect("write test config");

    let path_str = config_path.to_str().expect("path to str");
    let out_bare = exec(&["cloudflared", "--config", path_str]);
    let out_run = exec(&["cloudflared", "tunnel", "run", "--config", path_str]);

    // Both paths must reach the runtime and produce deployment contract.
    assert!(
        out_bare.stdout.contains("deploy-contract:"),
        "bare invocation must reach runtime"
    );
    assert!(
        out_run.stdout.contains("deploy-contract:"),
        "tunnel run must reach runtime"
    );
    // Same exit code from the runtime.
    assert_eq!(
        out_bare.exit_code, out_run.exit_code,
        "bare invocation and tunnel run must produce same exit code"
    );
    let _ = std::fs::remove_file(config_path);
}

// --- CLI-022: access subtree dispatch ---

#[test]
fn access_bare_shows_help() {
    // Go baseline: bare `access` with no subcommand shows access help
    // (urfave/cli default for commands with subcommands).
    let out = exec(&["cloudflared", "access"]);
    assert_eq!(out.exit_code, 0);
    assert!(
        out.stdout.contains("access"),
        "bare access must show access help text: {:?}",
        out.stdout,
    );
}

#[test]
fn forward_alias_shows_access_help() {
    // Go baseline: `forward` is an alias for `access`.
    let out = exec(&["cloudflared", "forward"]);
    assert_eq!(out.exit_code, 0);
    assert!(
        out.stdout.contains("access"),
        "forward alias must show access help text: {:?}",
        out.stdout,
    );
}

#[test]
fn access_bare_and_forward_produce_same_output() {
    let out_access = exec(&["cloudflared", "access"]);
    let out_forward = exec(&["cloudflared", "forward"]);
    assert_eq!(out_access.stdout, out_forward.stdout);
    assert_eq!(out_access.exit_code, out_forward.exit_code);
}

#[test]
fn access_login_reaches_explicit_deferred_boundary() {
    let out = exec(&["cloudflared", "access", "login"]);
    assert_eq!(out.exit_code, 1);
    assert!(
        out.stderr.contains("browser-based Access token flow"),
        "access login must explain the deferred browser flow: {:?}",
        out.stderr,
    );
    assert!(
        !out.stderr.contains("not yet implemented in the Rust rewrite"),
        "access login should no longer use the generic placeholder stub: {:?}",
        out.stderr,
    );
}

#[test]
fn access_curl_reaches_explicit_deferred_boundary() {
    let out = exec(&["cloudflared", "access", "curl"]);
    assert_eq!(out.exit_code, 1);
    assert!(
        out.stderr.contains("JWT header injection path"),
        "access curl must explain the deferred JWT wrapper path: {:?}",
        out.stderr,
    );
}

#[test]
fn access_token_reaches_explicit_deferred_boundary() {
    let out = exec(&["cloudflared", "access", "token"]);
    assert_eq!(out.exit_code, 1);
    assert!(
        out.stderr.contains("token storage and retrieval"),
        "access token must explain the deferred token path: {:?}",
        out.stderr,
    );
}

#[test]
fn access_tcp_reaches_explicit_deferred_boundary() {
    let out = exec(&["cloudflared", "access", "tcp"]);
    assert_eq!(out.exit_code, 1);
    assert!(
        out.stderr.contains("carrier WebSocket proxy/client path"),
        "access tcp must explain the deferred carrier path: {:?}",
        out.stderr,
    );
}

#[test]
fn access_rdp_alias_dispatches_same_as_tcp() {
    // Go baseline: rdp/ssh/smb are aliases for tcp.
    let out_tcp = exec(&["cloudflared", "access", "tcp"]);
    let out_rdp = exec(&["cloudflared", "access", "rdp"]);
    assert_eq!(out_tcp.exit_code, out_rdp.exit_code);
    assert_eq!(out_tcp.stderr, out_rdp.stderr);
}

#[test]
fn access_ssh_alias_dispatches_same_as_tcp() {
    let out_tcp = exec(&["cloudflared", "access", "tcp"]);
    let out_ssh = exec(&["cloudflared", "access", "ssh"]);
    assert_eq!(out_tcp.exit_code, out_ssh.exit_code);
    assert_eq!(out_tcp.stderr, out_ssh.stderr);
}

#[test]
fn access_smb_alias_dispatches_same_as_tcp() {
    let out_tcp = exec(&["cloudflared", "access", "tcp"]);
    let out_smb = exec(&["cloudflared", "access", "smb"]);
    assert_eq!(out_tcp.exit_code, out_smb.exit_code);
    assert_eq!(out_tcp.stderr, out_smb.stderr);
}

#[test]
fn access_ssh_config_renders_real_output() {
    let out = exec(&["cloudflared", "access", "ssh-config"]);
    assert_eq!(out.exit_code, 0);
    assert!(
        out.stdout.contains("ProxyCommand"),
        "access ssh-config must render an SSH config snippet: {:?}",
        out.stdout,
    );
    assert!(
        out.stdout.contains("access ssh --hostname %h"),
        "access ssh-config must reference the access ssh ProxyCommand: {:?}",
        out.stdout,
    );
}

#[test]
fn access_ssh_config_supports_short_lived_cert_flag() {
    let out = exec(&[
        "cloudflared",
        "access",
        "ssh-config",
        "--hostname",
        "ssh.example.com",
        "--short-lived-cert",
    ]);
    assert_eq!(out.exit_code, 0);
    assert!(
        out.stdout.contains("Match host ssh.example.com"),
        "short-lived cert mode must use Match host template: {:?}",
        out.stdout,
    );
    assert!(
        out.stdout.contains("access ssh-gen --hostname %h"),
        "short-lived cert mode must reference ssh-gen: {:?}",
        out.stdout,
    );
}

#[test]
fn access_ssh_gen_reaches_explicit_deferred_boundary() {
    let out = exec(&["cloudflared", "access", "ssh-gen"]);
    assert_eq!(out.exit_code, 1);
    assert!(
        out.stderr.contains("short-lived SSH certificate generation"),
        "access ssh-gen must explain the deferred SSH cert path: {:?}",
        out.stderr,
    );
}

// --- CLI-023: tail command dispatch ---

#[test]
fn tail_bare_dispatches_to_behavioral_implementation() {
    // Bare `tail` without a tunnel ID or token falls through to URL building
    // which needs either `--token` or origin cert for API token acquisition.
    // Without either, the error from build_client is expected.
    let out = exec(&["cloudflared", "tail"]);
    assert_eq!(out.exit_code, 1);
    // The tail streaming path needs a management URL which requires cert/token.
    assert!(
        out.stderr.contains("cert") || out.stderr.contains("tunnel"),
        "tail dispatch reached behavioral code: {:?}",
        out.stderr,
    );
}

#[test]
fn tail_token_dispatches_to_behavioral_implementation() {
    // `tail token` without origin cert fails at build_client.
    let out = exec(&["cloudflared", "tail", "token"]);
    assert_eq!(out.exit_code, 1);
    assert!(
        out.stderr.contains("cert") || out.stderr.contains("tunnel"),
        "tail token dispatch reached behavioral code: {:?}",
        out.stderr,
    );
}

// --- CLI-024: management command dispatch ---

#[test]
fn management_bare_shows_help() {
    let out = exec(&["cloudflared", "management"]);
    assert_eq!(out.exit_code, 0);
    assert!(
        out.stdout
            .contains("cloudflared management - Monitor cloudflared tunnels via management API"),
        "management help must describe the hidden command: {:?}",
        out.stdout,
    );
}

#[test]
fn management_token_dispatches_to_behavioral_implementation() {
    // `management token` without origin cert fails at build_client.
    let out = exec(&["cloudflared", "management", "token"]);
    assert_eq!(out.exit_code, 1);
    assert!(
        out.stderr.contains("cert") || out.stderr.contains("tunnel"),
        "management token dispatch reached behavioral code: {:?}",
        out.stderr,
    );
}

#[test]
fn management_token_help_shows_hidden_subcommand_flags() {
    let out = exec(&["cloudflared", "management", "token", "--help"]);
    assert_eq!(out.exit_code, 0);
    assert!(
        out.stdout.contains("--resource value"),
        "management token help must list the required resource flag: {:?}",
        out.stdout,
    );
}
