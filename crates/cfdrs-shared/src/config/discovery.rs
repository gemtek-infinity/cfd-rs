use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::config_source::ConfigSource;

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

impl fmt::Display for DiscoveryOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Explicit => f.write_str("explicit"),
            Self::Search => f.write_str("search"),
            Self::AutoCreateDefault => f.write_str("auto-create-default"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiscoveryAction {
    UseExisting,
    CreateDefaultConfig,
}

impl fmt::Display for DiscoveryAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UseExisting => f.write_str("use-existing"),
            Self::CreateDefaultConfig => f.write_str("create-default-config"),
        }
    }
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
}

pub fn default_nix_search_directories() -> Vec<PathBuf> {
    let home_directory = home_directory();
    DEFAULT_NIX_SEARCH_DIRECTORIES
        .into_iter()
        .map(|directory| expand_leading_tilde(directory, home_directory.as_deref()))
        .collect()
}

pub fn default_nix_primary_config_path() -> PathBuf {
    PathBuf::from(DEFAULT_NIX_PRIMARY_CONFIG_DIR).join(DEFAULT_CONFIG_FILES[0])
}

pub fn default_nix_log_directory() -> PathBuf {
    PathBuf::from(DEFAULT_NIX_PRIMARY_LOG_DIR)
}

fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn expand_leading_tilde(path: &str, home_directory: Option<&std::path::Path>) -> PathBuf {
    if path == "~"
        && let Some(home_directory) = home_directory
    {
        return home_directory.to_path_buf();
    }

    if let Some(remainder) = path.strip_prefix("~/")
        && let Some(home_directory) = home_directory
    {
        return home_directory.join(remainder);
    }

    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        DiscoveryOrigin, DiscoveryRequest, default_nix_search_directories, expand_leading_tilde,
        home_directory,
    };

    #[test]
    fn candidate_paths_follow_known_search_order() {
        let request = DiscoveryRequest::default();
        let candidates = request.candidate_paths();
        let home_directory = home_directory();

        assert_eq!(candidates[0].origin, DiscoveryOrigin::Search);
        assert_eq!(
            candidates[0].path,
            expand_leading_tilde("~/.cloudflared", home_directory.as_deref()).join("config.yml")
        );
        assert_eq!(
            candidates[1].path,
            expand_leading_tilde("~/.cloudflared", home_directory.as_deref()).join("config.yaml")
        );
        assert_eq!(
            candidates[2].path,
            expand_leading_tilde("~/.cloudflare-warp", home_directory.as_deref()).join("config.yml")
        );
    }

    #[test]
    fn default_search_directories_expand_home_prefix_only() {
        let directories = default_nix_search_directories();
        if let Some(home_directory) = home_directory() {
            assert_eq!(directories[0], home_directory.join(".cloudflared"));
            assert_eq!(directories[1], home_directory.join(".cloudflare-warp"));
            assert_eq!(directories[2], home_directory.join("cloudflare-warp"));
        } else {
            assert_eq!(directories[0], PathBuf::from("~/.cloudflared"));
            assert_eq!(directories[1], PathBuf::from("~/.cloudflare-warp"));
            assert_eq!(directories[2], PathBuf::from("~/cloudflare-warp"));
        }
        assert_eq!(directories[3], PathBuf::from("/etc/cloudflared"));
        assert_eq!(directories[4], PathBuf::from("/usr/local/etc/cloudflared"));
    }

    #[test]
    fn expand_leading_tilde_does_not_expand_other_patterns() {
        let home_directory = PathBuf::from("/tmp/home");

        assert_eq!(
            expand_leading_tilde("~/.cloudflared", Some(home_directory.as_path())),
            home_directory.join(".cloudflared")
        );
        assert_eq!(
            expand_leading_tilde("~other/.cloudflared", Some(home_directory.as_path())),
            PathBuf::from("~other/.cloudflared")
        );
        assert_eq!(
            expand_leading_tilde("/etc/cloudflared", Some(home_directory.as_path())),
            PathBuf::from("/etc/cloudflared")
        );
    }
}
