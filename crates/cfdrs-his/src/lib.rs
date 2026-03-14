#![forbid(unsafe_code)]

//! Host-facing service behavior: filesystem config discovery, credential
//! file lookup, filesystem layout, service installation, supervision
//! integration, watcher and reload, diagnostics collection, environment
//! and privilege assumptions, and local endpoint exposure.
//!
//! This crate owns the 74-row HIS parity surface: all interactions between
//! cloudflared and the local host.
//!
//! Config *types* and in-memory parsing live in `cfdrs-shared`.
//! This crate owns the filesystem discovery workflow: finding config files
//! on disk, creating default configs, and resolving platform-specific paths.

pub mod credentials;
pub mod diagnostics;
pub mod discovery;
pub mod environment;
pub mod hello;
pub mod icmp;
pub mod logging;
pub mod metrics_server;
pub mod process;
pub mod service;
pub mod signal;
pub mod updater;
pub mod watcher;

/// Discover config by searching platform-specific paths or creating a default.
pub fn discover_config(
    request: &cfdrs_shared::DiscoveryRequest,
) -> cfdrs_shared::Result<cfdrs_shared::DiscoveryOutcome> {
    discovery::find_or_create_config_path(request)
}
