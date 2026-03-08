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

pub mod credentials;
pub mod discovery;
pub mod error;
pub mod ingress;
pub mod normalized;
pub mod raw_config;

pub use crate::credentials::{
    CredentialSurface, OriginCertLocator, OriginCertToken, TunnelCredentialsFile, TunnelReference,
};
pub use crate::discovery::{
    ConfigSource, DiscoveryCandidate, DiscoveryDefaults, DiscoveryOrigin, DiscoveryPlan, DiscoveryRequest,
    default_nix_log_directory, default_nix_primary_config_path, default_nix_search_directories,
};
pub use crate::error::{ConfigError, Result};
pub use crate::ingress::{
    AccessConfig, DurationSpec, IngressIpRule, IngressMatch, IngressRule, IngressService,
    OriginRequestConfig, RawIngressRule,
};
pub use crate::normalized::{NormalizationWarning, NormalizedConfig};
pub use crate::raw_config::{RawConfig, WarpRoutingConfig};

pub fn parse_raw_config(source_name: &str, contents: &str) -> Result<RawConfig> {
    RawConfig::from_yaml_str(source_name, contents)
}

pub fn normalize_config(source: ConfigSource, raw: RawConfig) -> Result<NormalizedConfig> {
    NormalizedConfig::from_raw(source, raw)
}
