#![forbid(unsafe_code)]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::credentials::{CredentialSurface, TunnelReference};
use crate::discovery::ConfigSource;
use crate::error::Result;
use crate::ingress::IngressRule;
use crate::raw_config::{RawConfig, WarpRoutingConfig};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum NormalizationWarning {
    UnknownTopLevelKeys(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedConfig {
    pub source: ConfigSource,
    pub tunnel: Option<TunnelReference>,
    pub credentials: CredentialSurface,
    pub ingress: Vec<IngressRule>,
    pub origin_request: crate::ingress::OriginRequestConfig,
    pub warp_routing: WarpRoutingConfig,
    pub log_directory: Option<PathBuf>,
    pub warnings: Vec<NormalizationWarning>,
}

impl NormalizedConfig {
    pub fn from_raw(source: ConfigSource, raw: RawConfig) -> Result<Self> {
        let unknown_top_level_keys = raw.unknown_top_level_keys();
        let warnings = if unknown_top_level_keys.is_empty() {
            Vec::new()
        } else {
            vec![NormalizationWarning::UnknownTopLevelKeys(unknown_top_level_keys)]
        };
        let tunnel = raw.tunnel.map(TunnelReference::from_raw);

        let ingress = raw
            .ingress
            .into_iter()
            .map(IngressRule::from_raw)
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            source,
            tunnel: tunnel.clone(),
            credentials: CredentialSurface::configured(raw.credentials_file, raw.origin_cert, tunnel),
            ingress,
            origin_request: raw.origin_request,
            warp_routing: raw.warp_routing,
            log_directory: raw.log_directory,
            warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::credentials::TunnelReference;
    use crate::discovery::ConfigSource;
    use crate::normalized::{NormalizationWarning, NormalizedConfig};
    use crate::raw_config::RawConfig;

    fn ok<T, E: std::fmt::Display>(result: std::result::Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("unexpected error: {error}"),
        }
    }

    #[test]
    fn normalization_carries_unknown_top_level_key_warning() {
        let raw = ok(RawConfig::from_yaml_str(
            "fixture.yaml",
            "tunnel: config-file-test\ningress:\n  - service: https://localhost:8080\nextraKey: true\n",
        ));
        let normalized = ok(NormalizedConfig::from_raw(
            ConfigSource::DiscoveredPath("/tmp/config.yml".into()),
            raw,
        ));

        assert_eq!(
            normalized.warnings,
            vec![NormalizationWarning::UnknownTopLevelKeys(vec![
                "extraKey".to_owned(),
            ])]
        );
        assert_eq!(
            normalized.tunnel,
            Some(TunnelReference {
                raw: "config-file-test".to_owned(),
                uuid: None,
            })
        );
    }
}
