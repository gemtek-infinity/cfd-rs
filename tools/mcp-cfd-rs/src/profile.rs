use crate::context::{
    ACTIVE_CONTEXT_PATH, BundleEntry, ContextBrief, ContextBundle, ContextSnapshot, SnapshotFact,
};
use std::path::{Path, PathBuf};

pub fn governance_roots(repo_root: &Path) -> Vec<PathBuf> {
    vec![
        repo_root.join("REWRITE_CHARTER.md"),
        repo_root.join("STATUS.md"),
        repo_root.join("AGENTS.md"),
        repo_root.join("SKILLS.md"),
        repo_root.join("docs"),
        repo_root.join(".github"),
    ]
}

pub fn behavior_truth_roots(repo_root: &Path) -> Vec<PathBuf> {
    vec![
        repo_root.join("baseline-2026.2.0/design-audit"),
        repo_root.join("baseline-2026.2.0/old-impl"),
    ]
}

pub fn supported_bundle_names() -> Vec<&'static str> {
    vec![
        "scope-lane",
        "repo-state",
        "active-surface",
        "first-slice-parity",
        "runtime-deps",
        "behavior-baseline",
    ]
}

pub fn supported_snapshot_names() -> Vec<&'static str> {
    vec![
        "active-context",
        "governing-files",
        "scope-lane",
        "repo-state",
        "active-phase",
        "runtime-deps",
        "behavior-baseline",
        "lane-decisions",
    ]
}

pub fn bundle(name: &str) -> Option<ContextBundle> {
    match name {
        "scope-lane" => Some(scope_lane_bundle()),
        "repo-state" => Some(repo_state_bundle()),
        "active-surface" => Some(active_surface_bundle()),
        "first-slice-parity" => Some(first_slice_parity_bundle()),
        "runtime-deps" => Some(runtime_deps_bundle()),
        "behavior-baseline" => Some(behavior_baseline_bundle()),
        _ => None,
    }
}

fn scope_lane_bundle() -> ContextBundle {
    ContextBundle {
        bundle: "scope-lane",
        summary: "Use this bundle for scope boundaries and governing files.",
        entries: vec![
            BundleEntry {
                path: "REWRITE_CHARTER.md".to_string(),
                reason: "Repository scope and non-negotiable boundaries.",
            },
            BundleEntry {
                path: "docs/compatibility-scope.md".to_string(),
                reason: "Compatibility scope definition.",
            },
            BundleEntry {
                path: "docs/promotion-gates.md".to_string(),
                reason: "Phase governance source when needed.",
            },
        ],
    }
}

fn repo_state_bundle() -> ContextBundle {
    ContextBundle {
        bundle: "repo-state",
        summary: "Use this bundle for current repository state sources.",
        entries: vec![
            BundleEntry {
                path: "STATUS.md".to_string(),
                reason: "Short index for current state.",
            },
            BundleEntry {
                path: "docs/status/rewrite-foundation.md".to_string(),
                reason: "Focused current-state details.",
            },
            BundleEntry {
                path: "docs/status/active-surface.md".to_string(),
                reason: "Focused implementation surface details.",
            },
        ],
    }
}

fn active_surface_bundle() -> ContextBundle {
    ContextBundle {
        bundle: "active-surface",
        summary: "Use this bundle for active context without embedding phase truth in code.",
        entries: vec![
            BundleEntry {
                path: ACTIVE_CONTEXT_PATH.to_string(),
                reason: "Preferred source for current active context when present.",
            },
            BundleEntry {
                path: "docs/promotion-gates.md".to_string(),
                reason: "Authoritative phase file when active context needs broader grounding.",
            },
            BundleEntry {
                path: "STATUS.md".to_string(),
                reason: "Current-state index.",
            },
        ],
    }
}

