use cfdrs_cli::{CliOutput, GlobalFlags};
use cfdrs_his::discovery::find_default_config_path;
use cfdrs_shared::{
    ConfigSource, DiscoveryRequest, IngressRule, IngressService, OriginRequestConfig, RawConfig,
    find_matching_rule,
};
use reqwest::blocking::Client;
use url::Url;

const MISSING_METRICS_FLAG_MSG: &str = "--metrics has to be provided";
const NO_CONFIG_FILE_MSG: &str = "No configuration file was found. Please create one, or use the --config \
                                  flag to specify its filepath. You can use the help command to learn more \
                                  about configuration files";
const NO_INGRESS_RULES_MSG: &str = "Validation failed: The config file doesn't contain any ingress rules";
const URL_INCOMPATIBLE_WITH_INGRESS_MSG: &str =
    "You can't set the --url flag (or $TUNNEL_URL) when using multiple-origin ingress rules";

#[derive(Debug)]
struct StrictIngressConfig {
    rules: Vec<IngressRule>,
    warning_keys: Vec<String>,
}

struct IngressConfigInput {
    raw: RawConfig,
    source_display: String,
}

pub fn execute_tunnel_ready(flags: &GlobalFlags) -> CliOutput {
    let Some(metrics_addr) = flags.metrics.as_deref() else {
        return CliOutput::failure(String::new(), MISSING_METRICS_FLAG_MSG.to_owned(), 1);
    };

    let client = Client::new();

    let request_url = format!("http://{metrics_addr}/ready");
    let response = match client.get(&request_url).send() {
        Ok(response) => response,
        Err(error) => return CliOutput::failure(String::new(), error.to_string(), 1),
    };

    if response.status().as_u16() == 200 {
        return CliOutput::success(String::new());
    }

    let status = response.status().as_u16();
    let body = match response.text() {
        Ok(body) => body,
        Err(error) => return CliOutput::failure(String::new(), error.to_string(), 1),
    };

    CliOutput::failure(
        String::new(),
        format!("http://{metrics_addr}/ready endpoint returned status code {status}\n{body}"),
        1,
    )
}

pub fn execute_ingress_validate(flags: &GlobalFlags) -> CliOutput {
    let input = match load_ingress_config_for_validate(flags) {
        Ok(result) => result,
        Err(error) => return CliOutput::failure(String::new(), error, 1),
    };

    let mut stdout = format!("Validating rules from {}\n", input.source_display);

    let strict = match strict_parse_ingress(input.raw) {
        Ok(strict) => strict,
        Err(error) => return CliOutput::failure(stdout, error, 1),
    };

    if flags.url.is_some() {
        return CliOutput::failure(stdout, URL_INCOMPATIBLE_WITH_INGRESS_MSG.to_owned(), 1);
    }

    if strict.warning_keys.is_empty() {
        stdout.push_str("OK\n");
        return CliOutput::success(stdout);
    }

    stdout.push_str("Warning: unused keys detected in your config file. Here is a list of unused keys:\n");
    stdout.push_str(&strict.warning_keys.join("\n"));
    if !stdout.ends_with('\n') {
        stdout.push('\n');
    }

    CliOutput::success(stdout)
}

