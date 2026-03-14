use crate::context::{BundleEntry, ContextBrief, ContextBundle, ContextSnapshot, SnapshotFact};
use std::path::{Path, PathBuf};

const SUPPORTED_BUNDLES: &[&str] = &[
    "status-core",
    "phase5-roadmap",
    "parity-cli",
    "parity-cdc",
    "parity-his",
    "runtime-deps",
    "behavior-baseline",
    "crate-ownership",
];

const SUPPORTED_SNAPSHOTS: &[&str] = &[
    "governing-files",
    "status-active",
    "phase5-milestone",
    "scope-lane",
    "runtime-deps",
    "behavior-baseline",
    "crate-ownership",
];

pub fn governance_roots(repo_root: &Path) -> Vec<PathBuf> {
    vec![
        repo_root.join("REWRITE_CHARTER.md"),
        repo_root.join("STATUS.md"),
        repo_root.join("AGENTS.md"),
        repo_root.join("docs"),
        repo_root.join(".github"),
    ]
}

pub fn behavior_truth_roots(repo_root: &Path) -> Vec<PathBuf> {
    vec![repo_root.join("baseline-2026.2.0"), repo_root.join("docs/parity")]
}

pub fn supported_bundle_names() -> Vec<&'static str> {
    SUPPORTED_BUNDLES.to_vec()
}

pub fn supported_snapshot_names() -> Vec<&'static str> {
    SUPPORTED_SNAPSHOTS.to_vec()
}

pub fn bundle(name: &str) -> Option<ContextBundle> {
    match name {
        "status-core" => Some(status_core_bundle()),
        "phase5-roadmap" => Some(phase5_roadmap_bundle()),
        "parity-cli" => Some(parity_bundle("CLI")),
        "parity-cdc" => Some(parity_bundle("CDC")),
        "parity-his" => Some(parity_bundle("HIS")),
        "runtime-deps" => Some(runtime_deps_bundle()),
        "behavior-baseline" => Some(behavior_baseline_bundle()),
        "crate-ownership" => Some(crate_ownership_bundle()),
        _ => None,
    }
}

pub fn brief(name: &str) -> Option<ContextBrief> {
    let bundle = bundle(name)?;
    let mut paths = bundle.entries.into_iter().map(|entry| entry.path);
    let first_path = paths.next()?;
    let next_paths = paths.collect();

    Some(ContextBrief {
        bundle: bundle.bundle,
        summary: bundle.summary,
        first_path,
        next_paths,
    })
}

pub fn snapshot(name: &str) -> Option<ContextSnapshot> {
    match name {
        "governing-files" => Some(governing_files_snapshot()),
        "status-active" => Some(status_active_snapshot()),
        "phase5-milestone" => Some(phase5_milestone_snapshot()),
        "scope-lane" => Some(scope_lane_snapshot()),
        "runtime-deps" => Some(runtime_deps_snapshot()),
        "behavior-baseline" => Some(behavior_baseline_snapshot()),
        "crate-ownership" => Some(crate_ownership_snapshot()),
        _ => None,
    }
}

fn status_core_bundle() -> ContextBundle {
    ContextBundle {
        bundle: "status-core",
        summary: "Use this bundle for tracked status and current implementation priority.",
        entries: vec![
            BundleEntry {
                path: "STATUS.md".to_string(),
                reason: "Tracked status source with the Active Snapshot startup surface.",
            },
            BundleEntry {
                path: "docs/phase-5/roadmap.md".to_string(),
                reason: "Normative Phase 5 roadmap.",
            },
            BundleEntry {
                path: "docs/phase-5/roadmap-index.csv".to_string(),
                reason: "Exact row-to-milestone mapping.",
            },
        ],
    }
}

fn phase5_roadmap_bundle() -> ContextBundle {
    ContextBundle {
        bundle: "phase5-roadmap",
        summary: "Use this bundle for implementation order, milestone gates, and row ownership.",
        entries: vec![
            BundleEntry {
                path: "docs/phase-5/roadmap.md".to_string(),
                reason: "Normative milestone plan and exit evidence.",
            },
            BundleEntry {
                path: "docs/phase-5/roadmap-index.csv".to_string(),
                reason: "Exact row map for the parity ledgers.",
            },
            BundleEntry {
                path: "docs/promotion-gates.md".to_string(),
                reason: "Compact promotion model and production-alpha gate.",
            },
        ],
    }
}

