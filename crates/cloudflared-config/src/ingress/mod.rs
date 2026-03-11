use crate::error::Result;

mod flag_surface;
mod matching;
mod origin_request;
mod service_parser;
mod types;
mod validation;

pub use self::types::{
    AccessConfig, DurationSpec, IngressFlagRequest, IngressIpRule, IngressMatch, IngressRule, IngressService,
    NormalizedIngress, OriginRequestConfig, RawIngressRule,
};

pub const NO_INGRESS_RULES_FLAGS_MESSAGE: &str = "No ingress rules were defined in provided config (if any) \
                                                  nor from the provided flags, cloudflared will return 503 \
                                                  for all incoming HTTP requests";

const DEFAULT_HTTP_CONNECT_TIMEOUT: &str = "30s";
const DEFAULT_TLS_TIMEOUT: &str = "10s";
const DEFAULT_TCP_KEEP_ALIVE: &str = "30s";
const DEFAULT_KEEP_ALIVE_TIMEOUT: &str = "1m30s";
const DEFAULT_PROXY_ADDRESS: &str = "127.0.0.1";
const DEFAULT_KEEP_ALIVE_CONNECTIONS: u32 = 100;

pub fn default_no_ingress_rule() -> IngressRule {
    IngressRule {
        matcher: IngressMatch::default(),
        service: IngressService::HttpStatus(503),
        origin_request: OriginRequestConfig::default(),
    }
}

pub fn find_matching_rule(rules: &[IngressRule], hostname: &str, path: &str) -> Option<usize> {
    self::matching::find_matching_rule(rules, hostname, path)
}

pub fn parse_ingress_flags(flags: &[String]) -> Result<NormalizedIngress> {
    self::flag_surface::parse_ingress_flags(flags)
}

#[cfg(test)]
mod tests {
    use super::{
        DurationSpec, IngressFlagRequest, IngressIpRule, IngressRule, IngressService, NormalizedIngress,
        OriginRequestConfig, RawIngressRule, default_no_ingress_rule, find_matching_rule,
        parse_ingress_flags,
    };
    use crate::error::ConfigError;

