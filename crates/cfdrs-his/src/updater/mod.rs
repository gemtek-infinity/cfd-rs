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

const NO_AUTO_UPDATE_IN_SHELL_MESSAGE: &str = "cloudflared will not automatically update when run from the shell. To enable auto-updates, run cloudflared as a service: https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/configure-tunnels/local-management/as-a-service/";
const NO_AUTO_UPDATE_ON_WINDOWS_MESSAGE: &str =
    "cloudflared will not automatically update on Windows systems.";
const NO_AUTO_UPDATE_MANAGED_PACKAGE_MESSAGE: &str =
    "cloudflared will not automatically update if installed by a package manager.";
const NO_AUTO_UPDATE_DISABLED_FLAG_MESSAGE: &str =
    "cloudflared automatic updates are disabled by configuration.";

/// Auto-update settings resolved from CLI flags and runtime restrictions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoUpdateSettings {
    enabled: bool,
    frequency: Duration,
    disabled_reason: Option<&'static str>,
}

impl AutoUpdateSettings {
    pub fn new(enabled: bool, frequency: Duration, disabled_reason: Option<&'static str>) -> Self {
        Self {
            enabled,
            frequency,
            disabled_reason,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn frequency(&self) -> Duration {
        self.frequency
    }

    pub fn disabled_reason(&self) -> Option<&'static str> {
        self.disabled_reason
    }
}

/// Parse a Go-style auto-update duration.
pub fn parse_auto_update_freq(value: Option<&str>) -> Result<Duration> {
    let Some(raw_value) = value else {
        return Ok(DEFAULT_AUTOUPDATE_FREQ);
    };

    let trimmed = raw_value.trim();
    if trimmed.is_empty() {
        return Ok(DEFAULT_AUTOUPDATE_FREQ);
    }

    parse_go_duration(trimmed, "autoupdate-freq")
}

/// Resolve the effective auto-update settings for the current runtime.
pub fn resolve_auto_update_settings(
    update_disabled: bool,
    freq: Option<Duration>,
    package_managed: bool,
    running_from_terminal: bool,
    target_os: &str,
) -> AutoUpdateSettings {
    let requested_frequency = freq.unwrap_or(DEFAULT_AUTOUPDATE_FREQ);
    let disabled_reason = if target_os == "windows" {
        Some(NO_AUTO_UPDATE_ON_WINDOWS_MESSAGE)
    } else if package_managed {
        Some(NO_AUTO_UPDATE_MANAGED_PACKAGE_MESSAGE)
    } else if running_from_terminal {
        Some(NO_AUTO_UPDATE_IN_SHELL_MESSAGE)
    } else if update_disabled || requested_frequency.is_zero() {
        Some(NO_AUTO_UPDATE_DISABLED_FLAG_MESSAGE)
    } else {
        None
    };

    AutoUpdateSettings::new(
        disabled_reason.is_none(),
        if disabled_reason.is_none() {
            requested_frequency
        } else {
            DEFAULT_AUTOUPDATE_FREQ
        },
        disabled_reason,
    )
}

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

fn parse_go_duration(raw: &str, field_name: &str) -> Result<Duration> {
    if raw == "0" {
        return Ok(Duration::ZERO);
    }

    let mut total_nanos = 0f64;
    let mut rest = raw;

    while !rest.is_empty() {
        let number_end = rest
            .find(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
            .ok_or_else(|| ConfigError::invariant(format!("invalid {field_name} duration `{raw}`")))?;

        let number_text = &rest[..number_end];
        if number_text.is_empty() {
            return Err(ConfigError::invariant(format!(
                "invalid {field_name} duration `{raw}`"
            )));
        }

        let value = number_text
            .parse::<f64>()
            .map_err(|_| ConfigError::invariant(format!("invalid {field_name} duration `{raw}`")))?;

        rest = &rest[number_end..];

        let (unit_nanos, next_rest) = parse_go_duration_unit(rest, raw, field_name)?;
        total_nanos += value * unit_nanos;
        rest = next_rest;
    }

    if !total_nanos.is_finite() || total_nanos.is_sign_negative() || total_nanos > u64::MAX as f64 {
        return Err(ConfigError::invariant(format!(
            "invalid {field_name} duration `{raw}`"
        )));
    }

    Ok(Duration::from_nanos(total_nanos.round() as u64))
}

fn parse_go_duration_unit<'a>(raw: &'a str, full_value: &str, field_name: &str) -> Result<(f64, &'a str)> {
    for (unit, nanos) in [
        ("ms", 1_000_000f64),
        ("us", 1_000f64),
        ("ns", 1f64),
        ("h", 3_600_000_000_000f64),
        ("m", 60_000_000_000f64),
        ("s", 1_000_000_000f64),
    ] {
        if let Some(next_rest) = raw.strip_prefix(unit) {
            return Ok((nanos, next_rest));
        }
    }

    Err(ConfigError::invariant(format!(
        "invalid {field_name} duration `{full_value}`"
    )))
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
mod tests;