fn parity_bundle(domain: &str) -> ContextBundle {
    let (bundle, ledger_path, reason) = match domain {
        "CLI" => (
            "parity-cli",
            "docs/parity/cli/implementation-checklist.md",
            "CLI parity ledger.",
        ),
        "CDC" => (
            "parity-cdc",
            "docs/parity/cdc/implementation-checklist.md",
            "CDC parity ledger.",
        ),
        "HIS" => (
            "parity-his",
            "docs/parity/his/implementation-checklist.md",
            "HIS parity ledger.",
        ),
        _ => ("parity-cli", "docs/parity/README.md", "Parity index."),
    };

    ContextBundle {
        bundle,
        summary: "Use a single domain bundle first; do not preload all ledgers together.",
        entries: vec![
            BundleEntry {
                path: ledger_path.to_string(),
                reason,
            },
            BundleEntry {
                path: "docs/parity/README.md".to_string(),
                reason: "Parity index and domain document map.",
            },
            BundleEntry {
                path: "docs/phase-5/roadmap-index.csv".to_string(),
                reason: "Milestone and owner mapping for exact row IDs.",
            },
        ],
    }
}

fn runtime_deps_bundle() -> ContextBundle {
    ContextBundle {
        bundle: "runtime-deps",
        summary: "Use this bundle for dependency, allocator, and runtime policy questions.",
        entries: vec![
            BundleEntry {
                path: "docs/dependency-policy.md".to_string(),
                reason: "Dependency admission rules.",
            },
            BundleEntry {
                path: "docs/allocator-runtime-baseline.md".to_string(),
                reason: "Allocator and runtime baseline.",
            },
            BundleEntry {
                path: "docs/go-rust-semantic-mapping.md".to_string(),
                reason: "Lifecycle and concurrency doctrine.",
            },
            BundleEntry {
                path: "docs/adr/0001-hybrid-concurrency-model.md".to_string(),
                reason: "ADR-level runtime decision.",
            },
        ],
    }
}

fn behavior_baseline_bundle() -> ContextBundle {
    ContextBundle {
        bundle: "behavior-baseline",
        summary: "Use this bundle for frozen behavior truth and source routing.",
        entries: vec![
            BundleEntry {
                path: "docs/parity/source-map.csv".to_string(),
                reason: "Exact row-to-baseline routing for the parity ledgers.",
            },
            BundleEntry {
                path: "docs/parity/README.md".to_string(),
                reason: "Parity index and feature-document map.",
            },
            BundleEntry {
                path: "baseline-2026.2.0".to_string(),
                reason: "Frozen Go implementation tree.",
            },
        ],
    }
}

fn crate_ownership_bundle() -> ContextBundle {
    ContextBundle {
        bundle: "crate-ownership",
        summary: "Use this bundle for crate boundaries and ownership routing.",
        entries: vec![
            BundleEntry {
                path: "STATUS.md".to_string(),
                reason: "Architecture contract and allowed crate dependency direction.",
            },
            BundleEntry {
                path: "crates/cfdrs-bin/README.md".to_string(),
                reason: "Binary composition ownership.",
            },
            BundleEntry {
                path: "crates/cfdrs-cli/README.md".to_string(),
                reason: "CLI ownership.",
            },
            BundleEntry {
                path: "crates/cfdrs-cdc/README.md".to_string(),
                reason: "CDC ownership.",
            },
            BundleEntry {
                path: "crates/cfdrs-his/README.md".to_string(),
                reason: "HIS ownership.",
            },
            BundleEntry {
                path: "crates/cfdrs-shared/README.md".to_string(),
                reason: "Shared-type ownership.",
            },
        ],
    }
}

fn governing_files_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "governing-files",
        summary: "Compact map of the governing file set for rewrite work.",
        facts: vec![
            SnapshotFact {
                label: "tracked_status",
                value: "Use STATUS.md for the only tracked status source and Active Snapshot startup \
                        section.",
            },
            SnapshotFact {
                label: "roadmap",
                value: "Use docs/phase-5/roadmap.md and docs/phase-5/roadmap-index.csv for implementation \
                        order and exact row ownership.",
            },
            SnapshotFact {
                label: "scope_and_lane",
                value: "Use REWRITE_CHARTER.md, docs/compatibility-scope.md, and docs/promotion-gates.md \
                        for scope and promotion boundaries.",
            },
            SnapshotFact {
                label: "parity",
                value: "Use docs/parity/README.md and the relevant domain ledger for parity truth.",
            },
            SnapshotFact {
                label: "runtime_and_dependencies",
                value: "Use docs/dependency-policy.md and docs/allocator-runtime-baseline.md for dependency \
                        and runtime decisions.",
            },
            SnapshotFact {
                label: "behavior_truth",
                value: "Use baseline-2026.2.0 for frozen behavior truth and docs/parity/source-map.csv for \
                        bounded row-to-source routing.",
            },
        ],
        source_paths: vec![
            "STATUS.md".to_string(),
            "docs/phase-5/roadmap.md".to_string(),
            "REWRITE_CHARTER.md".to_string(),
            "docs/promotion-gates.md".to_string(),
            "docs/parity/README.md".to_string(),
            "CONTRIBUTING.md".to_string(),
        ],
    }
}

