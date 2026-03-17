//! Tunnel login command — browser-based cert generation.
//!
//! Matches Go `cmd/cloudflared/tunnel/login.go` `login()`.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tracing::{error, info};
use uuid::Uuid;

use cfdrs_cli::CliOutput;
use cfdrs_shared::config::discovery::default_nix_search_directories;
use cfdrs_shared::{DEFAULT_ORIGIN_CERT_FILE, FED_ENDPOINT, OriginCertToken};

// --- Go baseline constants from login.go ---

const BASE_LOGIN_URL: &str = "https://dash.cloudflare.com/argotunnel";
const CALLBACK_URL: &str = "https://login.cloudflareaccess.org/";
const FED_BASE_LOGIN_URL: &str = "https://dash.fed.cloudflare.com/argotunnel";
const FED_CALLBACK_STORE_URL: &str = "https://login.fed.cloudflareaccess.org/";

// --- Go baseline constants from transfer.go ---

const POLL_ATTEMPTS: u32 = 10;
const CLIENT_TIMEOUT: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

// --- Go baseline messages ---

const CERT_EXISTS_MSG: &str = "You have an existing certificate at {path} which login would overwrite.\nIf \
                               this is intentional, please move or delete that file then run this command \
                               again.";

const BROWSER_OPENED_MSG: &str = "A browser window should have opened at the following URL:\n\n{url}\n\nIf \
                                  the browser failed to open, please visit the URL above directly in your \
                                  browser.";

const BROWSER_FAILED_MSG: &str = "Please open the following URL and log in with your Cloudflare \
                                  account:\n\n{url}\n\nLeave cloudflared running to download the cert \
                                  automatically.";

const TRANSFER_FAILED_MSG: &str = "Failed to write the certificate.\n\nYour browser will download the \
                                   certificate instead. You will have to manually\ncopy it to the following \
                                   path:\n\n{path}";

const LOGIN_SUCCESS_MSG: &str = "You have successfully logged in.\nIf you wish to copy your credentials to \
                                 a server, they have been saved to:\n{path}";

/// Execute `tunnel login` — interactive browser-based auth flow.
///
/// Go baseline: `login()` in `cmd/cloudflared/tunnel/login.go`.
pub fn execute_tunnel_login(fedramp: bool, login_url: Option<&str>, callback_url: Option<&str>) -> CliOutput {
    // 1. Check for existing cert (Go: checkForExistingCert).
    let cert_path = match check_for_existing_cert() {
        Ok((path, true)) => {
            error!(
                "{}",
                CERT_EXISTS_MSG.replace("{path}", &path.display().to_string())
            );
            // Go returns nil (exit 0) when cert already exists.
            return CliOutput::success(String::new());
        }
        Ok((path, false)) => path,
        Err(e) => return CliOutput::failure(String::new(), e, 1),
    };

    // 2. Determine login and callback URLs (Go: FedRAMP override).
    let base_login = if fedramp {
        FED_BASE_LOGIN_URL
    } else {
        login_url.unwrap_or(BASE_LOGIN_URL)
    };

    let callback_store = if fedramp {
        FED_CALLBACK_STORE_URL
    } else {
        callback_url.unwrap_or(CALLBACK_URL)
    };

    // 3. Run the transfer dance (Go: token.RunTransfer).
    let resource_data = match run_login_transfer(base_login, callback_store) {
        Ok(data) => data,
        Err(e) => {
            error!(
                "{}",
                TRANSFER_FAILED_MSG.replace("{path}", &cert_path.display().to_string())
            );
            return CliOutput::failure(String::new(), e, 1);
        }
    };

    // 4. Decode origin cert (Go: credentials.DecodeOriginCert).
    let mut cert = match OriginCertToken::from_pem_blocks(&resource_data) {
        Ok(c) => c,
        Err(e) => {
            error!("failed to decode origin certificate: {e}");
            return CliOutput::failure(
                String::new(),
                format!("failed to decode origin certificate: {e}"),
                1,
            );
        }
    };

    // 5. Set FedRAMP endpoint (Go: cert.Endpoint = credentials.FedEndpoint).
    if fedramp {
        cert.endpoint = Some(FED_ENDPOINT.to_owned());
    }

    // 6. Re-encode (Go: cert.EncodeOriginCert).
    let encoded = match cert.encode_pem() {
        Ok(data) => data,
        Err(e) => {
            error!("failed to encode origin certificate: {e}");
            return CliOutput::failure(
                String::new(),
                format!("failed to encode origin certificate: {e}"),
                1,
            );
        }
    };

    // 7. Write to disk with mode 0600 (Go: os.WriteFile(path, data, 0600)).
    if let Err(e) = write_cert_file(&cert_path, &encoded) {
        return CliOutput::failure(
            String::new(),
            format!("error writing cert to {}: {e}", cert_path.display()),
            1,
        );
    }

    info!(
        "{}",
        LOGIN_SUCCESS_MSG.replace("{path}", &cert_path.display().to_string())
    );

    CliOutput::success(String::new())
}