pub fn execute_ingress_rule(flags: &GlobalFlags) -> CliOutput {
    let request_arg = match flags.rest_args.first() {
        Some(arg) => arg,
        None => {
            return CliOutput::failure(
                String::new(),
                "cloudflared tunnel rule expects a single argument, the URL to test".to_owned(),
                1,
            );
        }
    };

    let request_url = match parse_ingress_rule_url(request_arg) {
        Ok(url) => url,
        Err(error) => return CliOutput::failure(String::new(), error, 1),
    };

    let input = match load_ingress_config_for_rule(flags) {
        Ok(Some(result)) => result,
        Ok(None) => {
            return CliOutput::failure(
                "Using rules from \n".to_owned(),
                NO_INGRESS_RULES_MSG.to_owned(),
                1,
            );
        }
        Err(error) => return CliOutput::failure(String::new(), error, 1),
    };

    let mut stdout = format!("Using rules from {}\n", input.source_display);
    let strict = match strict_parse_ingress(input.raw) {
        Ok(strict) => strict,
        Err(error) => return CliOutput::failure(stdout, error, 1),
    };

    let Some(rule_index) = find_matching_rule(
        &strict.rules,
        request_url.host_str().unwrap_or_default(),
        request_url.path(),
    ) else {
        return CliOutput::failure(
            stdout,
            "Validation failed: no ingress rules were loaded".to_owned(),
            1,
        );
    };

    stdout.push_str(&format!("Matched rule #{rule_index}\n"));
    stdout.push_str(&format_ingress_rule(&strict.rules[rule_index]));
    stdout.push('\n');

    CliOutput::success(stdout)
}

fn load_ingress_config_for_validate(flags: &GlobalFlags) -> Result<IngressConfigInput, String> {
    if let Some(json) = flags.ingress_json.as_deref() {
        let raw = serde_json::from_str(json).map_err(|error| error.to_string())?;
        return Ok(IngressConfigInput {
            raw,
            source_display: "cmdline flag --json".to_owned(),
        });
    }

    match load_ingress_config_from_path(flags)? {
        Some(input) => Ok(input),
        None => Err(NO_CONFIG_FILE_MSG.to_owned()),
    }
}

fn load_ingress_config_for_rule(flags: &GlobalFlags) -> Result<Option<IngressConfigInput>, String> {
    load_ingress_config_from_path(flags)
}

fn load_ingress_config_from_path(flags: &GlobalFlags) -> Result<Option<IngressConfigInput>, String> {
    let request = DiscoveryRequest {
        explicit_config: flags.config_path.clone(),
        ..DiscoveryRequest::default()
    };

    let path = if let Some(path) = flags.config_path.clone() {
        path
    } else {
        match find_default_config_path(&request) {
            Some(path) => path,
            None => return Ok(None),
        }
    };

    let source = if flags.config_path.is_some() {
        ConfigSource::ExplicitPath(path.clone())
    } else {
        ConfigSource::DiscoveredPath(path.clone())
    };

    let raw = load_raw_config(&path).map_err(|error| error.to_string())?;
    Ok(Some(IngressConfigInput {
        raw,
        source_display: format_config_source(&source),
    }))
}

fn load_raw_config(path: &std::path::Path) -> Result<RawConfig, cfdrs_shared::ConfigError> {
    RawConfig::from_yaml_path(path)
}

fn format_config_source(source: &ConfigSource) -> String {
    match source {
        ConfigSource::ExplicitPath(path)
        | ConfigSource::DiscoveredPath(path)
        | ConfigSource::AutoCreatedPath(path) => path.display().to_string(),
    }
}