fn status_active_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "status-active",
        summary: "Current status is owned by STATUS.md, not by scattered phase trackers.",
        facts: vec![
            SnapshotFact {
                label: "startup_read",
                value: "Prefer status_summary first; if MCP is unavailable, read the Active Snapshot \
                        section in STATUS.md.",
            },
            SnapshotFact {
                label: "single_source",
                value: "STATUS.md is the only tracked status file for humans and AI.",
            },
            SnapshotFact {
                label: "guardrail",
                value: "Do not infer current work from deleted phase files, code comments, or branch names.",
            },
        ],
        source_paths: vec![
            "STATUS.md".to_string(),
            "docs/ai-context-routing.md".to_string(),
            "docs/promotion-gates.md".to_string(),
        ],
    }
}

fn phase5_milestone_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "phase5-milestone",
        summary: "Compact map of the active Phase 5 milestone surface.",
        facts: vec![
            SnapshotFact {
                label: "active_order",
                value: "Phase 5 advances through seven milestones ending at Performance Architecture \
                        Overhaul.",
            },
            SnapshotFact {
                label: "row_truth",
                value: "docs/phase-5/roadmap-index.csv is the exact row-to-milestone map for CLI, CDC, and \
                        HIS ledgers.",
            },
            SnapshotFact {
                label: "alpha_gate",
                value: "Production-alpha is claimed only after the final performance-architectural \
                        milestone reruns cleanly.",
            },
        ],
        source_paths: vec![
            "STATUS.md".to_string(),
            "docs/phase-5/roadmap.md".to_string(),
            "docs/phase-5/roadmap-index.csv".to_string(),
        ],
    }
}

fn scope_lane_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "scope-lane",
        summary: "Compact map of scope, lane, and promotion-boundary files.",
        facts: vec![
            SnapshotFact {
                label: "scope_owner",
                value: "Use REWRITE_CHARTER.md for non-negotiables and scope boundaries.",
            },
            SnapshotFact {
                label: "compatibility_owner",
                value: "Use docs/compatibility-scope.md for compatibility scope.",
            },
            SnapshotFact {
                label: "promotion_owner",
                value: "Use docs/promotion-gates.md for the current promotion model.",
            },
        ],
        source_paths: vec![
            "REWRITE_CHARTER.md".to_string(),
            "docs/compatibility-scope.md".to_string(),
            "docs/promotion-gates.md".to_string(),
        ],
    }
}

fn runtime_deps_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "runtime-deps",
        summary: "Compact map of runtime and dependency policy files.",
        facts: vec![
            SnapshotFact {
                label: "dependency_policy",
                value: "Use docs/dependency-policy.md for dependency admission rules.",
            },
            SnapshotFact {
                label: "runtime_policy",
                value: "Use docs/allocator-runtime-baseline.md for allocator and runtime baseline.",
            },
            SnapshotFact {
                label: "concurrency_policy",
                value: "Use docs/go-rust-semantic-mapping.md and ADR 0001 for concurrency doctrine.",
            },
        ],
        source_paths: vec![
            "docs/dependency-policy.md".to_string(),
            "docs/allocator-runtime-baseline.md".to_string(),
            "docs/go-rust-semantic-mapping.md".to_string(),
            "docs/adr/0001-hybrid-concurrency-model.md".to_string(),
        ],
    }
}

fn behavior_baseline_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "behavior-baseline",
        summary: "Compact map of the frozen behavior baseline sources.",
        facts: vec![
            SnapshotFact {
                label: "first_truth_source",
                value: "Use baseline-2026.2.0 code and tests for behavior truth.",
            },
            SnapshotFact {
                label: "routing_source",
                value: "Use docs/parity/source-map.csv to route one row back into the frozen baseline and \
                        matching parity feature doc.",
            },
            SnapshotFact {
                label: "guardrail",
                value: "Do not claim parity from rewrite shape alone.",
            },
        ],
        source_paths: vec![
            "baseline-2026.2.0".to_string(),
            "docs/parity/source-map.csv".to_string(),
            "docs/parity/README.md".to_string(),
        ],
    }
}