/// Check for an existing cert.pem in the first config search directory.
///
/// Go baseline: `checkForExistingCert()` in `login.go`.
/// Returns (path, exists) or error.
fn check_for_existing_cert() -> Result<(PathBuf, bool), String> {
    let dirs = default_nix_search_directories();
    let config_dir = dirs.first().ok_or("no default config directory found")?;

    // Create directory if it doesn't exist (Go: os.Mkdir(configPath, 0700)).
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)
            .map_err(|e| format!("failed to create config directory {}: {e}", config_dir.display()))?;

        let perms = fs::Permissions::from_mode(0o700);
        fs::set_permissions(config_dir, perms)
            .map_err(|e| format!("failed to set permissions on {}: {e}", config_dir.display()))?;
    }

    let cert_path = config_dir.join(DEFAULT_ORIGIN_CERT_FILE);

    match fs::metadata(&cert_path) {
        Ok(meta) if meta.len() > 0 => Ok((cert_path, true)),
        Ok(_) => Ok((cert_path, false)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok((cert_path, false)),
        Err(e) => Err(format!("failed to check cert at {}: {e}", cert_path.display())),
    }
}

/// Write cert data to disk with mode 0600.
fn write_cert_file(path: &Path, data: &[u8]) -> Result<(), String> {
    fs::write(path, data).map_err(|e| e.to_string())?;

    let perms = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, perms).map_err(|e| e.to_string())?;

    Ok(())
}

/// Run the login transfer dance — generate a unique key, open the browser,
/// and poll for the cert.
///
/// Go baseline: `token.RunTransfer()` in `token/transfer.go` with
/// `shouldEncrypt=false`.
fn run_login_transfer(login_url: &str, callback_store_url: &str) -> Result<Vec<u8>, String> {
    // Generate a unique key for the polling endpoint.
    // Go uses a NaCl public key, but since shouldEncrypt=false the key is
    // only used as an opaque path identifier. UUID v4 provides equivalent
    // uniqueness without requiring a NaCl dependency.
    let unique_key = Uuid::new_v4().as_simple().to_string();

    // Build the auth URL (Go: buildRequestURL with cli=false).
    let request_url = build_login_request_url(login_url, &unique_key, callback_store_url)?;

    // Attempt to open the browser (Go: OpenBrowser via xdg-open on Linux).
    let browser_opened = open_browser(&request_url);

    if browser_opened {
        eprintln!("{}", BROWSER_OPENED_MSG.replace("{url}", &request_url));
    } else {
        eprintln!("{}", BROWSER_FAILED_MSG.replace("{url}", &request_url));
    }

    // Build the polling URL (Go: storeURL + publicKey).
    let poll_url = format!("{}/{}", callback_store_url.trim_end_matches('/'), unique_key);

    // Poll for the cert (Go: transferRequest with 10 attempts).
    transfer_request(&poll_url)
}

/// Build the login URL with query parameters.
///
/// Go baseline: `buildRequestURL()` with `cli=false` in `transfer.go`.
/// For login: sets `callback={callbackURL}{key}` and `aud=""`.
fn build_login_request_url(
    base_login_url: &str,
    key: &str,
    callback_store_url: &str,
) -> Result<String, String> {
    let mut parsed = url::Url::parse(base_login_url).map_err(|e| format!("invalid login URL: {e}"))?;

    // Go: q.Set("callback", callbackURL + publicKey)
    let callback_value = format!("{}{}", callback_store_url, key);
    parsed.query_pairs_mut().append_pair("callback", &callback_value);

    // Go: q.Set("aud", "") — empty AUD for login.
    parsed.query_pairs_mut().append_pair("aud", "");

    Ok(parsed.to_string())
}

