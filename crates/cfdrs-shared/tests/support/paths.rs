use std::path::PathBuf;

pub(super) fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/shared-behavior")
}

pub(super) fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    for ancestor in manifest_dir.ancestors() {
        if ancestor.join("Cargo.toml").exists() && ancestor.join("baseline-2026.2.0").exists() {
            return ancestor.to_path_buf();
        }
    }

    panic!("failed to locate repo root from {}", manifest_dir.display());
}

pub(super) fn tool_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    for ancestor in manifest_dir.ancestors() {
        let candidate = ancestor.join("tools/shared_behavior_parity.py");
        if candidate.exists() {
            return candidate;
        }
    }

    panic!(
        "failed to locate tools/shared_behavior_parity.py from {}",
        manifest_dir.display()
    );
}
