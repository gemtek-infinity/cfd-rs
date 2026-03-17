//! Self-update and auto-update contracts.
//!
//! Covers HIS-046 through HIS-049.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use flate2::read::GzDecoder;
use reqwest::Url;
use reqwest::blocking::{Client, Response};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tar::Archive;

use cfdrs_shared::{ConfigError, Result};

const UPDATE_CLIENT_TIMEOUT: Duration = Duration::from_secs(60);
const UPDATE_SERVER_OS_KEY: &str = "os";
const UPDATE_SERVER_ARCH_KEY: &str = "arch";
const UPDATE_SERVER_BETA_KEY: &str = "beta";
const UPDATE_SERVER_VERSION_KEY: &str = "version";
const UPDATE_SERVER_CLIENT_VERSION_KEY: &str = "clientVersion";

// --- HIS-046: update command ---

/// Update server URL.
pub const UPDATE_SERVER: &str = "https://update.argotunnel.com";

/// Staging update server URL.
pub const STAGING_UPDATE_SERVER: &str = "https://staging-update.argotunnel.com";

/// Trait for the update check/apply contract.
pub trait Updater: Send + Sync {
    /// Check for an available update.
    fn check(&self) -> Result<UpdateCheck>;

    /// Apply an update (download and replace binary).
    fn apply(&self, info: &UpdateInfo) -> Result<()>;
}

/// Result of contacting the update service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheck {
    pub update: Option<UpdateInfo>,
    pub user_message: Option<String>,
}

/// Information about an available update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
    pub checksum: String,
    pub compressed: bool,
    pub user_message: Option<String>,
}

/// Inputs needed to talk to the Workers update service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkersUpdateRequest {
    pub current_version: String,
    pub base_url: String,
    pub target_path: PathBuf,
    pub is_beta: bool,
    pub is_forced: bool,
    pub intended_version: Option<String>,
}

impl WorkersUpdateRequest {
    pub fn new(
        current_version: impl Into<String>,
        target_path: PathBuf,
        is_beta: bool,
        is_staging: bool,
        is_forced: bool,
        intended_version: Option<String>,
    ) -> Self {
        Self {
            current_version: current_version.into(),
            base_url: if is_staging {
                STAGING_UPDATE_SERVER.to_owned()
            } else {
                UPDATE_SERVER.to_owned()
            },
            target_path,
            is_beta,
            is_forced,
            intended_version,
        }
    }

    pub fn request_url(&self) -> Result<Url> {
        let mut url = Url::parse(&self.base_url)
            .map_err(|error| ConfigError::invalid_url("update-server", &self.base_url, error))?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair(UPDATE_SERVER_OS_KEY, crate::environment::TARGET_OS);
            query.append_pair(UPDATE_SERVER_ARCH_KEY, update_arch());
            query.append_pair(UPDATE_SERVER_CLIENT_VERSION_KEY, &self.current_version);
            if self.is_beta {
                query.append_pair(UPDATE_SERVER_BETA_KEY, "true");
            }
            if let Some(version) = self.intended_version.as_deref()
                && !version.is_empty()
            {
                query.append_pair(UPDATE_SERVER_VERSION_KEY, version);
            }
        }
        Ok(url)
    }
}

/// Concrete updater backed by the Workers update service.
pub struct WorkersUpdater {
    client: Client,
    request: WorkersUpdateRequest,
}

impl WorkersUpdater {
    pub fn new(request: WorkersUpdateRequest) -> Result<Self> {
        let client = Client::builder()
            .timeout(UPDATE_CLIENT_TIMEOUT)
            .build()
            .map_err(|error| ConfigError::invariant(format!("failed to build update client: {error}")))?;
        Ok(Self { client, request })
    }
}

