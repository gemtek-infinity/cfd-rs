#![forbid(unsafe_code)]

//! Domain boundary for the accepted first slice.
//!
//! This crate now owns the Phase 1B.1 domain skeleton for:
//!
//! - config discovery inputs and sources
//! - raw YAML-backed config representation
//! - normalized config representation
//! - credentials and origin-cert surface types
//! - ingress rule representation
//! - explicit error taxonomy
//!
//! Externally visible first-slice behavior is still incomplete. This crate is a
//! synchronous domain skeleton, not a parity-complete implementation.

use std::path::Path;

pub mod artifact;
pub mod credentials;
pub mod discovery;
pub mod error;
pub mod ingress;
pub mod normalized;
pub mod raw_config;

pub use crate::credentials::{
    CredentialSurface, FED_ENDPOINT, OriginCertLocator, OriginCertToken, OriginCertUser,
    TunnelCredentialsFile, TunnelReference,
};
pub use crate::discovery::{
    ConfigSource, DiscoveryAction, DiscoveryCandidate, DiscoveryDefaults, DiscoveryOrigin, DiscoveryOutcome,
    DiscoveryPlan, DiscoveryRequest, default_nix_log_directory, default_nix_primary_config_path,
    default_nix_search_directories,
};
pub use crate::error::{ConfigError, Result};
pub use crate::ingress::{
    AccessConfig, DurationSpec, IngressFlagRequest, IngressIpRule, IngressMatch, IngressRule, IngressService,
    NO_INGRESS_RULES_FLAGS_MESSAGE, NormalizedIngress, OriginRequestConfig, OriginRequestConfigBuilder,
    RawIngressRule, find_matching_rule, parse_ingress_flags,
};
pub use crate::normalized::{NormalizationWarning, NormalizedConfig};
pub use crate::raw_config::{RawConfig, WarpRoutingConfig};

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

pub fn discover_config(request: &DiscoveryRequest) -> Result<DiscoveryOutcome> {
    request.find_or_create_config_path()
}
