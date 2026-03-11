use cloudflared_config::{ConfigSource, DiscoveryAction, IngressService, NormalizationWarning};

use crate::runtime::RuntimeExecution;

use super::StartupSurface;

pub(crate) fn render_validate_output(startup: &StartupSurface) -> String {
    let mut lines = vec![String::from("OK: admitted alpha startup surface validated")];
    lines.extend(render_startup_lines(startup));
    lines.join("\n") + "\n"
}

pub(crate) fn render_run_output(startup: &StartupSurface, report: &RuntimeExecution) -> String {
    let mut lines = vec![String::from("Resolved admitted alpha startup surface")];
    lines.extend(render_startup_lines(startup));
    lines.extend(report.summary_lines.iter().cloned());
    lines.join("\n") + "\n"
}

pub(crate) fn config_source_label(source: &ConfigSource) -> &'static str {
    match source {
        ConfigSource::ExplicitPath(_) => "explicit",
        ConfigSource::DiscoveredPath(_) => "discovered",
        ConfigSource::AutoCreatedPath(_) => "auto-created",
    }
}

fn render_startup_lines(startup: &StartupSurface) -> Vec<String> {
    let mut lines = vec![
        format!(
            "config-source: {}",
            config_source_label(&startup.discovery.source)
        ),
        format!("config-path: {}", startup.discovery.path.display()),
        format!("ingress-rules: {}", startup.normalized.ingress.len()),
        String::from("startup-readiness: admitted-for-runtime-handoff"),
    ];

    if startup.discovery.action == DiscoveryAction::CreateDefaultConfig {
        lines.push(String::from("created-default-config: yes"));
    }

    match warning_summary(&startup.normalized.warnings) {
        Some(summary) => lines.push(format!("warnings: {summary}")),
        None => lines.push(String::from("warnings: none")),
    }

    if startup.normalized.ingress.len() == 1
        && startup.normalized.ingress[0].service == IngressService::HttpStatus(503)
    {
        lines.push(String::from(
            "ingress-default: catch-all http_status:503 is admitted when no ingress rules are configured",
        ));
    }

    lines
}

fn warning_summary(warnings: &[NormalizationWarning]) -> Option<String> {
    let mut parts = Vec::new();

    for warning in warnings {
        match warning {
            NormalizationWarning::UnknownTopLevelKeys(keys) => {
                parts.push(format!("unknown-top-level-keys={}", keys.join(",")));
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}
