use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::credentials::{CredentialSurface, TunnelReference};
use crate::discovery::ConfigSource;
use crate::error::Result;
use crate::ingress::{IngressRule, OriginRequestConfig, default_no_ingress_rule};
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
        let inherited_origin_request = OriginRequestConfig::materialized_config_defaults(&raw.origin_request);

        let ingress = if raw.ingress.is_empty() {
            vec![default_no_ingress_rule()]
        } else {
            let total_rules = raw.ingress.len();
            raw.ingress
                .into_iter()
                .enumerate()
                .map(|(rule_index, rule)| {
                    IngressRule::from_raw(rule, &inherited_origin_request, rule_index, total_rules)
                })
                .collect::<Result<Vec<_>>>()?
        };

        Ok(Self {
            source,
            tunnel: tunnel.clone(),
            credentials: CredentialSurface::configured(raw.credentials_file, raw.origin_cert, tunnel),
            ingress,
            origin_request: inherited_origin_request,
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
    use crate::ingress::IngressService;
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

    #[test]
    fn normalization_creates_default_503_ingress_when_missing() {
        let raw = ok(RawConfig::from_yaml_str(
            "fixture.yaml",
            "tunnel: config-file-test\noriginRequest:\n  connectTimeout: 30s\n",
        ));
        let normalized = ok(NormalizedConfig::from_raw(
            ConfigSource::DiscoveredPath("/tmp/config.yml".into()),
            raw,
        ));

        assert_eq!(normalized.ingress.len(), 1);
        assert_eq!(normalized.ingress[0].service, IngressService::HttpStatus(503));
        assert_eq!(
            normalized
                .origin_request
                .connect_timeout
                .as_ref()
                .map(|value| value.0.as_str()),
            Some("30s")
        );
        assert_eq!(
            normalized
                .origin_request
                .keep_alive_timeout
                .as_ref()
                .map(|value| value.0.as_str()),
            Some("1m30s")
        );
    }

    #[test]
    fn normalization_propagates_materialized_origin_request_to_rules() {
        let raw = ok(RawConfig::from_yaml_str(
            "fixture.yaml",
            "originRequest:\n  ipRules:\n    - prefix: 10.0.0.0/8\n      ports: [80, 8080]\n      allow: false\ningress:\n  - hostname: tunnel1.example.com\n    service: https://localhost:8080\n  - service: https://localhost:8001\n",
        ));
        let normalized = ok(NormalizedConfig::from_raw(
            ConfigSource::DiscoveredPath("/tmp/config.yml".into()),
            raw,
        ));

        assert_eq!(normalized.ingress.len(), 2);
        assert_eq!(
            normalized.ingress[0]
                .origin_request
                .keep_alive_timeout
                .as_ref()
                .map(|value| value.0.as_str()),
            Some("1m30s")
        );
        assert_eq!(normalized.ingress[0].origin_request.proxy_port, Some(0));
        assert_eq!(normalized.ingress[0].origin_request.bastion_mode, Some(false));
        assert_eq!(normalized.ingress[0].origin_request.ip_rules.len(), 1);
        assert_eq!(normalized.ingress[1].origin_request.ip_rules.len(), 1);
    }
}
