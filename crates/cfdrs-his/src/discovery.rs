use std::fs;
use std::path::PathBuf;

use cfdrs_shared::config::discovery::{DiscoveryAction, DiscoveryOutcome, DiscoveryPlan, DiscoveryRequest};
use cfdrs_shared::{ConfigError, ConfigSource, Result};

/// Search platform-specific candidate paths and return the first that exists.
pub fn find_default_config_path(request: &DiscoveryRequest) -> Option<PathBuf> {
    request.candidate_paths().into_iter().find_map(|candidate| {
        if candidate.path.exists() {
            Some(candidate.path)
        } else {
            None
        }
    })
}

/// Resolve a config file: use explicit path, discover an existing one, or
/// auto-create a default config at the platform primary path.
pub fn find_or_create_config_path(request: &DiscoveryRequest) -> Result<DiscoveryOutcome> {
    if let Some(explicit_config) = &request.explicit_config {
        return Ok(DiscoveryOutcome {
            action: DiscoveryAction::UseExisting,
            source: ConfigSource::ExplicitPath(explicit_config.clone()),
            path: explicit_config.clone(),
            created_paths: Vec::new(),
            written_config: None,
        });
    }

    if let Some(path) = find_default_config_path(request) {
        return Ok(DiscoveryOutcome {
            action: DiscoveryAction::UseExisting,
            source: ConfigSource::DiscoveredPath(path.clone()),
            path,
            created_paths: Vec::new(),
            written_config: None,
        });
    }

    let plan = request.auto_create_plan();
    execute_auto_create_plan(&plan)
}

fn execute_auto_create_plan(plan: &DiscoveryPlan) -> Result<DiscoveryOutcome> {
    let config_directory = plan.path.parent().unwrap_or(plan.path.as_path()).to_path_buf();

    fs::create_dir_all(&config_directory)
        .map_err(|source| ConfigError::create_directory(config_directory.clone(), source))?;
    fs::create_dir_all(&plan.log_directory)
        .map_err(|source| ConfigError::create_directory(plan.log_directory.clone(), source))?;

    let contents = minimal_auto_create_config(&plan.log_directory);
    fs::write(&plan.path, &contents).map_err(|source| ConfigError::write_file(plan.path.clone(), source))?;

    Ok(DiscoveryOutcome {
        action: DiscoveryAction::CreateDefaultConfig,
        source: ConfigSource::AutoCreatedPath(plan.path.clone()),
        path: plan.path.clone(),
        created_paths: vec![config_directory, plan.path.clone(), plan.log_directory.clone()],
        written_config: Some(contents),
    })
}

pub fn minimal_auto_create_config(log_directory: &std::path::Path) -> String {
    format!("logDirectory: {}\n", log_directory.display())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use cfdrs_shared::config::discovery::{DiscoveryAction, DiscoveryDefaults, DiscoveryRequest};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cloudflared-config-{name}-{unique}"));
        fs::create_dir_all(&path).expect("temp directory should be created");
        path
    }

    #[test]
    fn find_or_create_writes_minimal_config() {
        let root = temp_dir("discovery");
        let request = DiscoveryRequest {
            explicit_config: None,
            defaults: DiscoveryDefaults {
                config_filenames: vec!["config.yml".to_owned()],
                search_directories: vec![root.join("home/.cloudflared")],
                primary_config_path: root.join("usr/local/etc/cloudflared/config.yml"),
                primary_log_directory: root.join("var/log/cloudflared"),
            },
        };

        let outcome = super::find_or_create_config_path(&request).expect("auto-create should succeed");

        assert_eq!(outcome.action, DiscoveryAction::CreateDefaultConfig);
        assert!(outcome.path.exists());
        assert_eq!(
            fs::read_to_string(&outcome.path).expect("config should be written"),
            format!("logDirectory: {}\n", root.join("var/log/cloudflared").display())
        );

        fs::remove_dir_all(root).expect("temp directory should be removable");
    }
}