impl Updater for WorkersUpdater {
    fn check(&self) -> Result<UpdateCheck> {
        let response = self
            .client
            .get(self.request.request_url()?)
            .send()
            .map_err(|error| ConfigError::invariant(format!("update check failed: {error}")))?;

        if !response.status().is_success() {
            return Err(ConfigError::invariant(format!(
                "unable to check for update: {}",
                response.status().as_u16()
            )));
        }

        let payload: UpdateServiceResponse = response
            .json()
            .map_err(|error| ConfigError::invariant(format!("failed to decode update response: {error}")))?;

        if !payload.error.is_empty() {
            return Err(ConfigError::invariant(payload.error));
        }

        let user_message = normalize_optional_message(payload.user_message);
        if !payload.should_update || payload.version.is_empty() {
            return Ok(UpdateCheck {
                update: None,
                user_message,
            });
        }

        Ok(UpdateCheck {
            update: Some(UpdateInfo {
                version: payload.version,
                url: payload.url,
                checksum: payload.checksum,
                compressed: payload.compressed,
                user_message: user_message.clone(),
            }),
            user_message,
        })
    }

    fn apply(&self, info: &UpdateInfo) -> Result<()> {
        let new_path = suffixed_path(&self.request.target_path, ".new");
        let old_path = suffixed_path(&self.request.target_path, ".old");
        let _ = fs::remove_file(&new_path);
        let _ = fs::remove_file(&old_path);

        download_update(&self.client, &info.url, &new_path, info.compressed)?;

        let downloaded_checksum = file_checksum_hex(&new_path)?;
        if !downloaded_checksum.eq_ignore_ascii_case(&info.checksum) {
            let _ = fs::remove_file(&new_path);
            return Err(ConfigError::invariant("checksum validation failed"));
        }

        let current_checksum = file_checksum_hex(&self.request.target_path)?;
        if downloaded_checksum.eq_ignore_ascii_case(&current_checksum) {
            let _ = fs::remove_file(&new_path);
            return Err(ConfigError::invariant(
                "checksum validation matches currently running process",
            ));
        }

        fs::rename(&self.request.target_path, &old_path).map_err(|error| {
            ConfigError::invariant(format!(
                "failed to prepare binary replacement for {}: {error}",
                self.request.target_path.display()
            ))
        })?;

        if let Err(error) = fs::rename(&new_path, &self.request.target_path) {
            let _ = fs::rename(&old_path, &self.request.target_path);
            return Err(ConfigError::invariant(format!(
                "failed to install updated binary to {}: {error}",
                self.request.target_path.display()
            )));
        }

        let _ = fs::remove_file(&old_path);
        Ok(())
    }
}

/// Outcome of a manual `cloudflared update` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManualUpdateOutcome {
    PackageManaged {
        message: String,
    },
    NoUpdate {
        user_message: Option<String>,
    },
    Updated {
        version: String,
        user_message: Option<String>,
    },
}

pub fn run_manual_update(updater: &dyn Updater, package_managed: bool) -> Result<ManualUpdateOutcome> {
    if package_managed {
        return Ok(ManualUpdateOutcome::PackageManaged {
            message: "cloudflared was installed by a package manager. Please update using the same method."
                .to_owned(),
        });
    }

    let check = updater.check()?;
    let Some(update) = check.update else {
        return Ok(ManualUpdateOutcome::NoUpdate {
            user_message: check.user_message,
        });
    };

    updater.apply(&update)?;

    Ok(ManualUpdateOutcome::Updated {
        version: update.version,
        user_message: update.user_message,
    })
}

// --- HIS-047: auto-updater ---

/// Default auto-update frequency.
pub const DEFAULT_AUTOUPDATE_FREQ: Duration = Duration::from_secs(24 * 60 * 60);

/// Trait for the auto-update timer loop.
pub trait AutoUpdater: Send + Sync {
    /// Start the auto-update loop. Blocks until shutdown.
    fn run(&self) -> Result<()>;

    /// Signal shutdown.
    fn shutdown(&self);
}

// --- HIS-048: update exit codes ---

/// Exit code emitted by `cloudflared update` on successful binary replacement.
pub const UPDATE_EXIT_SUCCESS: i32 = 11;

/// Exit code emitted by `cloudflared update` on failure.
pub const UPDATE_EXIT_FAILURE: i32 = 10;

// --- HIS-049: package manager awareness ---

