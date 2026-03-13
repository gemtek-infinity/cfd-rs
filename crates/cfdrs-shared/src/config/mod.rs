//! Config types, error taxonomy, in-memory parsing, normalization,
//! ingress rule representation, credentials surface, and discovery types.
//!
//! Filesystem discovery IO lives in `cfdrs-his`.

use std::path::Path;

pub mod config_source;
pub mod credentials;
pub mod discovery;
pub mod error;
pub mod ingress;
pub mod normalized;
pub mod raw_config;

pub use self::config_source::ConfigSource;
pub use self::credentials::{
    CredentialSurface, FED_ENDPOINT, OriginCertLocator, OriginCertToken, OriginCertUser,
    TunnelCredentialsFile, TunnelReference, TunnelSecret,
};
pub use self::discovery::{
    DiscoveryAction, DiscoveryCandidate, DiscoveryDefaults, DiscoveryOrigin, DiscoveryOutcome, DiscoveryPlan,
    DiscoveryRequest, default_nix_log_directory, default_nix_primary_config_path,
    default_nix_search_directories,
};
pub use self::error::{ConfigError, ErrorCategory, Result};
pub use self::ingress::{
    AccessConfig, DurationSpec, IngressFlagRequest, IngressIpRule, IngressMatch, IngressRule, IngressService,
    NO_INGRESS_RULES_FLAGS_MESSAGE, NormalizedIngress, OriginRequestConfig, OriginRequestConfigBuilder,
    ProxyType, RawIngressRule, find_matching_rule, parse_ingress_flags,
};
pub use self::normalized::{NormalizationWarning, NormalizedConfig};
pub use self::raw_config::{RawConfig, WarpRoutingConfig};

pub fn parse_raw_config(source_name: &str, contents: &str) -> Result<RawConfig> {
    RawConfig::from_yaml_str(source_name, contents)
}

pub fn load_raw_config(path: &Path) -> Result<RawConfig> {
    RawConfig::from_yaml_path(path)
}

pub fn normalize_config(source: ConfigSource, raw: RawConfig) -> Result<NormalizedConfig> {
    NormalizedConfig::from_raw(source, raw)
}

pub fn load_normalized_config(path: &Path, source: ConfigSource) -> Result<NormalizedConfig> {
    let raw = load_raw_config(path)?;
    normalize_config(source, raw)
}
