#![forbid(unsafe_code)]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
    use super::{DiscoveryOrigin, DiscoveryRequest};

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
}
