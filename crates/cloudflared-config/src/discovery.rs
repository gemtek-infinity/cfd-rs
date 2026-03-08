use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, Result};

const DEFAULT_CONFIG_FILES: [&str; 2] = ["config.yml", "config.yaml"];
const DEFAULT_NIX_SEARCH_DIRECTORIES: [&str; 5] = [
    "~/.cloudflared",
    "~/.cloudflare-warp",
    "~/cloudflare-warp",
    "/etc/cloudflared",
    "/usr/local/etc/cloudflared",
];
const DEFAULT_NIX_PRIMARY_CONFIG_DIR: &str = "/usr/local/etc/cloudflared";
const DEFAULT_NIX_PRIMARY_LOG_DIR: &str = "/var/log/cloudflared";

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiscoveryOrigin {
    Explicit,
    Search,
    AutoCreateDefault,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiscoveryAction {
    UseExisting,
    CreateDefaultConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConfigSource {
    ExplicitPath(PathBuf),
    DiscoveredPath(PathBuf),
    AutoCreatedPath(PathBuf),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryCandidate {
    pub origin: DiscoveryOrigin,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryOutcome {
    pub action: DiscoveryAction,
    pub source: ConfigSource,
    pub path: PathBuf,
    pub created_paths: Vec<PathBuf>,
    pub written_config: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryPlan {
    pub path: PathBuf,
    pub log_directory: PathBuf,
    pub create_config_directory: bool,
    pub create_config_file: bool,
    pub create_log_directory: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryDefaults {
    pub config_filenames: Vec<String>,
    pub search_directories: Vec<PathBuf>,
    pub primary_config_path: PathBuf,
    pub primary_log_directory: PathBuf,
}

impl Default for DiscoveryDefaults {
    fn default() -> Self {
        Self {
            config_filenames: DEFAULT_CONFIG_FILES.into_iter().map(str::to_owned).collect(),
            search_directories: default_nix_search_directories(),
            primary_config_path: default_nix_primary_config_path(),
            primary_log_directory: default_nix_log_directory(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct DiscoveryRequest {
    pub explicit_config: Option<PathBuf>,
    pub defaults: DiscoveryDefaults,
}

impl DiscoveryRequest {
    pub fn candidate_paths(&self) -> Vec<DiscoveryCandidate> {
        if let Some(explicit_config) = &self.explicit_config {
            return vec![DiscoveryCandidate {
                origin: DiscoveryOrigin::Explicit,
                path: explicit_config.clone(),
            }];
        }

        self.defaults
            .search_directories
            .iter()
            .flat_map(|directory| {
                self.defaults
                    .config_filenames
                    .iter()
                    .map(|filename| DiscoveryCandidate {
                        origin: DiscoveryOrigin::Search,
                        path: directory.join(filename),
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    pub fn auto_create_plan(&self) -> DiscoveryPlan {
        DiscoveryPlan {
            path: self.defaults.primary_config_path.clone(),
            log_directory: self.defaults.primary_log_directory.clone(),
            create_config_directory: true,
            create_config_file: true,
            create_log_directory: true,
        }
    }

    pub fn find_default_config_path(&self) -> Option<PathBuf> {
        self.candidate_paths().into_iter().find_map(|candidate| {
            if candidate.path.exists() {
                Some(candidate.path)
            } else {
                None
            }
        })
    }

    pub fn find_or_create_config_path(&self) -> Result<DiscoveryOutcome> {
        if let Some(explicit_config) = &self.explicit_config {
            return Ok(DiscoveryOutcome {
                action: DiscoveryAction::UseExisting,
                source: ConfigSource::ExplicitPath(explicit_config.clone()),
                path: explicit_config.clone(),
                created_paths: Vec::new(),
                written_config: None,
            });
        }

        if let Some(path) = self.find_default_config_path() {
            return Ok(DiscoveryOutcome {
                action: DiscoveryAction::UseExisting,
                source: ConfigSource::DiscoveredPath(path.clone()),
                path,
                created_paths: Vec::new(),
                written_config: None,
            });
        }

        let plan = self.auto_create_plan();
        let config_directory = plan.path.parent().unwrap_or(plan.path.as_path()).to_path_buf();
        fs::create_dir_all(&config_directory)
            .map_err(|source| ConfigError::create_directory(config_directory.clone(), source))?;
        fs::create_dir_all(&plan.log_directory)
            .map_err(|source| ConfigError::create_directory(plan.log_directory.clone(), source))?;

        let contents = minimal_auto_create_config(&plan.log_directory);
        fs::write(&plan.path, &contents)
            .map_err(|source| ConfigError::write_file(plan.path.clone(), source))?;

        Ok(DiscoveryOutcome {
            action: DiscoveryAction::CreateDefaultConfig,
            source: ConfigSource::AutoCreatedPath(plan.path.clone()),
            path: plan.path.clone(),
            created_paths: vec![config_directory, plan.path, plan.log_directory],
            written_config: Some(contents),
        })
    }
}

pub fn minimal_auto_create_config(log_directory: &std::path::Path) -> String {
    format!("logDirectory: {}\n", log_directory.display())
}

pub fn default_nix_search_directories() -> Vec<PathBuf> {
    DEFAULT_NIX_SEARCH_DIRECTORIES
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

pub fn default_nix_primary_config_path() -> PathBuf {
    PathBuf::from(DEFAULT_NIX_PRIMARY_CONFIG_DIR).join(DEFAULT_CONFIG_FILES[0])
}

pub fn default_nix_log_directory() -> PathBuf {
    PathBuf::from(DEFAULT_NIX_PRIMARY_LOG_DIR)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{DiscoveryAction, DiscoveryDefaults, DiscoveryOrigin, DiscoveryRequest};

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
    fn candidate_paths_follow_known_search_order() {
        let request = DiscoveryRequest::default();
        let candidates = request.candidate_paths();

        assert_eq!(candidates[0].origin, DiscoveryOrigin::Search);
        assert_eq!(candidates[0].path.to_string_lossy(), "~/.cloudflared/config.yml");
        assert_eq!(candidates[1].path.to_string_lossy(), "~/.cloudflared/config.yaml");
        assert_eq!(
            candidates[2].path.to_string_lossy(),
            "~/.cloudflare-warp/config.yml"
        );
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

        let outcome = request
            .find_or_create_config_path()
            .expect("auto-create should succeed");

        assert_eq!(outcome.action, DiscoveryAction::CreateDefaultConfig);
        assert!(outcome.path.exists());
        assert_eq!(
            fs::read_to_string(&outcome.path).expect("config should be written"),
            format!("logDirectory: {}\n", root.join("var/log/cloudflared").display())
        );

        fs::remove_dir_all(root).expect("temp directory should be removable");
    }
}
