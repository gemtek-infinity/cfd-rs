use super::helpers::read_repo_text;
use super::{SOURCE_MAP_PATH, SourceMapEntry};
use std::collections::HashMap;
use std::path::Path;

pub(super) fn parse_source_map(repo_root: &Path) -> Result<HashMap<String, SourceMapEntry>, String> {
    let text = read_repo_text(repo_root, SOURCE_MAP_PATH)?;
    let mut lines = text.lines();
    let Some(header) = lines.next() else {
        return Err("source map is empty".to_string());
    };

    let columns = split_csv_row(header);
    let expected_header = vec![
        "row_id".to_string(),
        "domain".to_string(),
        "feature_doc".to_string(),
        "baseline_paths".to_string(),
        "symbol_hints".to_string(),
    ];

    if columns != expected_header {
        return Err("source map header does not match the expected contract".to_string());
    }

    let mut rows = HashMap::new();

    for (line_number, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let columns = split_csv_row(line);
        if columns.len() != 5 {
            return Err(format!(
                "source map row {} has {} columns, expected 5",
                line_number + 2,
                columns.len()
            ));
        }

        let row_id = columns[0].to_string();
        rows.insert(
            row_id,
            SourceMapEntry {
                feature_doc: columns[2].to_string(),
                baseline_paths: split_semicolon_list(&columns[3]),
                symbol_hints: split_semicolon_list(&columns[4]),
            },
        );
    }

    Ok(rows)
}

fn split_csv_row(line: &str) -> Vec<String> {
    let mut columns = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    let _ = chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                columns.push(current.clone());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    columns.push(current);
    columns
}

fn split_semicolon_list(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
