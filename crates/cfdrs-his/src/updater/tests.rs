use super::*;

use std::collections::VecDeque;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

struct MockUpdateServer {
    address: String,
    requests: Arc<Mutex<Vec<String>>>,
    responses: Arc<Mutex<VecDeque<(String, String, String)>>>,
    join: Option<thread::JoinHandle<()>>,
}

impl MockUpdateServer {
    fn start(responses: Vec<(String, String, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("addr").to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let request_log = Arc::clone(&requests);
        let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
        let server_responses = Arc::clone(&responses);

        let join = thread::spawn(move || {
            while let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0_u8; 8192];
                let read = stream.read(&mut buffer).expect("read request");
                let request = String::from_utf8_lossy(&buffer[..read]).into_owned();
                request_log.lock().expect("request log").push(request.clone());
                let Some((status_line, content_type, body)) =
                    server_responses.lock().expect("responses").pop_front()
                else {
                    break;
                };
                let response = format!(
                    "HTTP/1.1 {status_line}\r\nContent-Length: {}\r\nContent-Type: \
                     {content_type}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).expect("write response");
            }
        });

        Self {
            address,
            requests,
            responses,
            join: Some(join),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.address, path)
    }

    fn push_response(&self, status_line: &str, content_type: &str, body: String) {
        self.responses.lock().expect("responses").push_back((
            status_line.to_owned(),
            content_type.to_owned(),
            body,
        ));
    }

    fn first_request_line(&self) -> String {
        self.requests
            .lock()
            .expect("request log")
            .first()
            .and_then(|request| request.lines().next())
            .expect("request line")
            .to_owned()
    }
}