    fn ok<T, E: std::fmt::Display>(result: std::result::Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("unexpected error: {error}"),
        }
    }

    #[test]
    fn service_parser_recognizes_http_services() {
        let service = ok(IngressService::parse("service", "https://localhost:8080"));

        match service {
            IngressService::Http(url) => assert_eq!(url.scheme(), "https"),
            other => panic!("expected HTTP service, found {other:?}"),
        }
    }

    #[test]
    fn service_parser_recognizes_tcp_over_websocket_services() {
        let service = ok(IngressService::parse("service", "tcp://localhost:8080"));

        match service {
            IngressService::TcpOverWebsocket(url) => assert_eq!(url.scheme(), "tcp"),
            other => panic!("expected TCP-over-websocket service, found {other:?}"),
        }
    }

    #[test]
    fn raw_rule_without_hostname_or_path_is_catch_all() {
        let rule = ok(IngressRule::from_raw(
            RawIngressRule {
                service: Some("https://localhost:8080".to_owned()),
                ..RawIngressRule::default()
            },
            &OriginRequestConfig::default(),
            0,
            1,
        ));

        assert!(rule.is_catch_all());
    }

    #[test]
    fn wildcard_not_at_start_is_rejected() {
        let error = IngressRule::from_raw(
            RawIngressRule {
                hostname: Some("test.*.example.com".to_owned()),
                service: Some("https://localhost:8080".to_owned()),
                ..RawIngressRule::default()
            },
            &OriginRequestConfig::default(),
            0,
            1,
        )
        .expect_err("wildcard should be rejected");

        assert!(matches!(error, ConfigError::IngressBadWildcard));
    }

    #[test]
    fn no_ingress_default_rule_is_http_503() {
        assert_eq!(default_no_ingress_rule().service, IngressService::HttpStatus(503));
    }

    #[test]
    fn unicode_hostname_captures_punycode() {
        let rule = ok(IngressRule::from_raw(
            RawIngressRule {
                hostname: Some("môô.cloudflare.com".to_owned()),
                service: Some("https://localhost:8080".to_owned()),
                ..RawIngressRule::default()
            },
            &OriginRequestConfig::default(),
            0,
            2,
        ));

        assert_eq!(
            rule.matcher.punycode_hostname.as_deref(),
            Some("xn--m-xgaa.cloudflare.com")
        );
    }

    #[test]
    fn matching_prefers_first_matching_rule_and_strips_port() {
        let rules = vec![
            ok(IngressRule::from_raw(
                RawIngressRule {
                    hostname: Some("tunnel-a.example.com".to_owned()),
                    service: Some("https://localhost:8080".to_owned()),
                    ..RawIngressRule::default()
                },
                &OriginRequestConfig::default(),
                0,
                3,
            )),
            ok(IngressRule::from_raw(
                RawIngressRule {
                    hostname: Some("tunnel-b.example.com".to_owned()),
                    path: Some("/health".to_owned()),
                    service: Some("https://localhost:8081".to_owned()),
                    ..RawIngressRule::default()
                },
                &OriginRequestConfig::default(),
                1,
                3,
            )),
            ok(IngressRule::from_raw(
                RawIngressRule {
                    service: Some("http_status:404".to_owned()),
                    ..RawIngressRule::default()
                },
                &OriginRequestConfig::default(),
                2,
                3,
            )),
        ];

        assert_eq!(
            find_matching_rule(&rules, "tunnel-a.example.com:443", "/"),
            Some(0)
        );
        assert_eq!(
            find_matching_rule(&rules, "tunnel-b.example.com", "/health"),
            Some(1)
        );
        assert_eq!(
            find_matching_rule(&rules, "tunnel-b.example.com", "/index.html"),
            Some(2)
        );
        assert_eq!(find_matching_rule(&rules, "unknown.example.com", "/"), Some(2));
    }

    #[test]
    fn unicode_rule_matches_punycode_hostname() {
        let rules = vec![
            ok(IngressRule::from_raw(
                RawIngressRule {
                    hostname: Some("môô.cloudflare.com".to_owned()),
                    service: Some("https://localhost:8080".to_owned()),
                    ..RawIngressRule::default()
                },
                &OriginRequestConfig::default(),
                0,
                2,
            )),
            ok(IngressRule::from_raw(
                RawIngressRule {
                    service: Some("http_status:404".to_owned()),
                    ..RawIngressRule::default()
                },
                &OriginRequestConfig::default(),
                1,
                2,
            )),
        ];

        assert_eq!(
            find_matching_rule(&rules, "xn--m-xgaa.cloudflare.com", "/"),
            Some(0)
        );
    }

    #[test]
    fn flag_request_parses_flags() {
        let request = IngressFlagRequest::from_flags(&[
            "--url=http://localhost:8080".to_owned(),
            "--hello-world=false".to_owned(),
        ]);

        assert_eq!(request.url.as_deref(), Some("http://localhost:8080"));
        assert!(!request.hello_world);
    }

    #[test]
    fn flag_ingress_hello_world_normalizes() {
        let ingress = ok(parse_ingress_flags(&["--hello-world=true".to_owned()]));

        assert_eq!(ingress.rules.len(), 1);
        assert_eq!(ingress.rules[0].service, IngressService::HelloWorld);
        assert_eq!(
            ingress
                .defaults
                .connect_timeout
                .as_ref()
                .map(|value| value.0.as_str()),
            Some("30s")
        );
    }

    #[test]
    fn flag_ingress_bastion_sets_bastion_mode() {
        let ingress = ok(NormalizedIngress::from_flag_request(&IngressFlagRequest {
            bastion: true,
            ..IngressFlagRequest::default()
        }));

        assert_eq!(ingress.rules[0].service, IngressService::Bastion);
        assert_eq!(ingress.defaults.bastion_mode, Some(true));
    }

    #[test]
    fn flag_ingress_materializes_go_default_representation() {
        let ingress = ok(parse_ingress_flags(&["--hello-world".to_owned()]));

        assert_eq!(
            ingress.defaults.keep_alive_timeout,
            Some(DurationSpec("1m30s".to_owned()))
        );
        assert_eq!(ingress.defaults.proxy_port, Some(0));
        assert_eq!(ingress.defaults.bastion_mode, Some(false));
        assert_eq!(ingress.rules[0].origin_request, ingress.defaults);
    }

    #[test]
    fn inherited_origin_request_defaults_materialize_and_merge() {
        let inherited = OriginRequestConfig::materialized_config_defaults(&OriginRequestConfig {
            ip_rules: vec![IngressIpRule {
                prefix: Some("10.0.0.0/8".to_owned()),
                ports: vec![80, 8080],
                allow: false,
            }],
            ..OriginRequestConfig::default()
        });
        let rule = ok(IngressRule::from_raw(
            RawIngressRule {
                service: Some("https://localhost:8080".to_owned()),
                ..RawIngressRule::default()
            },
            &inherited,
            0,
            1,
        ));

        assert_eq!(
            rule.origin_request.connect_timeout,
            Some(DurationSpec("30s".to_owned()))
        );
        assert_eq!(
            rule.origin_request.keep_alive_timeout,
            Some(DurationSpec("1m30s".to_owned()))
        );
        assert_eq!(rule.origin_request.proxy_port, Some(0));
        assert_eq!(rule.origin_request.bastion_mode, Some(false));
        assert_eq!(rule.origin_request.ip_rules, inherited.ip_rules);
    }

    #[test]
    fn flag_ingress_without_origin_is_an_error() {
        let error = parse_ingress_flags(&[]).expect_err("missing origin should fail");

        assert!(matches!(error, ConfigError::NoIngressRulesFlags));
        assert_eq!(error.category(), "no-ingress-rules-flags");
    }
}
