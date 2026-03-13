use std::path::PathBuf;

use cfdrs_shared::{ConfigSource, DiscoveryAction, DiscoveryOutcome, NormalizedConfig, RawConfig};

use super::super::{RuntimeConfig, RuntimeExecution};

pub(super) fn runtime_config() -> RuntimeConfig {
    let raw = RawConfig::from_yaml_str(
        "runtime-test.yaml",
        "tunnel: runtime-test\ningress:\n  - service: http_status:503\n",
    )
    .expect("runtime config fixture should parse");
    let normalized =
        NormalizedConfig::from_raw(ConfigSource::ExplicitPath("/tmp/runtime-test.yml".into()), raw)
            .expect("runtime config fixture should normalize");
    let discovery = DiscoveryOutcome {
        action: DiscoveryAction::UseExisting,
        source: ConfigSource::ExplicitPath(PathBuf::from("/tmp/runtime-test.yml")),
        path: PathBuf::from("/tmp/runtime-test.yml"),
        created_paths: Vec::new(),
        written_config: None,
    };

    RuntimeConfig::new(discovery, normalized)
}

pub(super) fn summary_contains(execution: &RuntimeExecution, needle: &str) -> bool {
    execution.summary_lines.iter().any(|line| line.contains(needle))
}
