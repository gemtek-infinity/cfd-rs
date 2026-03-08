use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) struct FixtureEntry {
    pub(crate) id: String,
    pub(crate) input: String,
}

#[allow(dead_code)]
pub(crate) fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/first-slice")
}

#[allow(dead_code)]
pub(crate) fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    for ancestor in manifest_dir.ancestors() {
        if ancestor.join("Cargo.toml").exists() && ancestor.join("baseline-2026.2.0").exists() {
            return ancestor.to_path_buf();
        }
    }

    panic!("failed to locate repo root from {}", manifest_dir.display());
}

#[allow(dead_code)]
pub(crate) fn fixture_entries() -> Vec<FixtureEntry> {
    let index_path = fixtures_root().join("fixture-index.toml");
    let contents = fs::read_to_string(&index_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", index_path.display()));

    let mut entries = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_input: Option<String> = None;

    for raw_line in contents.lines() {
        let line = raw_line.trim();

        if line == "[[fixture]]" {
            if let (Some(id), Some(input)) = (current_id.take(), current_input.take()) {
                entries.push(FixtureEntry { id, input });
            }
            continue;
        }

        if let Some(value) = parse_string_value(line, "id") {
            current_id = Some(value);
            continue;
        }

        if let Some(value) = parse_string_value(line, "input") {
            current_input = Some(value);
        }
    }

    if let (Some(id), Some(input)) = (current_id.take(), current_input.take()) {
        entries.push(FixtureEntry { id, input });
    }

    assert!(
        !entries.is_empty(),
        "fixture index must contain at least one [[fixture]] entry"
    );

    entries
}

#[allow(dead_code)]
pub(crate) fn fixture_ids() -> Vec<String> {
    fixture_entries().into_iter().map(|entry| entry.id).collect()
}

#[allow(dead_code)]
pub(crate) fn tool_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    for ancestor in manifest_dir.ancestors() {
        let candidate = ancestor.join("tools/first_slice_parity.py");
        if candidate.exists() {
            return candidate;
        }
    }

    panic!(
        "failed to locate tools/first_slice_parity.py from {}",
        manifest_dir.display()
    );
}

#[allow(dead_code)]
fn parse_string_value(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} = ");
    if !line.starts_with(&prefix) {
        return None;
    }

    let quoted = line[prefix.len()..].trim();
    if !(quoted.starts_with('"') && quoted.ends_with('"')) {
        return None;
    }

    Some(quoted[1..quoted.len() - 1].to_owned())
}
