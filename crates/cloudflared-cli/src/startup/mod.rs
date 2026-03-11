use cloudflared_config::{DiscoveryOutcome, NormalizedConfig};

mod render;
mod resolve;

pub(crate) use self::render::{config_source_label, render_run_output, render_validate_output};
pub(crate) use self::resolve::resolve_startup;

#[derive(Debug)]
pub(crate) struct StartupSurface {
    pub(crate) discovery: DiscoveryOutcome,
    pub(crate) normalized: NormalizedConfig,
}