fn first_slice_parity_bundle() -> ContextBundle {
    ContextBundle {
        bundle: "first-slice-parity",
        summary: "Use this bundle for parity workflow sources.",
        entries: vec![
            BundleEntry {
                path: ACTIVE_CONTEXT_PATH.to_string(),
                reason: "Preferred active-slice source when present.",
            },
            BundleEntry {
                path: "docs/status/first-slice-parity.md".to_string(),
                reason: "Parity status details.",
            },
            BundleEntry {
                path: "tools/first_slice_parity.py".to_string(),
                reason: "Parity harness entry point.",
            },
        ],
    }
}

fn runtime_deps_bundle() -> ContextBundle {
    ContextBundle {
        bundle: "runtime-deps",
        summary: "Use this bundle for dependency and runtime policy sources.",
        entries: vec![
            BundleEntry {
                path: "docs/dependency-policy.md".to_string(),
                reason: "Dependency policy source.",
            },
            BundleEntry {
                path: "docs/allocator-runtime-baseline.md".to_string(),
                reason: "Allocator and runtime policy source.",
            },
            BundleEntry {
                path: "docs/go-rust-semantic-mapping.md".to_string(),
                reason: "Lifecycle and semantic mapping source.",
            },
        ],
    }
}

fn behavior_baseline_bundle() -> ContextBundle {
    ContextBundle {
        bundle: "behavior-baseline",
        summary: "Use this bundle for behavior/parity baseline sources.",
        entries: vec![
            BundleEntry {
                path: "baseline-2026.2.0/design-audit/REPO_SOURCE_INDEX.md".to_string(),
                reason: "Topic-to-source map into frozen baseline.",
            },
            BundleEntry {
                path: "baseline-2026.2.0/design-audit/REPO_REFERENCE.md".to_string(),
                reason: "Baseline reference index.",
            },
            BundleEntry {
                path: "baseline-2026.2.0/old-impl".to_string(),
                reason: "Frozen behavior source tree.",
            },
        ],
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
        "active-context" => Some(active_context_snapshot()),
        "governing-files" => Some(governing_files_snapshot()),
        "scope-lane" => Some(scope_lane_snapshot()),
        "repo-state" => Some(repo_state_snapshot()),
        "active-phase" => Some(active_phase_snapshot()),
        "runtime-deps" => Some(runtime_deps_snapshot()),
        "behavior-baseline" => Some(behavior_baseline_snapshot()),
        "lane-decisions" => Some(lane_decisions_snapshot()),
        _ => None,
    }
}

fn active_context_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "active-context",
        summary: "Active context is file-driven and should come from docs/ACTIVE_CONTEXT.md when present.",
        facts: vec![
            SnapshotFact {
                label: "preferred_source",
                value: "Use get_active_context to read docs/ACTIVE_CONTEXT.md with bounded output.",
            },
            SnapshotFact {
                label: "missing_behavior",
                value: "If docs/ACTIVE_CONTEXT.md is missing, do not infer current phase from code \
                        constants.",
            },
        ],
        source_paths: vec![
            ACTIVE_CONTEXT_PATH.to_string(),
            "docs/promotion-gates.md".to_string(),
            "STATUS.md".to_string(),
        ],
    }
}

fn governing_files_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "governing-files",
        summary: "Compact map of repository files by governance topic.",
        facts: vec![
            SnapshotFact {
                label: "scope_and_lane",
                value: "Use REWRITE_CHARTER.md and docs/compatibility-scope.md for scope boundaries.",
            },
            SnapshotFact {
                label: "current_state_and_phase",
                value: "Use STATUS.md and docs/promotion-gates.md for current state and phase governance.",
            },
            SnapshotFact {
                label: "dependencies_and_runtime",
                value: "Use docs/dependency-policy.md and docs/allocator-runtime-baseline.md.",
            },
            SnapshotFact {
                label: "behavior_and_parity",
                value: "Use baseline-2026.2.0/old-impl and baseline-2026.2.0/design-audit.",
            },
        ],
        source_paths: vec![
            "REWRITE_CHARTER.md".to_string(),
            "STATUS.md".to_string(),
            "docs/promotion-gates.md".to_string(),
            "docs/dependency-policy.md".to_string(),
            "baseline-2026.2.0/old-impl".to_string(),
        ],
    }
}

