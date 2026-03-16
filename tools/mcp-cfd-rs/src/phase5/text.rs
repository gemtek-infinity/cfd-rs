use super::{CanonicalLink, PriorityQueueEntry, StatusField};

pub(super) fn parse_sections(text: &str, prefix: &str) -> Vec<(String, String)> {
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

pub(super) fn section_content<'a>(sections: &'a [(String, String)], title: &str) -> Result<&'a str, String> {
    sections
        .iter()
        .find(|(section_title, _)| section_title == title)
        .map(|(_, content)| content.as_str())
        .ok_or_else(|| format!("missing required section in STATUS.md: {title}"))
}

pub(super) fn parse_status_fields(section: &str) -> Vec<StatusField> {
    section
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- "))
        .filter_map(|line| line.split_once(':'))
        .map(|(label, value)| StatusField {
            label: label.trim().to_string(),
            value: value.trim().to_string(),
        })
        .collect()
}

pub(super) fn first_paragraph(section: &str) -> String {
    let mut lines = Vec::new();

    for line in section.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !lines.is_empty() {
                break;
            }

            continue;
        }

        if trimmed.ends_with(':') {
            break;
        }

        lines.push(trimmed.to_string());
    }

    lines.join(" ")
}

pub(super) fn extract_list_block(section: &str, label: &str) -> Vec<String> {
    let target = format!("{label}:");
    let mut capture = false;
    let mut items = Vec::new();

    for line in section.lines() {
        let trimmed = line.trim();

        if !capture {
            if trimmed == target {
                capture = true;
            }

            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        if is_block_label(trimmed) || trimmed.starts_with("### ") {
            break;
        }

        if let Some(item) = trimmed.strip_prefix("- ") {
            items.push(item.trim().to_string());
            continue;
        }

        items.push(trimmed.to_string());
    }

    items
}

pub(super) fn first_h3_heading(section: &str) -> Result<String, String> {
    section
        .lines()
        .find_map(|line| line.trim().strip_prefix("### "))
        .map(|value| value.trim().to_string())
        .ok_or_else(|| "missing h3 heading in Active Milestone section".to_string())
}

pub(super) fn extract_inline_backtick_item(section: &str, label_prefix: &str) -> Option<String> {
    let mut capture = false;

    for line in section.lines() {
        let trimmed = line.trim();

        if !capture {
            if trimmed.starts_with(label_prefix) && trimmed.ends_with(':') {
                capture = true;
            }

            continue;
        }

        if let Some(item) = trimmed.strip_prefix("- `") {
            return item.strip_suffix('`').map(|value| value.to_string());
        }

        if is_block_label(trimmed) || trimmed.starts_with("### ") {
            break;
        }
    }

    None
}

pub(super) fn parse_priority_queue(section: &str) -> Result<Vec<PriorityQueueEntry>, String> {
    let mut entries = Vec::new();
    let mut current_rank: Option<u32> = None;
    let mut current_lines = Vec::new();

    for line in section.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if let Some((rank_text, remainder)) = trimmed.split_once('.')
            && let Ok(rank) = rank_text.trim().parse::<u32>()
        {
            if let Some(previous_rank) = current_rank.replace(rank) {
                entries.push(build_priority_entry(previous_rank, &current_lines.join(" "))?);
                current_lines.clear();
            }

            current_lines.push(remainder.trim().to_string());
            continue;
        }

        if current_rank.is_some() {
            current_lines.push(trimmed.to_string());
        }
    }

    if let Some(rank) = current_rank {
        entries.push(build_priority_entry(rank, &current_lines.join(" "))?);
    }

    Ok(entries)
}

pub(super) fn parse_canonical_links(section: &str) -> Vec<CanonicalLink> {
    section
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- "))
        .filter_map(|line| line.split_once(':'))
        .map(|(label, path)| CanonicalLink {
            label: label.trim().to_string(),
            path: path.trim().trim_matches('`').to_string(),
        })
        .collect()
}

fn is_block_label(line: &str) -> bool {
    line.ends_with(':')
        && !line.starts_with("- ")
        && !line.starts_with('*')
        && !line.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

fn split_priority_line(line: &str) -> (&str, &str) {
    if let Some((left, right)) = line.split_once(" — ") {
        return (left, right);
    }

    if let Some((left, right)) = line.split_once(" - ") {
        return (left, right);
    }

    (line, "")
}

fn build_priority_entry(rank: u32, text: &str) -> Result<PriorityQueueEntry, String> {
    let (row_source, summary) = split_priority_line(text.trim());
    let row_ids = extract_row_ids(row_source)?;

    Ok(PriorityQueueEntry {
        rank,
        row_ids,
        summary: summary.to_string(),
    })
}

fn extract_row_ids(text: &str) -> Result<Vec<String>, String> {
    let mut row_ids = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            current.push(ch);
            continue;
        }

        push_row_token(&mut row_ids, &mut current);
    }

    push_row_token(&mut row_ids, &mut current);

    if row_ids.is_empty() {
        return Err(format!("no row ids found in priority line: {text}"));
    }

    Ok(row_ids)
}

fn push_row_token(row_ids: &mut Vec<String>, current: &mut String) {
    if current.len() == 7 {
        let prefix = &current[0..3];
        let digits = &current[4..7];

        if matches!(prefix, "CLI" | "CDC" | "HIS")
            && current.as_bytes().get(3) == Some(&b'-')
            && digits.chars().all(|ch| ch.is_ascii_digit())
        {
            row_ids.push(current.clone());
        }
    }

    current.clear();
}
