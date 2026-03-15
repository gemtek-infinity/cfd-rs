use cfdrs_shared::{ConfigSource, DiscoveryOutcome, NormalizedConfig};

mod render;
mod resolve;
mod runtime_overrides;

pub(crate) use self::render::{render_run_output, render_validate_output};
pub(crate) use self::resolve::resolve_startup;
pub(crate) use self::runtime_overrides::{PreparedRuntimeStartup, prepare_runtime_startup};

#[derive(Debug)]
pub(crate) struct StartupSurface {
    pub(crate) discovery: DiscoveryOutcome,
    pub(crate) normalized: NormalizedConfig,
}

pub(crate) fn config_source_label(source: &ConfigSource) -> &'static str {
    match source {
        ConfigSource::ExplicitPath(_) => "explicit",
        ConfigSource::DiscoveredPath(_) => "discovered",
        ConfigSource::AutoCreatedPath(_) => "auto-created",
    }
}
