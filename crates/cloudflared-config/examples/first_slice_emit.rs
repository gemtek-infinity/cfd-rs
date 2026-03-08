#![allow(unused_crate_dependencies)]

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cloudflared_config::artifact::{
    DiscoveryCase, DiscoveryReportPayload, EmissionPlan, FixtureSpec, credential_envelope,
    discovery_envelope, error_envelope, normalized_config_envelope,
};
use cloudflared_config::{
    ConfigSource, DiscoveryDefaults, DiscoveryRequest, OriginCertToken, discover_config,
    load_normalized_config,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plan = read_plan()?;
    fs::create_dir_all(&plan.output_dir)?;

    for fixture in &plan.fixtures {
        let envelope = match fixture.category.as_str() {
            "config-discovery" => emit_discovery_fixture(fixture)?,
            "yaml-config" | "ordering-defaulting" | "invalid-input" => {
                emit_config_fixture(&plan.fixture_root, fixture)?
            }
            "credentials-origin-cert" => emit_origin_cert_fixture(&plan, fixture)?,
            other => {
                return Err(format!("unsupported fixture category for current first slice: {other}").into());
            }
        };

        let output_path = plan.output_dir.join(format!("{}.json", fixture.fixture_id));
        fs::write(output_path, serde_json::to_string_pretty(&envelope)?)?;
    }

    Ok(())
}

fn read_plan() -> Result<EmissionPlan, Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    Ok(serde_json::from_str(&input)?)
}

fn emit_discovery_fixture(
    fixture: &FixtureSpec,
) -> Result<cloudflared_config::artifact::ArtifactEnvelope, Box<dyn std::error::Error>> {
    let Some(discovery_case) = fixture.discovery_case.as_ref() else {
        return Err(format!("fixture {} is missing discovery case data", fixture.fixture_id).into());
    };

    let sandbox_root = build_discovery_sandbox(discovery_case)?;
    let request = DiscoveryRequest {
        explicit_config: explicit_config_path(discovery_case, &sandbox_root),
        defaults: discovery_defaults(&sandbox_root),
    };

    let outcome = discover_config(&request)?;
    let payload = DiscoveryReportPayload::from_outcome(&outcome, &sandbox_root);
    let envelope = discovery_envelope(fixture, payload)?;
    fs::remove_dir_all(&sandbox_root)?;
    Ok(envelope)
}

fn emit_config_fixture(
    fixture_root: &Path,
    fixture: &FixtureSpec,
) -> Result<cloudflared_config::artifact::ArtifactEnvelope, Box<dyn std::error::Error>> {
    let input_path = fixture_root.join(&fixture.input);
    let source = ConfigSource::DiscoveredPath(PathBuf::from(&fixture.input));

    match load_normalized_config(&input_path, source) {
        Ok(normalized) => Ok(normalized_config_envelope(
            fixture,
            Path::new(&fixture.input),
            &normalized,
        )?),
        Err(error) => Ok(error_envelope(fixture, &error)?),
    }
}

fn emit_origin_cert_fixture(
    plan: &EmissionPlan,
    fixture: &FixtureSpec,
) -> Result<cloudflared_config::artifact::ArtifactEnvelope, Box<dyn std::error::Error>> {
    let Some(source_path) = fixture.origin_cert_source.as_ref() else {
        return Err(format!(
            "fixture {} is missing origin cert source data",
            fixture.fixture_id
        )
        .into());
    };

    let input_path = plan.repo_root.join(source_path);
    match OriginCertToken::from_pem_path(&input_path) {
        Ok(token) => Ok(credential_envelope(fixture, source_path, &token)?),
        Err(error) => Ok(error_envelope(fixture, &error)?),
    }
}

fn build_discovery_sandbox(case: &DiscoveryCase) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let sandbox_root = std::env::temp_dir().join(format!("cloudflared-config-discovery-{unique}"));
    fs::create_dir_all(&sandbox_root)?;

    for logical_path in &case.present {
        let actual_path = logical_to_sandbox_path(&sandbox_root, logical_path);
        if let Some(parent) = actual_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(actual_path, "logDirectory: /var/log/cloudflared\n")?;
    }

    Ok(sandbox_root)
}

fn explicit_config_path(case: &DiscoveryCase, sandbox_root: &Path) -> Option<PathBuf> {
    if case.explicit_config {
        Some(logical_to_sandbox_path(
            sandbox_root,
            "home/.cloudflared/config.yml",
        ))
    } else {
        None
    }
}

fn discovery_defaults(sandbox_root: &Path) -> DiscoveryDefaults {
    DiscoveryDefaults {
        config_filenames: vec!["config.yml".to_owned(), "config.yaml".to_owned()],
        search_directories: vec![
            logical_to_sandbox_path(sandbox_root, "home/.cloudflared"),
            logical_to_sandbox_path(sandbox_root, "home/.cloudflare-warp"),
            logical_to_sandbox_path(sandbox_root, "home/cloudflare-warp"),
            logical_to_sandbox_path(sandbox_root, "etc/cloudflared"),
            logical_to_sandbox_path(sandbox_root, "usr/local/etc/cloudflared"),
        ],
        primary_config_path: logical_to_sandbox_path(sandbox_root, "usr/local/etc/cloudflared/config.yml"),
        primary_log_directory: logical_to_sandbox_path(sandbox_root, "var/log/cloudflared"),
    }
}

fn logical_to_sandbox_path(sandbox_root: &Path, logical_path: &str) -> PathBuf {
    sandbox_root.join(logical_path.trim_start_matches('/'))
}