fn crate_ownership_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "crate-ownership",
        summary: "Compact map of workspace crate ownership and dependency direction.",
        facts: vec![
            SnapshotFact {
                label: "cfdrs_bin",
                value: "cfdrs-bin owns process entry, runtime composition, and orchestration seams.",
            },
            SnapshotFact {
                label: "cfdrs_cli",
                value: "cfdrs-cli owns user-visible command structure, help, flags, and dispatch.",
            },
            SnapshotFact {
                label: "cfdrs_cdc",
                value: "cfdrs-cdc owns Cloudflare-facing protocol and contract surfaces.",
            },
            SnapshotFact {
                label: "cfdrs_his",
                value: "cfdrs-his owns host interaction surfaces.",
            },
            SnapshotFact {
                label: "cfdrs_shared",
                value: "cfdrs-shared owns shared config, credentials, ingress, and error types only when \
                        multiple domains need them.",
            },
            SnapshotFact {
                label: "dependency_direction",
                value: "bin -> cli, cdc, his, shared; cli -> shared; cdc -> shared; his -> shared; no \
                        cross-dependencies among cli/cdc/his.",
            },
        ],
        source_paths: vec![
            "STATUS.md".to_string(),
            "crates/cfdrs-bin/README.md".to_string(),
            "crates/cfdrs-cli/README.md".to_string(),
            "crates/cfdrs-cdc/README.md".to_string(),
            "crates/cfdrs-his/README.md".to_string(),
            "crates/cfdrs-shared/README.md".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{brief, bundle, snapshot, supported_bundle_names, supported_snapshot_names};
    use std::fs;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .map(|path| path.to_path_buf())
            .expect("repo root")
    }

    fn bullet_list_from_doc(section_heading: &str) -> Vec<String> {
        let path = repo_root().join("docs/ai-context-routing.md");
        let text = fs::read_to_string(path).expect("routing doc");
        let mut capture = false;
        let mut items = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();

            if !capture {
                if trimmed == section_heading {
                    capture = true;
                }
                continue;
            }

            if trimmed.starts_with("#") && trimmed != section_heading {
                break;
            }

            if let Some(item) = trimmed.strip_prefix("- `")
                && let Some(value) = item.strip_suffix('`')
            {
                items.push(value.to_string());
            }
        }

        items
    }

    #[test]
    fn exposes_status_core_bundle() {
        let bundle = bundle("status-core").expect("bundle should exist");

        assert_eq!(bundle.bundle, "status-core");
        assert_eq!(
            bundle.entries.first().map(|entry| entry.path.as_str()),
            Some("STATUS.md")
        );
    }

    #[test]
    fn exposes_compact_context_brief() {
        let brief = brief("status-core").expect("brief should exist");

        assert_eq!(brief.bundle, "status-core");
        assert_eq!(brief.first_path, "STATUS.md");
    }

    #[test]
    fn exposes_status_active_snapshot() {
        let snapshot = snapshot("status-active").expect("snapshot should exist");

        assert_eq!(snapshot.snapshot, "status-active");
        assert_eq!(snapshot.facts.len(), 3);
        assert!(snapshot.source_paths.contains(&"STATUS.md".to_string()));
    }

    #[test]
    fn exposes_phase5_milestone_snapshot() {
        let snapshot = snapshot("phase5-milestone").expect("snapshot should exist");

        assert_eq!(snapshot.snapshot, "phase5-milestone");
        assert!(
            snapshot
                .source_paths
                .contains(&"docs/phase-5/roadmap-index.csv".to_string())
        );
    }

    #[test]
    fn exposes_behavior_baseline_snapshot() {
        let snapshot = snapshot("behavior-baseline").expect("snapshot should exist");

        assert_eq!(snapshot.snapshot, "behavior-baseline");
        assert!(snapshot.source_paths.contains(&"baseline-2026.2.0".to_string()));
    }

    #[test]
    fn supported_bundle_names_match_routing_doc() {
        let from_doc = bullet_list_from_doc("### Core bundles");
        let from_code = supported_bundle_names()
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>();

        assert_eq!(from_code, from_doc);
    }

    #[test]
    fn supported_snapshot_names_match_routing_doc() {
        let from_doc = bullet_list_from_doc("### Core snapshots");
        let from_code = supported_snapshot_names()
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>();

        assert_eq!(from_code, from_doc);
    }
}
