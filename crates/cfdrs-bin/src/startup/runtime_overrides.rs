use std::collections::BTreeMap;
use std::path::Path;

use cfdrs_cli::GlobalFlags;
use cfdrs_his::credentials::search_credential_by_id;
use cfdrs_his::environment::{current_uid, is_container_runtime};
use cfdrs_his::metrics_server::parse_metrics_address;
use cfdrs_his::signal::parse_grace_period;
use cfdrs_shared::{ConfigError, OriginCertLocator};
use cfdrs_shared::{LogConfig, LogLevel, build_log_config};

use crate::runtime::RuntimeConfig;

use super::StartupSurface;

#[derive(Debug)]
pub(crate) struct PreparedRuntimeStartup {
    pub(crate) startup: StartupSurface,
    pub(crate) runtime_config: RuntimeConfig,
    pub(crate) log_config: LogConfig,
    pub(crate) transport_log_level: Option<LogLevel>,
}

pub(crate) fn prepare_runtime_startup(
    mut startup: StartupSurface,
    flags: &GlobalFlags,
) -> Result<PreparedRuntimeStartup, ConfigError> {
    apply_runtime_credential_discovery(&mut startup, flags)?;

    let grace_period = parse_grace_period(flags.grace_period.as_deref())?;
    let log_config = resolve_log_config(&startup, flags)?;
    let transport_log_level = flags.transport_loglevel.as_deref().map(str::parse).transpose()?;
    let diagnostic_configuration = resolve_diagnostic_configuration(&log_config);

    let mut runtime_config = RuntimeConfig::new(startup.discovery.clone(), startup.normalized.clone())
        .with_shutdown_grace_period(grace_period)
        .with_container_runtime(is_container_runtime())
        .with_diagnostic_configuration(diagnostic_configuration);

    if let Some(pidfile_path) = flags.pidfile.clone() {
        runtime_config = runtime_config.with_pidfile_path(pidfile_path);
    }

    if let Some(metrics_bind_address) = resolve_metrics_bind_address(flags)? {
        runtime_config = runtime_config.with_metrics_bind_address(metrics_bind_address);
    }

    Ok(PreparedRuntimeStartup {
        startup,
        runtime_config,
        log_config,
        transport_log_level,
    })
}

/// Go baseline: `findCredentials(tunnelID)` in subcommands.go.
///
/// Resolution order (matching Go):
///   1. Already resolved (e.g. from `--token` path) — skip.
///   2. `--credentials-contents` flag — inline JSON credential material.
///   3. `{TunnelID}.json` file search in origin cert directory.
fn apply_runtime_credential_discovery(
    startup: &mut StartupSurface,
    flags: &GlobalFlags,
) -> Result<(), ConfigError> {
    if startup.normalized.credentials.credentials_file.is_some() {
        return Ok(());
    }

    // Go baseline: sc.c.IsSet("credentials-contents") → json.Unmarshal
    if let Some(contents) = flags.credentials_contents.as_deref() {
        let creds = cfdrs_shared::TunnelCredentialsFile::from_json_str(contents)?;
        let cred_dir = std::env::temp_dir();
        let cred_path = cred_dir.join(format!("{}.json", creds.tunnel_id));
        let json = creds.to_pretty_json()?;
        std::fs::write(&cred_path, json)
            .map_err(|source| ConfigError::write_file(cred_path.clone(), source))?;
        startup.normalized.credentials.credentials_file = Some(cred_path);
        return Ok(());
    }

    let Some(tunnel_id) = startup.normalized.tunnel.as_ref().and_then(|tunnel| tunnel.uuid) else {
        return Ok(());
    };

    let origin_cert_dir = configured_origin_cert_dir(&startup.normalized.credentials.origin_cert);

    if let Ok(credentials_path) = search_credential_by_id(tunnel_id, origin_cert_dir) {
        startup.normalized.credentials.credentials_file = Some(credentials_path);
    }

    Ok(())
}

fn configured_origin_cert_dir(origin_cert: &Option<OriginCertLocator>) -> Option<&Path> {
    match origin_cert.as_ref() {
        Some(OriginCertLocator::ConfiguredPath(path)) | Some(OriginCertLocator::DefaultSearchPath(path)) => {
            if path.exists() {
                return path.parent();
            }

            None
        }
        None => None,
    }
}

