use super::FixtureEntry;

pub(super) fn parse_fixture_entries(contents: &str) -> Vec<FixtureEntry> {
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

    entries
}

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
