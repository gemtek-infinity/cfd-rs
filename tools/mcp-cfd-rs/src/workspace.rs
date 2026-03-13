use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const WORKSPACE_CRATES: &[(&str, &str, &str)] = &[
    (
        "cfdrs-bin",
        "crates/cfdrs-bin/Cargo.toml",
        "crates/cfdrs-bin/README.md",
    ),
    (
        "cfdrs-cli",
        "crates/cfdrs-cli/Cargo.toml",
        "crates/cfdrs-cli/README.md",
    ),
    (
        "cfdrs-cdc",
        "crates/cfdrs-cdc/Cargo.toml",
        "crates/cfdrs-cdc/README.md",
    ),
    (
        "cfdrs-his",
        "crates/cfdrs-his/Cargo.toml",
        "crates/cfdrs-his/README.md",
    ),
    (
        "cfdrs-shared",
        "crates/cfdrs-shared/Cargo.toml",
        "crates/cfdrs-shared/README.md",
    ),
];

#[derive(Debug, Clone, Serialize)]
pub struct CrateSurfaceSummaryResponse {
    pub source_paths: Vec<String>,
    pub crate_name: String,
    pub summary: String,
    pub owns: Vec<String>,
    pub governing_docs: Vec<String>,
    pub direct_dependencies: Vec<String>,
    pub direct_dependents: Vec<String>,
    pub policy_ok: bool,
    pub policy_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub crate_name: String,
    pub cargo_toml_path: String,
    pub readme_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub allowed: bool,
    pub policy_scoped: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrateDependencyGraphResponse {
    pub source_paths: Vec<String>,
    pub allowed_rules: Vec<String>,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub violations: Vec<String>,
}

pub fn crate_surface_summary(
    repo_root: &Path,
    crate_name: &str,
) -> Result<CrateSurfaceSummaryResponse, String> {
    let normalized = normalize_crate_name(crate_name)?;
    let graph = crate_dependency_graph(repo_root)?;
    let (cargo_toml_path, readme_path) = crate_paths(&normalized)?;
    let readme_text = read_repo_text(repo_root, readme_path)?;
    let sections = parse_sections(&readme_text, "## ");

    let direct_dependencies: Vec<String> = graph
        .edges
        .iter()
        .filter(|edge| edge.from == normalized && edge.kind == "dependencies")
        .map(|edge| edge.to.clone())
        .collect();

    let direct_dependents: Vec<String> = graph
        .edges
        .iter()
        .filter(|edge| edge.to == normalized && edge.kind == "dependencies")
        .map(|edge| edge.from.clone())
        .collect();

    let policy_notes = graph
        .violations
        .iter()
        .filter(|violation| violation.contains(&normalized))
        .cloned()
        .collect::<Vec<_>>();

    Ok(CrateSurfaceSummaryResponse {
        source_paths: vec![
            cargo_toml_path.to_string(),
            readme_path.to_string(),
            "STATUS.md".to_string(),
        ],
        crate_name: normalized,
        summary: readme_summary(&readme_text),
        owns: section_bullets(&sections, "Owns"),
        governing_docs: section_bullets(&sections, "Governing docs"),
        direct_dependencies,
        direct_dependents,
        policy_ok: policy_notes.is_empty(),
        policy_notes,
    })
}

pub fn crate_dependency_graph(repo_root: &Path) -> Result<CrateDependencyGraphResponse, String> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut source_paths = Vec::new();

    for (crate_name, cargo_toml_path, readme_path) in WORKSPACE_CRATES {
        nodes.push(GraphNode {
            crate_name: (*crate_name).to_string(),
            cargo_toml_path: (*cargo_toml_path).to_string(),
            readme_path: (*readme_path).to_string(),
        });
        source_paths.push((*cargo_toml_path).to_string());
        source_paths.push((*readme_path).to_string());

        let manifest = read_repo_text(repo_root, cargo_toml_path)?;
        let manifest_edges = parse_manifest_edges(crate_name, &manifest)?;

        for edge in manifest_edges {
            edges.push(GraphEdge {
                allowed: edge.kind != "dev-dependencies" && dependency_allowed(&edge.from, &edge.to),
                policy_scoped: edge.kind != "dev-dependencies",
                from: edge.from,
                to: edge.to,
                kind: edge.kind,
            });
        }
    }

    edges.sort_by(|left, right| {
        (left.from.as_str(), left.kind.as_str(), left.to.as_str()).cmp(&(
            right.from.as_str(),
            right.kind.as_str(),
            right.to.as_str(),
        ))
    });

    let violations = edges
        .iter()
        .filter(|edge| edge.policy_scoped && !edge.allowed)
        .map(|edge| format!("{} -> {} is not allowed in [{}]", edge.from, edge.to, edge.kind))
        .collect();

    source_paths.push("STATUS.md".to_string());

    Ok(CrateDependencyGraphResponse {
        source_paths,
        allowed_rules: vec![
            "cfdrs-bin -> cfdrs-cli, cfdrs-cdc, cfdrs-his, cfdrs-shared".to_string(),
            "cfdrs-cli -> cfdrs-shared".to_string(),
            "cfdrs-cdc -> cfdrs-shared".to_string(),
            "cfdrs-his -> cfdrs-shared".to_string(),
            "cfdrs-shared -> no domain crates".to_string(),
            "CLI, CDC, and HIS do not depend on each other directly".to_string(),
        ],
        nodes,
        edges,
        violations,
    })
}

