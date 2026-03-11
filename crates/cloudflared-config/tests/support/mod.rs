mod fixture_index;
mod paths;

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
    paths::fixtures_root()
}

#[allow(dead_code)]
pub(crate) fn repo_root() -> PathBuf {
    paths::repo_root()
}

#[allow(dead_code)]
pub(crate) fn fixture_entries() -> Vec<FixtureEntry> {
    let index_path = fixtures_root().join("fixture-index.toml");
    let contents = fs::read_to_string(&index_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", index_path.display()));
    let entries = fixture_index::parse_fixture_entries(&contents);

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
    paths::tool_path()
}