impl Drop for MockUpdateServer {
    fn drop(&mut self) {
        let _ = std::net::TcpStream::connect(&self.address);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

#[test]
fn default_autoupdate_freq_is_24h() {
    assert_eq!(DEFAULT_AUTOUPDATE_FREQ, Duration::from_secs(86400));
}

#[test]
fn parse_auto_update_freq_defaults_when_unset() {
    assert_eq!(
        parse_auto_update_freq(None).expect("default auto-update frequency should parse"),
        DEFAULT_AUTOUPDATE_FREQ
    );
}

#[test]
fn parse_auto_update_freq_accepts_go_style_sequences() {
    assert_eq!(
        parse_auto_update_freq(Some("1h30m")).expect("go-style duration should parse"),
        Duration::from_secs(5400)
    );
    assert_eq!(
        parse_auto_update_freq(Some("0")).expect("zero should parse"),
        Duration::ZERO
    );
}

#[test]
fn parse_auto_update_freq_rejects_invalid_duration() {
    let error = parse_auto_update_freq(Some("30")).expect_err("bare number should fail");
    assert!(error.to_string().contains("invalid autoupdate-freq duration"));
}

#[test]
fn resolve_auto_update_settings_defaults_to_enabled_24h() {
    let settings = resolve_auto_update_settings(false, None, false, false, "linux");
    assert!(settings.enabled());
    assert_eq!(settings.frequency(), DEFAULT_AUTOUPDATE_FREQ);
    assert_eq!(settings.disabled_reason(), None);
}

#[test]
fn resolve_auto_update_settings_disables_on_terminal() {
    let settings = resolve_auto_update_settings(false, Some(Duration::from_secs(5)), false, true, "linux");
    assert!(!settings.enabled());
    assert_eq!(settings.frequency(), DEFAULT_AUTOUPDATE_FREQ);
    assert_eq!(settings.disabled_reason(), Some(NO_AUTO_UPDATE_IN_SHELL_MESSAGE));
}

#[test]
fn resolve_auto_update_settings_disables_for_package_managed_install() {
    let settings = resolve_auto_update_settings(false, Some(Duration::from_secs(5)), true, false, "linux");
    assert!(!settings.enabled());
    assert_eq!(
        settings.disabled_reason(),
        Some(NO_AUTO_UPDATE_MANAGED_PACKAGE_MESSAGE)
    );
}

#[test]
fn resolve_auto_update_settings_disables_for_zero_frequency() {
    let settings = resolve_auto_update_settings(false, Some(Duration::ZERO), false, false, "linux");
    assert!(!settings.enabled());
    assert_eq!(
        settings.disabled_reason(),
        Some(NO_AUTO_UPDATE_DISABLED_FLAG_MESSAGE)
    );
}

#[test]
fn stub_updater_returns_deferred() {
    let updater = StubUpdater;
    assert!(updater.check().is_err());
}

#[test]
fn update_exit_success_is_11() {
    assert_eq!(UPDATE_EXIT_SUCCESS, 11);
}

#[test]
fn update_exit_failure_is_10() {
    assert_eq!(UPDATE_EXIT_FAILURE, 10);
}

#[test]
fn marker_path_matches_go_postinst() {
    assert_eq!(
        crate::environment::INSTALLED_FROM_PACKAGE_MARKER,
        "/usr/local/etc/cloudflared/.installedFromPackageManager",
    );
}

#[test]
fn should_skip_update_delegates_to_package_managed() {
    assert_eq!(should_skip_update(), crate::environment::is_package_managed(),);
}

#[test]
fn update_server_matches_go() {
    assert_eq!(UPDATE_SERVER, "https://update.argotunnel.com");
    assert_eq!(STAGING_UPDATE_SERVER, "https://staging-update.argotunnel.com");
}

#[test]
fn workers_request_uses_expected_query_parameters() {
    let request = WorkersUpdateRequest::new(
        "2026.2.0",
        PathBuf::from("/tmp/cloudflared"),
        true,
        false,
        true,
        Some("2026.2.1".to_owned()),
    );

    let url = request.request_url().expect("request url");
    let query = url.query_pairs().collect::<Vec<_>>();

    assert!(query.contains(&("os".into(), "linux".into())));
    assert!(query.contains(&("arch".into(), "amd64".into())));
    assert!(query.contains(&("clientVersion".into(), "2026.2.0".into())));
    assert!(query.contains(&("beta".into(), "true".into())));
    assert!(query.contains(&("version".into(), "2026.2.1".into())));
    assert!(
        !url.as_str().contains("force"),
        "force must remain accepted but not serialized"
    );
}

#[test]
fn workers_request_uses_staging_server_when_requested() {
    let request = WorkersUpdateRequest::new(
        "2026.2.0",
        PathBuf::from("/tmp/cloudflared"),
        false,
        true,
        false,
        None,
    );

    assert_eq!(request.base_url, STAGING_UPDATE_SERVER);
}

#[test]
fn workers_updater_rejects_non_200_status() {
    let server = MockUpdateServer::start(vec![(
        "503 Service Unavailable".to_owned(),
        "application/json".to_owned(),
        "{}".to_owned(),
    )]);
    let request = WorkersUpdateRequest {
        current_version: "2026.2.0".to_owned(),
        base_url: server.url("/check"),
        target_path: PathBuf::from("/tmp/cloudflared"),
        is_beta: false,
        is_forced: false,
        intended_version: None,
    };
    let updater = WorkersUpdater::new(request).expect("updater");

    let error = updater.check().expect_err("non-200 must fail");
    assert!(error.to_string().contains("unable to check for update: 503"));
}

#[test]
fn workers_updater_returns_no_update_when_service_says_false() {
    let server = MockUpdateServer::start(vec![(
        "200 OK".to_owned(),
        "application/json".to_owned(),
        r#"{"url":"https://example.com/cloudflared","version":"","checksum":"","compressed":false,"userMessage":"already current","shouldUpdate":false,"error":""}"#
            .to_owned(),
    )]);
    let request = WorkersUpdateRequest {
        current_version: "2026.2.0".to_owned(),
        base_url: server.url("/check"),
        target_path: PathBuf::from("/tmp/cloudflared"),
        is_beta: false,
        is_forced: false,
        intended_version: None,
    };
    let updater = WorkersUpdater::new(request).expect("updater");

    let check = updater.check().expect("no-update check");
    assert!(check.update.is_none());
    assert_eq!(check.user_message.as_deref(), Some("already current"));
    assert!(server.first_request_line().contains("clientVersion=2026.2.0"));
}

#[test]
fn workers_updater_downloads_and_replaces_binary() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let target = tempdir.path().join("cloudflared");
    let current_contents = b"old-binary";
    let next_contents = b"new-binary";
    fs::write(&target, current_contents).expect("write current binary");

    let checksum = sha256_hex(next_contents);
    let server = MockUpdateServer::start(Vec::new());
    let payload_url = server.url("/artifact");
    let check_body = format!(
        "{{\"url\":\"{payload_url}\",\"version\":\"2026.2.1\",\"checksum\":\"{checksum}\",\"compressed\":\
         false,\"userMessage\":\"\",\"shouldUpdate\":true,\"error\":\"\"}}"
    );
    server.push_response("200 OK", "application/json", check_body);
    server.push_response(
        "200 OK",
        "application/octet-stream",
        String::from_utf8_lossy(next_contents).into_owned(),
    );
    let request = WorkersUpdateRequest {
        current_version: "2026.2.0".to_owned(),
        base_url: server.url("/check"),
        target_path: target.clone(),
        is_beta: false,
        is_forced: false,
        intended_version: None,
    };
    let updater = WorkersUpdater::new(request).expect("updater");

    let check = updater.check().expect("check");
    let update = check.update.expect("update");
    updater.apply(&update).expect("apply");

    assert_eq!(fs::read(&target).expect("target bytes"), next_contents);
    assert!(!suffixed_path(&target, ".new").exists());
    assert!(!suffixed_path(&target, ".old").exists());
}

#[test]
fn workers_updater_rejects_checksum_mismatch() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let target = tempdir.path().join("cloudflared");
    fs::write(&target, b"old-binary").expect("write current binary");
    let next_contents = b"new-binary";
    let server = MockUpdateServer::start(Vec::new());
    let check_body = format!(
        "{{\"url\":\"{}\",\"version\":\"2026.2.1\",\"checksum\":\"deadbeef\",\"compressed\":false,\"\
         userMessage\":\"\",\"shouldUpdate\":true,\"error\":\"\"}}",
        server.url("/artifact")
    );
    server.push_response("200 OK", "application/json", check_body);
    server.push_response(
        "200 OK",
        "application/octet-stream",
        String::from_utf8_lossy(next_contents).into_owned(),
    );
    let request = WorkersUpdateRequest {
        current_version: "2026.2.0".to_owned(),
        base_url: server.url("/check"),
        target_path: target,
        is_beta: false,
        is_forced: false,
        intended_version: None,
    };
    let updater = WorkersUpdater::new(request).expect("updater");

