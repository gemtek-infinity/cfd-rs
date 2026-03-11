use crate::fs as mcp_fs;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tokio::fs;

use super::analysis::MAX_ANALYZABLE_SIZE;

// ---------------------------------------------------------------------------
// File reading
// ---------------------------------------------------------------------------

pub async fn read_analyzable(path: &Path) -> Result<String, &'static str> {
    let meta = fs::symlink_metadata(path)
        .await
        .map_err(|_| "file not found or not readable")?;

    if !meta.is_file() {
        return Err("path is not a regular file");
    }

    if meta.len() > MAX_ANALYZABLE_SIZE {
        return Err("file too large for Debtmap analysis");
    }

    if !mcp_fs::is_text_file(path) {
        return Err("not a recognized text file type");
    }

    fs::read_to_string(path)
        .await
        .map_err(|_| "file not readable as UTF-8 text")
}

// ---------------------------------------------------------------------------
// File collection
// ---------------------------------------------------------------------------

pub async fn collect_analyzable_files(base: &Path, out: &mut BTreeSet<PathBuf>) {
    let Ok(meta) = fs::symlink_metadata(base).await else {
        return;
    };

    if meta.file_type().is_symlink() {
        return;
    }

    if meta.is_file() {
        if is_analyzable_file(base, &meta) {
            out.insert(base.to_path_buf());
        }
        return;
    }

    if meta.is_dir() {
        walk_dir_for_analysis(base, out).await;
    }
}

async fn walk_dir_for_analysis(start: &Path, out: &mut BTreeSet<PathBuf>) {
    let mut stack = vec![start.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(mut read_dir) = fs::read_dir(&dir).await else {
            continue;
        };

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            classify_dir_entry(entry, out, &mut stack).await;
        }
    }
}

async fn classify_dir_entry(
    entry: tokio::fs::DirEntry,
    out: &mut BTreeSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
) {
    let path = entry.path();

    let Ok(meta) = fs::symlink_metadata(&path).await else {
        return;
    };

    if meta.file_type().is_symlink() {
        return;
    }

    if meta.is_dir() {
        stack.push(path);
    } else if is_analyzable_file(&path, &meta) {
        out.insert(path);
    }
}

fn is_analyzable_file(path: &Path, meta: &std::fs::Metadata) -> bool {
    meta.is_file() && meta.len() <= MAX_ANALYZABLE_SIZE && mcp_fs::is_text_file(path)
}
