use url::Url;

use crate::config::error::{ConfigError, Result};

pub(super) fn validate_hostname(
    hostname: Option<&str>,
    path: Option<&str>,
    rule_index: usize,
    total_rules: usize,
) -> Result<()> {
    let hostname = hostname.unwrap_or_default();
    let path = path.unwrap_or_default();

    if hostname.contains(':') {
        return Err(ConfigError::IngressHostnameContainsPort);
    }
    if hostname.rfind('*').is_some_and(|index| index > 0) {
        return Err(ConfigError::IngressBadWildcard);
    }

    let is_catch_all = (hostname.is_empty() || hostname == "*") && path.is_empty();
    let is_last_rule = rule_index + 1 == total_rules;
    if is_last_rule && !is_catch_all {
        return Err(ConfigError::IngressLastRuleNotCatchAll);
    }
    if !is_last_rule && is_catch_all {
        return Err(ConfigError::IngressCatchAllNotLast {
            index: rule_index + 1,
            hostname: hostname.to_owned(),
        });
    }

    Ok(())
}

pub(super) fn normalized_punycode_hostname(hostname: Option<&str>) -> Result<Option<String>> {
    let Some(hostname) = hostname else {
        return Ok(None);
    };
    if hostname.is_empty() || hostname == "*" || hostname.contains('*') {
        return Ok(None);
    }

    let url = Url::parse(&format!("https://{hostname}"))
        .map_err(|source| ConfigError::invalid_url("hostname", hostname, source))?;
    let Some(punycode) = url.host_str() else {
        return Ok(None);
    };
    if punycode == hostname {
        Ok(None)
    } else {
        Ok(Some(punycode.to_owned()))
    }
}
