use std::path::PathBuf;

use cloudflared_config::{ConfigError, DiscoveryRequest, discover_config, load_normalized_config};

use super::StartupSurface;

pub(crate) fn resolve_startup(config_path: Option<PathBuf>) -> Result<StartupSurface, ConfigError> {
    let request = DiscoveryRequest {
        explicit_config: config_path,
        ..DiscoveryRequest::default()
    };
    let discovery = discover_config(&request)?;
    let normalized = load_normalized_config(&discovery.path, discovery.source.clone())?;

    Ok(StartupSurface {
        discovery,
        normalized,
    })
}
