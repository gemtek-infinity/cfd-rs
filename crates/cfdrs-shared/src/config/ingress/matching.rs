use super::IngressRule;

pub(super) fn find_matching_rule(rules: &[IngressRule], hostname: &str, path: &str) -> Option<usize> {
    if rules.is_empty() {
        return None;
    }

    for (index, rule) in rules.iter().enumerate() {
        if matches_rule(rule, hostname, path) {
            return Some(index);
        }
    }

    Some(rules.len() - 1)
}

pub(super) fn matches_rule(rule: &IngressRule, hostname: &str, path: &str) -> bool {
    let hostname = strip_port(hostname);
    let host_match = match rule.matcher.hostname.as_deref() {
        None | Some("") | Some("*") => true,
        Some(rule_host) => match_host(rule_host, hostname),
    };
    let punycode_match = rule
        .matcher
        .punycode_hostname
        .as_deref()
        .is_some_and(|rule_host| match_host(rule_host, hostname));
    let path_match = rule
        .matcher
        .path
        .as_deref()
        .is_none_or(|pattern| match_path(pattern, path));

    (host_match || punycode_match) && path_match
}

fn match_host(rule_host: &str, req_host: &str) -> bool {
    if rule_host == req_host {
        return true;
    }

    if let Some(suffix) = rule_host.strip_prefix("*.") {
        let suffix = format!(".{suffix}");
        return req_host.ends_with(&suffix);
    }

    false
}

fn match_path(pattern: &str, path: &str) -> bool {
    path.contains(pattern)
}

fn strip_port(hostname: &str) -> &str {
    if hostname.starts_with('[') {
        return hostname;
    }

    if let Some((host, port)) = hostname.rsplit_once(':')
        && !host.contains(':')
        && !host.is_empty()
        && !port.is_empty()
        && port.chars().all(|ch| ch.is_ascii_digit())
    {
        return host;
    }

    hostname
}