fn resolve_metrics_bind_address(flags: &GlobalFlags) -> Result<Option<std::net::SocketAddr>, ConfigError> {
    let Some(metrics_address) = flags.metrics.as_deref() else {
        return Ok(None);
    };

    parse_metrics_address(metrics_address).map(Some).ok_or_else(|| {
        ConfigError::invariant(format!(
            "metrics address {metrics_address:?} must be a socket address such as 127.0.0.1:20241"
        ))
    })
}

fn resolve_log_config(startup: &StartupSurface, flags: &GlobalFlags) -> Result<LogConfig, ConfigError> {
    let logfile = flags
        .logfile
        .as_ref()
        .map(|path| path_str(path.as_path()))
        .transpose()?;
    let log_directory = flags
        .log_directory
        .as_ref()
        .or(startup.normalized.log_directory.as_ref())
        .map(|path| path_str(path.as_path()))
        .transpose()?;

    build_log_config(
        flags.loglevel.as_deref(),
        flags.log_format_output.as_deref(),
        logfile,
        log_directory,
    )
}

fn resolve_diagnostic_configuration(log_config: &LogConfig) -> BTreeMap<String, String> {
    let mut diagnostic_configuration = BTreeMap::from([("uid".to_owned(), current_uid().to_string())]);

    if let Some(file) = log_config.file.as_ref() {
        diagnostic_configuration.insert("log_file".to_owned(), file.full_path().display().to_string());
    }

    if let Some(rolling) = log_config.rolling.as_ref() {
        diagnostic_configuration.insert("log_directory".to_owned(), rolling.dirname.display().to_string());
    }

    diagnostic_configuration
}