/// Check if update should be skipped because binary was installed via
/// package manager.
pub fn should_skip_update() -> bool {
    crate::environment::is_package_managed()
}

// --- Stub implementations ---

/// Stub updater used by tests that assert deferred behavior.
pub struct StubUpdater;

impl Updater for StubUpdater {
    fn check(&self) -> Result<UpdateCheck> {
        Err(ConfigError::deferred("update check"))
    }

    fn apply(&self, _info: &UpdateInfo) -> Result<()> {
        Err(ConfigError::deferred("update apply"))
    }
}

pub struct StubAutoUpdater;

impl AutoUpdater for StubAutoUpdater {
    fn run(&self) -> Result<()> {
        Err(ConfigError::deferred("auto-updater"))
    }

    fn shutdown(&self) {}
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateServiceResponse {
    url: String,
    version: String,
    checksum: String,
    compressed: bool,
    user_message: String,
    should_update: bool,
    error: String,
}

fn normalize_optional_message(message: String) -> Option<String> {
    let message = message.trim().to_owned();
    if message.is_empty() { None } else { Some(message) }
}

fn update_arch() -> &'static str {
    match crate::environment::TARGET_ARCH {
        "x86_64" => "amd64",
        "x86" => "386",
        "aarch64" => "arm64",
        other => other,
    }
}

fn download_update(client: &Client, url: &str, path: &Path, compressed: bool) -> Result<()> {
    let response = client
        .get(url)
        .send()
        .map_err(|error| ConfigError::invariant(format!("failed to download update payload: {error}")))?;

    if !response.status().is_success() {
        return Err(ConfigError::invariant(format!(
            "failed to download update payload: {}",
            response.status().as_u16()
        )));
    }

    let mut output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| ConfigError::create_file(path, error))?;

    if compressed || is_compressed_file(url) {
        write_compressed_payload(response, &mut output)?;
    } else {
        write_response_payload(response, &mut output)?;
    }

    output
        .flush()
        .map_err(|error| ConfigError::write_file(path, error))?;
    set_executable_permissions(&output, path)?;
    Ok(())
}

fn write_response_payload(mut response: Response, output: &mut File) -> Result<()> {
    std::io::copy(&mut response, output)
        .map(|_| ())
        .map_err(|error| ConfigError::invariant(format!("failed to write update payload: {error}")))
}

fn write_compressed_payload(response: Response, output: &mut File) -> Result<()> {
    let decoder = GzDecoder::new(response);
    let mut archive = Archive::new(decoder);
    let mut entries = archive.entries().map_err(|error| {
        ConfigError::invariant(format!("failed to read compressed update payload: {error}"))
    })?;
    let Some(entry) = entries.next() else {
        return Err(ConfigError::invariant("compressed update payload was empty"));
    };
    let mut entry = entry.map_err(|error| {
        ConfigError::invariant(format!("failed to unpack compressed update payload: {error}"))
    })?;
    std::io::copy(&mut entry, output)
        .map(|_| ())
        .map_err(|error| ConfigError::invariant(format!("failed to write update payload: {error}")))
}

fn is_compressed_file(url: &str) -> bool {
    url.ends_with(".tgz")
        || Url::parse(url)
            .map(|parsed| parsed.path().ends_with(".tgz"))
            .unwrap_or(false)
}

fn file_checksum_hex(path: &Path) -> Result<String> {
    let mut file = File::open(path).map_err(|error| ConfigError::read(path, error))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| ConfigError::read(path, error))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }

    Ok(format!("{:x}", digest.finalize()))
}

fn suffixed_path(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn set_executable_permissions(file: &File, path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = file
            .metadata()
            .map_err(|error| ConfigError::read(path, error))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).map_err(|error| ConfigError::write_file(path, error))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
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
            "{{\"url\":\"{payload_url}\",\"version\":\"2026.2.1\",\"checksum\":\"{checksum}\",\"compressed\"\
             :false,\"userMessage\":\"\",\"shouldUpdate\":true,\"error\":\"\"}}"
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
}