fn strict_parse_ingress(raw: RawConfig) -> Result<StrictIngressConfig, String> {
    let warning_keys = raw.unknown_top_level_keys();
    if raw.ingress.is_empty() {
        return Err(NO_INGRESS_RULES_MSG.to_owned());
    }

    let inherited_origin_request = OriginRequestConfig::materialized_config_defaults(&raw.origin_request);
    let total_rules = raw.ingress.len();
    let rules = raw
        .ingress
        .into_iter()
        .enumerate()
        .map(|(rule_index, rule)| {
            IngressRule::from_raw(rule, &inherited_origin_request, rule_index, total_rules)
                .map_err(|error| format!("Validation failed: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(StrictIngressConfig { rules, warning_keys })
}

fn parse_ingress_rule_url(input: &str) -> Result<Url, String> {
    match Url::parse(input) {
        Ok(url) => {
            if url.host_str().is_none() && url.scheme().is_empty() {
                Err(format!(
                    "{input} doesn't have a hostname, consider adding a scheme"
                ))
            } else {
                Ok(url)
            }
        }
        Err(_) if !input.contains("://") => Err(format!(
            "{input} doesn't have a hostname, consider adding a scheme"
        )),
        Err(_) => Err(format!("{input} is not a valid URL")),
    }
}

fn format_ingress_rule(rule: &IngressRule) -> String {
    let mut output = String::new();

    if let Some(hostname) = rule.matcher.hostname.as_deref()
        && !hostname.is_empty()
    {
        output.push_str("\thostname: ");
        output.push_str(hostname);
        output.push('\n');
    }

    if let Some(path) = rule.matcher.path.as_deref()
        && !path.is_empty()
    {
        output.push_str("\tpath: ");
        output.push_str(path);
        output.push('\n');
    }

    output.push_str("\tservice: ");
    output.push_str(&format_ingress_service(&rule.service));
    output
}

fn format_ingress_service(service: &IngressService) -> String {
    match service {
        IngressService::Http(url) | IngressService::TcpOverWebsocket(url) => display_origin_url(url),
        IngressService::UnixSocket(path) => format!("unix:{}", path.display()),
        IngressService::UnixSocketTls(path) => format!("unix+tls:{}", path.display()),
        IngressService::HttpStatus(code) => format!("http_status:{code}"),
        IngressService::HelloWorld => "hello_world".to_owned(),
        IngressService::Bastion => "bastion".to_owned(),
        IngressService::SocksProxy => "socks-proxy".to_owned(),
        IngressService::NamedToken(token) => token.clone(),
    }
}

fn display_origin_url(url: &Url) -> String {
    let rendered = url.to_string();
    if url.path() == "/" && url.query().is_none() && url.fragment().is_none() {
        rendered.trim_end_matches('/').to_owned()
    } else {
        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cfdrs_shared::RawIngressRule;

    #[test]
    fn ingress_rule_without_scheme_suggests_scheme() {
        let error = parse_ingress_rule_url("example.com").expect_err("missing scheme should fail");
        assert_eq!(
            error,
            "example.com doesn't have a hostname, consider adding a scheme"
        );
    }

    #[test]
    fn ingress_rule_with_invalid_url_reports_invalid() {
        let error = parse_ingress_rule_url("http://[invalid").expect_err("invalid url should fail");
        assert_eq!(error, "http://[invalid is not a valid URL");
    }

    #[test]
    fn ingress_rule_formatter_matches_go_multiline_shape() {
        let rule = IngressRule {
            matcher: cfdrs_shared::IngressMatch {
                hostname: Some("app.example.com".to_owned()),
                punycode_hostname: None,
                path: Some("/health".to_owned()),
            },
            service: IngressService::Http(Url::parse("https://localhost:8080").expect("url")),
            origin_request: cfdrs_shared::OriginRequestConfig::default(),
        };

        assert_eq!(
            format_ingress_rule(&rule),
            "\thostname: app.example.com\n\tpath: /health\n\tservice: https://localhost:8080"
        );
    }

    #[test]
    fn strict_parse_ingress_rejects_missing_rules() {
        let error = strict_parse_ingress(RawConfig::default()).expect_err("missing ingress should fail");
        assert_eq!(error, NO_INGRESS_RULES_MSG);
    }

    #[test]
    fn strict_parse_ingress_carries_unknown_keys_without_defaulting_rules() {
        let strict = strict_parse_ingress(RawConfig {
            ingress: vec![RawIngressRule {
                service: Some("http_status:404".to_owned()),
                ..RawIngressRule::default()
            }],
            additional_fields: [("extraKey".to_owned(), serde_yaml::Value::Bool(true))]
                .into_iter()
                .collect(),
            ..RawConfig::default()
        })
        .expect("strict ingress should parse");

        assert_eq!(strict.warning_keys, vec!["extraKey".to_owned()]);
        assert_eq!(strict.rules.len(), 1);
    }
}