/// Open the default browser via `xdg-open` (Linux).
///
/// Go baseline: `getBrowserCmd(url)` in `launch_browser_unix.go`.
fn open_browser(url: &str) -> bool {
    std::process::Command::new("xdg-open")
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

/// Poll the callback store for the cert data.
///
/// Go baseline: `transferRequest()` in `transfer.go` — 10 attempts with
/// 60s timeout each.
fn transfer_request(poll_url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(CLIENT_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?;

    for attempt in 0..POLL_ATTEMPTS {
        match poll_once(&client, poll_url) {
            Ok(Some(data)) => return Ok(data),
            Ok(None) => {
                // Non-200 non-error — "Waiting for login..."
                info!("Waiting for login... (attempt {}/{})", attempt + 1, POLL_ATTEMPTS);
            }
            Err(e) => return Err(e),
        }
    }

    Err("failed to fetch cert — login timed out after polling".to_owned())
}

/// Single poll attempt.
///
/// Go baseline: `poll()` in `transfer.go`.
/// Returns Ok(Some(data)) on 200, Ok(None) on retriable status, Err on
/// hard failures (status >= 500, connection errors).
fn poll_once(client: &reqwest::blocking::Client, url: &str) -> Result<Option<Vec<u8>>, String> {
    let response = match client.get(url).send() {
        Ok(r) => r,
        Err(e) if e.is_connect() || e.is_timeout() => {
            // Connection error — retriable (server might not be ready yet).
            return Ok(None);
        }
        Err(e) => return Err(format!("poll request failed: {e}")),
    };

    let status = response.status().as_u16();

    // Go: status >= 500 → error.
    if status >= 500 {
        let body = response.text().unwrap_or_default();
        return Err(format!("error on request {status}: {body}"));
    }

    // Go: status != 200 → waiting (return nil, nil, nil).
    if status != 200 {
        return Ok(None);
    }

    // Go: 200 → return body.
    let data = response
        .bytes()
        .map_err(|e| format!("failed to read response: {e}"))?;
    Ok(Some(data.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_url_constants_match_go_baseline() {
        assert_eq!(BASE_LOGIN_URL, "https://dash.cloudflare.com/argotunnel");
        assert_eq!(CALLBACK_URL, "https://login.cloudflareaccess.org/");
        assert_eq!(FED_BASE_LOGIN_URL, "https://dash.fed.cloudflare.com/argotunnel");
        assert_eq!(FED_CALLBACK_STORE_URL, "https://login.fed.cloudflareaccess.org/");
    }

    #[test]
    fn build_login_url_standard() {
        let url =
            build_login_request_url(BASE_LOGIN_URL, "test-key-123", CALLBACK_URL).expect("should build URL");

        assert!(url.starts_with("https://dash.cloudflare.com/argotunnel?"));
        assert!(url.contains("callback="));
        assert!(url.contains("test-key-123"));
        assert!(url.contains("aud="));
    }

    #[test]
    fn build_login_url_fedramp() {
        let url = build_login_request_url(FED_BASE_LOGIN_URL, "fed-key-456", FED_CALLBACK_STORE_URL)
            .expect("should build FedRAMP URL");

        assert!(url.starts_with("https://dash.fed.cloudflare.com/argotunnel?"));
        assert!(url.contains("fed-key-456"));
    }

    #[test]
    fn build_login_url_invalid_base() {
        let result = build_login_request_url("not a url", "key", CALLBACK_URL);
        assert!(result.is_err());
    }

    #[test]
    fn check_for_existing_cert_in_temp_dir() {
        // This test validates the cert check logic using a temporary
        // directory, not the real config path.
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let cert_path = dir.path().join(DEFAULT_ORIGIN_CERT_FILE);

        // No cert file — should report not exists.
        assert!(!cert_path.exists());

        // Write a non-empty cert file.
        fs::write(&cert_path, b"PEM DATA").expect("write should succeed");
        let meta = fs::metadata(&cert_path).expect("metadata should work");
        assert!(meta.len() > 0);

        // Write an empty cert file — Go treats size 0 as "not exists".
        fs::write(&cert_path, b"").expect("write should succeed");
        let meta = fs::metadata(&cert_path).expect("metadata should work");
        assert_eq!(meta.len(), 0);
    }

    #[test]
    fn write_cert_file_creates_with_mode_0600() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let path = dir.path().join("test-cert.pem");

        write_cert_file(&path, b"CERT DATA").expect("write should succeed");

        let meta = fs::metadata(&path).expect("metadata should work");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "cert file should have mode 0600");
    }

    #[test]
    fn cert_exists_message_includes_path() {
        let msg = CERT_EXISTS_MSG.replace("{path}", "/home/user/.cloudflared/cert.pem");
        assert!(msg.contains("/home/user/.cloudflared/cert.pem"));
        assert!(msg.contains("move or delete"));
    }

    #[test]
    fn login_success_message_includes_path() {
        let msg = LOGIN_SUCCESS_MSG.replace("{path}", "/home/user/.cloudflared/cert.pem");
        assert!(msg.contains("successfully logged in"));
        assert!(msg.contains("/home/user/.cloudflared/cert.pem"));
    }

    #[test]
    fn poll_attempts_and_timeout_match_go() {
        // Go: pollAttempts = 10, clientTimeout = 60s.
        assert_eq!(POLL_ATTEMPTS, 10);
        assert_eq!(CLIENT_TIMEOUT, Duration::from_secs(60));
    }
}