#[derive(Debug, Clone)]
struct ManifestEdge {
    from: String,
    to: String,
    kind: String,
}

fn crate_paths(crate_name: &str) -> Result<(&'static str, &'static str), String> {
    WORKSPACE_CRATES
        .iter()
        .find(|(name, _, _)| *name == crate_name)
        .map(|(_, cargo_toml, readme)| (*cargo_toml, *readme))
        .ok_or_else(|| format!("unknown workspace crate: {crate_name}"))
}

fn normalize_crate_name(crate_name: &str) -> Result<String, String> {
    let normalized = crate_name.trim();
    if WORKSPACE_CRATES.iter().any(|(name, _, _)| *name == normalized) {
        return Ok(normalized.to_string());
    }

    Err(format!("unknown workspace crate: {crate_name}"))
}

fn dependency_allowed(from: &str, to: &str) -> bool {
    let allowed = allowed_dependencies();
    allowed
        .get(from)
        .map(|targets| targets.contains(to))
        .unwrap_or(false)
}

fn allowed_dependencies() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    BTreeMap::from([
        (
            "cfdrs-bin",
            BTreeSet::from(["cfdrs-cli", "cfdrs-cdc", "cfdrs-his", "cfdrs-shared"]),
        ),
        ("cfdrs-cli", BTreeSet::from(["cfdrs-shared"])),
        ("cfdrs-cdc", BTreeSet::from(["cfdrs-shared"])),
        ("cfdrs-his", BTreeSet::from(["cfdrs-shared"])),
        ("cfdrs-shared", BTreeSet::new()),
    ])
}

fn read_repo_text(repo_root: &Path, relative_path: &str) -> Result<String, String> {
    fs::read_to_string(repo_root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))
}

fn parse_manifest_edges(crate_name: &str, manifest: &str) -> Result<Vec<ManifestEdge>, String> {
    let workspace_crates = WORKSPACE_CRATES
        .iter()
        .map(|(name, _, _)| *name)
        .collect::<BTreeSet<_>>();
    let mut edges = Vec::new();
    let mut current_section = String::new();

    for line in manifest.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed.trim_matches(['[', ']']).to_string();
            continue;
        }

        if !matches!(
            current_section.as_str(),
            "dependencies" | "build-dependencies" | "dev-dependencies"
        ) {
            continue;
        }

        if trimmed.is_empty() || trimmed.starts_with('#') || line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }

        let Some((name, _)) = trimmed.split_once('=') else {
            continue;
        };
        let dependency_name = name.trim().split('.').next().unwrap_or(name.trim());

        if !workspace_crates.contains(dependency_name) {
            continue;
        }

        edges.push(ManifestEdge {
            from: crate_name.to_string(),
            to: dependency_name.to_string(),
            kind: current_section.clone(),
        });
    }

    if edges.is_empty() && crate_name == "cfdrs-bin" {
        return Err("failed to parse crate dependencies from cfdrs-bin manifest".to_string());
    }

    Ok(edges)
}

fn parse_sections(text: &str, prefix: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_lines = Vec::new();

    for line in text.lines() {
        if let Some(title) = line.strip_prefix(prefix) {
            if let Some(previous_title) = current_title.replace(title.trim().to_string()) {
                sections.push((previous_title, current_lines.join("\n").trim().to_string()));
                current_lines.clear();
            }
            continue;
        }

        if current_title.is_some() {
            current_lines.push(line.to_string());
        }
    }

    if let Some(title) = current_title {
        sections.push((title, current_lines.join("\n").trim().to_string()));
    }

    sections
}

fn readme_summary(text: &str) -> String {
    let mut lines = Vec::new();

    for line in text.lines().skip(1) {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !lines.is_empty() {
                break;
            }
            continue;
        }

        if trimmed.starts_with("## ") {
            break;
        }

        lines.push(trimmed.to_string());
    }

    lines.join(" ")
}

fn section_bullets(sections: &[(String, String)], title: &str) -> Vec<String> {
    sections
        .iter()
        .find(|(section_title, _)| section_title == title)
        .map(|(_, content)| {
            content
                .lines()
                .filter_map(|line| line.trim().strip_prefix("- "))
                .map(|line| line.trim().trim_matches('`').to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{crate_dependency_graph, crate_surface_summary};
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .map(|path| path.to_path_buf())
            .expect("repo root")
    }

    #[test]
    fn workspace_dependency_graph_has_no_policy_violations() {
        let graph = crate_dependency_graph(&repo_root()).expect("graph");

        assert!(graph.violations.is_empty(), "{:?}", graph.violations);
    }

    #[test]
    fn crate_surface_summary_reports_cli_surface() {
        let summary = crate_surface_summary(&repo_root(), "cfdrs-cli").expect("crate surface");

        assert_eq!(summary.crate_name, "cfdrs-cli");
        assert!(summary.direct_dependencies.contains(&"cfdrs-shared".to_string()));
        assert!(summary.policy_ok);
    }
}
