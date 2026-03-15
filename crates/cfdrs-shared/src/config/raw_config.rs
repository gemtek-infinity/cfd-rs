use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::error::{ConfigError, Result};
use crate::config::ingress::{OriginRequestConfig, RawIngressRule};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct WarpRoutingConfig {
    #[serde(rename = "connectTimeout", default)]
    pub connect_timeout: Option<crate::config::ingress::DurationSpec>,
    #[serde(rename = "maxActiveFlows", default)]
    pub max_active_flows: Option<u64>,
    #[serde(rename = "tcpKeepAlive", default)]
    pub tcp_keep_alive: Option<crate::config::ingress::DurationSpec>,
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

    /// HIS-003: Go `ReadConfigFile` double-parse accepts unknown keys on
    /// the first (lenient) pass and only surfaces them as warnings on the
    /// second (strict) pass.  Rust captures them via `serde(flatten)` in a
    /// single parse.  This test confirms that unknown keys never cause a
    /// parse failure — the core strict-mode parity invariant.
    #[test]
    fn unknown_keys_accepted_not_rejected_strict_mode_parity() {
        let yaml =
            "tunnel: abc\ningress:\n  - service: https://localhost:8080\nbadKey1: hello\nbadKey2: 42\n";
        let raw = ok(RawConfig::from_yaml_str("strict.yaml", yaml));

        let mut unknown = raw.unknown_top_level_keys();
        unknown.sort();
        assert_eq!(unknown, vec!["badKey1".to_owned(), "badKey2".to_owned()]);

        // Known fields still parsed correctly alongside unknown keys.
        assert_eq!(raw.tunnel.as_deref(), Some("abc"));
        assert_eq!(raw.ingress.len(), 1);
    }

    /// HIS-003: Go baseline handles empty config files gracefully — the
    /// first decode returns `io.EOF` which is logged but not fatal.  The
    /// Rust path must also accept an empty YAML string without error.
    #[test]
    fn empty_config_parses_without_error() {
        let raw = ok(RawConfig::from_yaml_str("empty.yaml", ""));

        assert!(raw.tunnel.is_none());
        assert!(raw.ingress.is_empty());
        assert!(raw.unknown_top_level_keys().is_empty());
    }

    /// HIS-003: A config with only known keys produces zero unknown-key
    /// warnings — confirms the unknown-key detection does not false-positive.
    #[test]
    fn known_keys_only_produces_no_unknown_warnings() {
        let yaml = "tunnel: test\ncredentials-file: /tmp/cred.json\ningress:\n  - service: http_status:503\n";
        let raw = ok(RawConfig::from_yaml_str("clean.yaml", yaml));

        assert!(raw.unknown_top_level_keys().is_empty());
        assert_eq!(raw.tunnel.as_deref(), Some("test"));
        assert_eq!(
            raw.credentials_file
                .as_ref()
                .map(|p| p.to_str().expect("path should be UTF-8")),
            Some("/tmp/cred.json")
        );
    }
}