fn path_str(path: &Path) -> Result<&str, ConfigError> {
    path.to_str()
        .ok_or_else(|| ConfigError::invariant(format!("path {} is not valid UTF-8", path.display())))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use cfdrs_shared::{
        ConfigSource, CredentialSurface, DiscoveryAction, DiscoveryOutcome, NormalizedConfig,
        OriginRequestConfig, TunnelReference, WarpRoutingConfig,
    };

    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cfdrs-runtime-overrides-{name}-{unique}"));
        fs::create_dir_all(&path).expect("temp directory should be created");
        path
    }

    fn startup_surface(origin_cert: Option<std::path::PathBuf>) -> StartupSurface {
        let config_path = std::path::PathBuf::from("/tmp/config.yml");

        StartupSurface {
            discovery: DiscoveryOutcome {
                action: DiscoveryAction::UseExisting,
                path: config_path.clone(),
                source: ConfigSource::DiscoveredPath(config_path.clone()),
                created_paths: vec![],
                written_config: None,
            },
            normalized: NormalizedConfig {
                source: ConfigSource::DiscoveredPath(config_path),
                tunnel: Some(TunnelReference::from_raw(uuid::Uuid::nil().to_string())),
                credentials: CredentialSurface {
                    credentials_file: None,
                    origin_cert: origin_cert.map(OriginCertLocator::ConfiguredPath),
                    tunnel: Some(TunnelReference::from_raw(uuid::Uuid::nil().to_string())),
                },
                ingress: vec![],
                origin_request: OriginRequestConfig::default(),
                warp_routing: WarpRoutingConfig::default(),
                log_directory: Some(std::path::PathBuf::from("/var/log/cloudflared")),
                warnings: vec![],
            },
        }
    }

    #[test]
    fn prepare_runtime_startup_discovers_credentials_from_origin_cert_dir() {
        let root = temp_dir("credentials");
        let origin_cert = root.join("cert.pem");
        let credentials_path = root.join(format!("{}.json", uuid::Uuid::nil()));

        fs::write(&origin_cert, b"pem").expect("origin cert should be written");
        fs::write(
            &credentials_path,
            r#"{"AccountTag":"acct","TunnelSecret":"AQID","TunnelID":"00000000-0000-0000-0000-000000000000"}"#,
        )
        .expect("credentials should be written");

        let prepared = prepare_runtime_startup(startup_surface(Some(origin_cert)), &GlobalFlags::default())
            .expect("runtime startup should prepare");

        assert_eq!(
            prepared.startup.normalized.credentials.credentials_file,
            Some(credentials_path)
        );

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn prepare_runtime_startup_uses_config_log_directory_when_flag_missing() {
        let root = temp_dir("log-directory");
        let origin_cert = root.join("cert.pem");
        let credentials_path = root.join(format!("{}.json", uuid::Uuid::nil()));

        fs::write(&origin_cert, b"pem").expect("origin cert should be written");
        fs::write(
            &credentials_path,
            r#"{"AccountTag":"acct","TunnelSecret":"AQID","TunnelID":"00000000-0000-0000-0000-000000000000"}"#,
        )
        .expect("credentials should be written");

        let prepared = prepare_runtime_startup(startup_surface(Some(origin_cert)), &GlobalFlags::default())
            .expect("runtime startup should prepare");

        assert_eq!(
            prepared
                .log_config
                .rolling
                .as_ref()
                .map(|config| config.dirname.clone()),
            Some(std::path::PathBuf::from("/var/log/cloudflared"))
        );
        assert_eq!(
            prepared
                .runtime_config
                .diagnostic_configuration()
                .get("log_directory")
                .map(String::as_str),
            Some("/var/log/cloudflared")
        );

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn resolve_metrics_bind_address_rejects_invalid_socket() {
        let flags = GlobalFlags {
            metrics: Some("not-a-socket".to_owned()),
            ..GlobalFlags::default()
        };

        let error = resolve_metrics_bind_address(&flags).expect_err("invalid metrics address should fail");
        assert_eq!(error.category().to_string(), "invariant-violation");
    }

    #[test]
    fn credentials_contents_flag_writes_temp_credential_file() {
        let mut startup = startup_surface(None);
        let flags = GlobalFlags {
            credentials_contents: Some(
                r#"{"AccountTag":"acct","TunnelSecret":"AQID","TunnelID":"00000000-0000-0000-0000-000000000000"}"#
                    .to_owned(),
            ),
            ..GlobalFlags::default()
        };

        apply_runtime_credential_discovery(&mut startup, &flags)
            .expect("credential discovery should succeed");

        let cred_path = startup
            .normalized
            .credentials
            .credentials_file
            .as_ref()
            .expect("credentials_file should be set");

        assert!(cred_path.exists(), "temp credential file should exist");
        let contents = fs::read_to_string(cred_path).expect("credential file should be readable");
        assert!(
            contents.contains("00000000-0000-0000-0000-000000000000"),
            "credential file should contain tunnel ID"
        );
    }

    #[test]
    fn credential_discovery_skips_when_already_resolved() {
        let mut startup = startup_surface(None);
        let existing = std::path::PathBuf::from("/tmp/already-set.json");
        startup.normalized.credentials.credentials_file = Some(existing.clone());

        let flags = GlobalFlags {
            credentials_contents: Some(r#"{"AccountTag":"a","TunnelSecret":"AQ==","TunnelID":"11111111-1111-1111-1111-111111111111"}"#.to_owned()),
            ..GlobalFlags::default()
        };

        apply_runtime_credential_discovery(&mut startup, &flags)
            .expect("credential discovery should succeed");

        assert_eq!(
            startup.normalized.credentials.credentials_file,
            Some(existing),
            "credentials_file should not be overridden when already set"
        );
    }

    // --- HIS-031: container/runtime-class detection ---

    #[test]
    fn container_runtime_setter_enables_virtual_binding() {
        let startup = startup_surface(None);
        let config = RuntimeConfig::new(startup.discovery.clone(), startup.normalized.clone())
            .with_container_runtime(true);

        assert!(
            config.is_container_runtime(),
            "with_container_runtime(true) should enable container mode"
        );
    }

    #[test]
    fn container_runtime_defaults_to_host_mode() {
        let startup = startup_surface(None);
        let config = RuntimeConfig::new(startup.discovery.clone(), startup.normalized.clone());

        assert!(
            !config.is_container_runtime(),
            "default RuntimeConfig should be host mode"
        );
    }

    #[test]
    fn prepare_runtime_startup_wires_container_detection() {
        // In normal test builds (no CONTAINER_BUILD env, no /.dockerenv),
        // the runtime config should report host mode.
        let root = temp_dir("container-detect");
        let origin_cert = root.join("cert.pem");
        let credentials_path = root.join(format!("{}.json", uuid::Uuid::nil()));

        fs::write(&origin_cert, b"pem").expect("origin cert should be written");
        fs::write(
            &credentials_path,
            r#"{"AccountTag":"acct","TunnelSecret":"AQID","TunnelID":"00000000-0000-0000-0000-000000000000"}"#,
        )
        .expect("credentials should be written");

        let prepared = prepare_runtime_startup(startup_surface(Some(origin_cert)), &GlobalFlags::default())
            .expect("runtime startup should prepare");

        assert_eq!(
            prepared.runtime_config.is_container_runtime(),
            is_container_runtime(),
            "runtime config container flag should match host detection"
        );

        fs::remove_dir_all(root).expect("cleanup");
    }
}