fn scope_lane_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "scope-lane",
        summary: "Compact map of scope/lane governance files.",
        facts: vec![
            SnapshotFact {
                label: "scope_owner",
                value: "Use REWRITE_CHARTER.md for non-negotiables and scope boundaries.",
            },
            SnapshotFact {
                label: "compatibility_owner",
                value: "Use docs/compatibility-scope.md for compatibility scope definition.",
            },
            SnapshotFact {
                label: "phase_owner",
                value: "Use docs/promotion-gates.md for phase governance.",
            },
        ],
        source_paths: vec![
            "REWRITE_CHARTER.md".to_string(),
            "docs/compatibility-scope.md".to_string(),
            "docs/promotion-gates.md".to_string(),
        ],
    }
}

fn repo_state_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "repo-state",
        summary: "Compact source map for current repository state.",
        facts: vec![
            SnapshotFact {
                label: "state_index",
                value: "STATUS.md is the short current-state index.",
            },
            SnapshotFact {
                label: "state_details",
                value: "Use docs/status/* for focused current-state details.",
            },
            SnapshotFact {
                label: "active_context",
                value: "Use docs/ACTIVE_CONTEXT.md when present for current active context.",
            },
        ],
        source_paths: vec![
            "STATUS.md".to_string(),
            "docs/status/rewrite-foundation.md".to_string(),
            ACTIVE_CONTEXT_PATH.to_string(),
        ],
    }
}

fn active_phase_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "active-phase",
        summary: "Active-phase truth is file-owned, not code-owned.",
        facts: vec![
            SnapshotFact {
                label: "phase_source",
                value: "Read docs/ACTIVE_CONTEXT.md first when present.",
            },
            SnapshotFact {
                label: "fallback_source",
                value: "If missing, read docs/promotion-gates.md directly.",
            },
            SnapshotFact {
                label: "guardrail",
                value: "Do not infer phase numbers or slice truth from hardcoded Rust constants.",
            },
        ],
        source_paths: vec![
            ACTIVE_CONTEXT_PATH.to_string(),
            "docs/promotion-gates.md".to_string(),
            "STATUS.md".to_string(),
        ],
    }
}

fn runtime_deps_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "runtime-deps",
        summary: "Compact map of runtime/dependency policy files.",
        facts: vec![
            SnapshotFact {
                label: "dependency_policy",
                value: "Use docs/dependency-policy.md.",
            },
            SnapshotFact {
                label: "runtime_policy",
                value: "Use docs/allocator-runtime-baseline.md.",
            },
            SnapshotFact {
                label: "semantic_mapping",
                value: "Use docs/go-rust-semantic-mapping.md.",
            },
        ],
        source_paths: vec![
            "docs/dependency-policy.md".to_string(),
            "docs/allocator-runtime-baseline.md".to_string(),
            "docs/go-rust-semantic-mapping.md".to_string(),
        ],
    }
}

fn behavior_baseline_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "behavior-baseline",
        summary: "Compact map of baseline behavior/parity sources.",
        facts: vec![
            SnapshotFact {
                label: "first_truth_source",
                value: "Use baseline-2026.2.0/old-impl code and tests.",
            },
            SnapshotFact {
                label: "second_truth_source",
                value: "Use baseline-2026.2.0/design-audit for routing and references.",
            },
            SnapshotFact {
                label: "guardrail",
                value: "Do not claim parity from rewrite shape alone.",
            },
        ],
        source_paths: vec![
            "baseline-2026.2.0/old-impl".to_string(),
            "baseline-2026.2.0/design-audit/REPO_SOURCE_INDEX.md".to_string(),
            "baseline-2026.2.0/design-audit/REPO_REFERENCE.md".to_string(),
        ],
    }
}