    let check = updater.check().expect("check");
    let error = updater
        .apply(&check.update.expect("update"))
        .expect_err("checksum mismatch must fail");
    assert!(error.to_string().contains("checksum validation failed"));
}

#[test]
fn workers_updater_rejects_matching_current_binary_checksum() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let target = tempdir.path().join("cloudflared");
    let contents = b"same-binary";
    fs::write(&target, contents).expect("write current binary");
    let checksum = sha256_hex(contents);
    let server = MockUpdateServer::start(Vec::new());
    let check_body = format!(
        "{{\"url\":\"{}\",\"version\":\"2026.2.1\",\"checksum\":\"{checksum}\",\"compressed\":false,\"\
         userMessage\":\"\",\"shouldUpdate\":true,\"error\":\"\"}}",
        server.url("/artifact")
    );
    server.push_response("200 OK", "application/json", check_body);
    server.push_response(
        "200 OK",
        "application/octet-stream",
        String::from_utf8_lossy(contents).into_owned(),
    );
    let request = WorkersUpdateRequest {
        current_version: "2026.2.0".to_owned(),
        base_url: server.url("/check"),
        target_path: target,
        is_beta: false,
        is_forced: false,
        intended_version: None,
    };
    let updater = WorkersUpdater::new(request).expect("updater");

    let check = updater.check().expect("check");
    let error = updater
        .apply(&check.update.expect("update"))
        .expect_err("same checksum must fail");
    assert!(
        error
            .to_string()
            .contains("checksum validation matches currently running process")
    );
}

#[test]
fn run_manual_update_short_circuits_for_package_managed() {
    let outcome = run_manual_update(&StubUpdater, true).expect("package-managed short circuit");
    assert!(matches!(outcome, ManualUpdateOutcome::PackageManaged { .. }));
}

#[test]
fn update_arch_matches_go_for_linux_x86_64() {
    assert_eq!(update_arch(), "amd64");
}
