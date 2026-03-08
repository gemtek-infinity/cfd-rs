use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, Result};
use crate::ingress::{OriginRequestConfig, RawIngressRule};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct WarpRoutingConfig {
    #[serde(rename = "connectTimeout", default)]
    pub connect_timeout: Option<crate::ingress::DurationSpec>,
    #[serde(rename = "maxActiveFlows", default)]
    pub max_active_flows: Option<u64>,
    #[serde(rename = "tcpKeepAlive", default)]
    pub tcp_keep_alive: Option<crate::ingress::DurationSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawConfig {
    #[serde(default)]
    pub tunnel: Option<String>,
    #[serde(rename = "credentials-file", default)]
    pub credentials_file: Option<PathBuf>,
    #[serde(rename = "origincert", default)]
    pub origin_cert: Option<PathBuf>,
    #[serde(default)]
    pub ingress: Vec<RawIngressRule>,
    #[serde(rename = "warp-routing", default)]
    pub warp_routing: WarpRoutingConfig,
    #[serde(rename = "originRequest", default)]
    pub origin_request: OriginRequestConfig,
    #[serde(rename = "logDirectory", default)]
    pub log_directory: Option<PathBuf>,
    #[serde(flatten)]
    pub additional_fields: BTreeMap<String, serde_yaml::Value>,
}

impl RawConfig {
    pub fn from_yaml_str(source_name: &str, contents: &str) -> Result<Self> {
        serde_yaml::from_str(contents).map_err(|source| ConfigError::yaml(source_name, source))
    }

    pub fn from_yaml_path(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path).map_err(|source| ConfigError::read(path, source))?;
        Self::from_yaml_str(&path.display().to_string(), &contents)
    }

    pub fn unknown_top_level_keys(&self) -> Vec<String> {
        self.additional_fields.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::RawConfig;

    fn ok<T, E: std::fmt::Display>(result: std::result::Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("unexpected error: {error}"),
        }
    }

    #[test]
    fn unknown_top_level_keys_are_retained() {
        let raw = ok(RawConfig::from_yaml_str(
            "fixture.yaml",
            "ingress:\n  - service: https://localhost:8080\nextraKey: true\n",
        ));

        assert_eq!(raw.unknown_top_level_keys(), vec!["extraKey".to_owned()]);
    }
}