fn lane_decisions_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        snapshot: "lane-decisions",
        summary: "Use ADR files for lane decisions; code does not hardcode those decisions.",
        facts: vec![
            SnapshotFact {
                label: "transport_lane",
                value: "Read docs/adr/0002-transport-tls-crypto-lane.md.",
            },
            SnapshotFact {
                label: "pingora_lane",
                value: "Read docs/adr/0003-pingora-critical-path.md.",
            },
            SnapshotFact {
                label: "fips_lane",
                value: "Read docs/adr/0004-fips-in-alpha-definition.md.",
            },
            SnapshotFact {
                label: "deployment_lane",
                value: "Read docs/adr/0005-deployment-contract.md.",
            },
        ],
        source_paths: vec![
            "docs/adr/0002-transport-tls-crypto-lane.md".to_string(),
            "docs/adr/0003-pingora-critical-path.md".to_string(),
            "docs/adr/0004-fips-in-alpha-definition.md".to_string(),
            "docs/adr/0005-deployment-contract.md".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{brief, bundle, snapshot, supported_bundle_names, supported_snapshot_names};
    use crate::context::ACTIVE_CONTEXT_PATH;

    #[test]
    fn exposes_known_context_bundle() {
        let bundle = bundle("repo-state").expect("bundle should exist");

        assert_eq!(bundle.bundle, "repo-state");
        assert_eq!(bundle.entries.len(), 3);
    }

    #[test]
    fn exposes_compact_context_brief() {
        let brief = brief("repo-state").expect("brief should exist");

        assert_eq!(brief.bundle, "repo-state");
        assert_eq!(brief.first_path, "STATUS.md");
        assert_eq!(brief.next_paths.len(), 2);
    }

    #[test]
    fn exposes_active_context_snapshot() {
        let snapshot = snapshot("active-context").expect("snapshot should exist");

        assert_eq!(snapshot.snapshot, "active-context");
        assert_eq!(snapshot.facts.len(), 2);
        assert!(snapshot.source_paths.contains(&ACTIVE_CONTEXT_PATH.to_string()));
    }

    #[test]
    fn exposes_runtime_dependency_snapshot() {
        let snapshot = snapshot("runtime-deps").expect("snapshot should exist");

        assert_eq!(snapshot.snapshot, "runtime-deps");
        assert_eq!(snapshot.facts.len(), 3);
        assert!(
            snapshot
                .source_paths
                .contains(&"docs/dependency-policy.md".to_string())
        );
    }

    #[test]
    fn exposes_behavior_baseline_snapshot() {
        let snapshot = snapshot("behavior-baseline").expect("snapshot should exist");

        assert_eq!(snapshot.snapshot, "behavior-baseline");
        assert_eq!(snapshot.facts.len(), 3);
        assert!(
            snapshot
                .source_paths
                .contains(&"baseline-2026.2.0/old-impl".to_string())
        );
    }

    #[test]
    fn exposes_lane_decision_snapshot() {
        let snapshot = snapshot("lane-decisions").expect("snapshot should exist");

        assert_eq!(snapshot.snapshot, "lane-decisions");
        assert_eq!(snapshot.facts.len(), 4);
        assert!(
            snapshot
                .source_paths
                .contains(&"docs/adr/0002-transport-tls-crypto-lane.md".to_string())
        );
    }

    #[test]
    fn exposes_governing_file_snapshot() {
        let snapshot = snapshot("governing-files").expect("snapshot should exist");

        assert_eq!(snapshot.snapshot, "governing-files");
        assert_eq!(snapshot.facts.len(), 4);
        assert!(snapshot.source_paths.contains(&"REWRITE_CHARTER.md".to_string()));
    }

    #[test]
    fn advertises_supported_snapshot_names() {
        let supported = supported_snapshot_names();

        assert!(supported.contains(&"active-context"));
        assert!(supported.contains(&"scope-lane"));
        assert!(supported.contains(&"repo-state"));
        assert!(supported.contains(&"active-phase"));
        assert!(supported.contains(&"runtime-deps"));
        assert!(supported.contains(&"behavior-baseline"));
        assert!(supported.contains(&"lane-decisions"));
        assert!(supported.contains(&"governing-files"));
    }

    #[test]
    fn advertises_supported_bundle_names() {
        let supported = supported_bundle_names();

        assert!(supported.contains(&"scope-lane"));
        assert!(supported.contains(&"behavior-baseline"));
    }
}
