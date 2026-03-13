use std::path::Path;

use super::types::{
    CredentialKind, CredentialReportPayload, CredentialSurfacePayload, DiscoveryActionKind,
    DiscoveryReportPayload, IngressReportPayload, IngressRulePayload, IngressServiceKind,
    IngressServicePayload, NormalizedConfigPayload, OriginCertLocatorKind, OriginCertLocatorPayload,
    SourceKind, TunnelReferencePayload, WarningKind, WarningPayload,
};
use crate::config::config_source::ConfigSource;
use crate::config::credentials::{CredentialSurface, OriginCertLocator, OriginCertToken, TunnelReference};
use crate::config::discovery::{DiscoveryAction, DiscoveryOutcome};
use crate::config::ingress::{IngressRule, IngressService, NormalizedIngress};
use crate::config::normalized::{NormalizationWarning, NormalizedConfig};

impl DiscoveryReportPayload {
    pub fn from_outcome(outcome: &DiscoveryOutcome, sandbox_root: &Path) -> Self {
        Self {
            action: match outcome.action {
                DiscoveryAction::UseExisting => DiscoveryActionKind::UseExisting,
                DiscoveryAction::CreateDefaultConfig => DiscoveryActionKind::CreateDefaultConfig,
            },
            source_kind: config_source_to_kind(&outcome.source),
            resolved_path: display_path(&outcome.path, sandbox_root),
            created_paths: outcome
                .created_paths
                .iter()
                .map(|path| display_path(path, sandbox_root))
                .collect(),
            written_config: outcome.written_config.clone(),
        }
    }
}

impl NormalizedConfigPayload {
    pub fn from_normalized(source_path: &Path, normalized: &NormalizedConfig) -> Self {
        let warnings = if normalized.warnings.is_empty() {
            None
        } else {
            Some(
                normalized
                    .warnings
                    .iter()
                    .map(WarningPayload::from_warning)
                    .collect(),
            )
        };

        Self {
            source_kind: config_source_to_kind(&normalized.source),
            source_path: source_path.display().to_string(),
            tunnel: normalized
                .tunnel
                .as_ref()
                .map(TunnelReferencePayload::from_tunnel),
            credentials: CredentialSurfacePayload::from_credentials(&normalized.credentials),
            ingress: normalized
                .ingress
                .iter()
                .map(IngressRulePayload::from_rule)
                .collect(),
            origin_request: normalized.origin_request.clone(),
            warp_routing: normalized.warp_routing.clone(),
            log_directory: normalized
                .log_directory
                .as_ref()
                .map(|path| path.display().to_string()),
            warnings,
        }
    }
}

impl TunnelReferencePayload {
    pub(super) fn from_tunnel(tunnel: &TunnelReference) -> Self {
        Self {
            raw: tunnel.raw.clone(),
            uuid: tunnel.uuid.map(|value| value.to_string()),
        }
    }
}

impl CredentialSurfacePayload {
    pub(super) fn from_credentials(credentials: &CredentialSurface) -> Self {
        Self {
            credentials_file: credentials
                .credentials_file
                .as_ref()
                .map(|path| path.display().to_string()),
            origin_cert: credentials
                .origin_cert
                .as_ref()
                .map(OriginCertLocatorPayload::from_locator),
            tunnel: credentials
                .tunnel
                .as_ref()
                .map(TunnelReferencePayload::from_tunnel),
        }
    }
}

impl OriginCertLocatorPayload {
    pub(super) fn from_locator(locator: &OriginCertLocator) -> Self {
        match locator {
            OriginCertLocator::ConfiguredPath(path) => Self {
                kind: OriginCertLocatorKind::ConfiguredPath,
                path: path.display().to_string(),
            },
            OriginCertLocator::DefaultSearchPath(path) => Self {
                kind: OriginCertLocatorKind::DefaultSearchPath,
                path: path.display().to_string(),
            },
        }
    }
}

impl IngressRulePayload {
    pub(super) fn from_rule(rule: &IngressRule) -> Self {
        Self {
            hostname: rule.matcher.hostname.clone(),
            punycode_hostname: rule.matcher.punycode_hostname.clone(),
            path: rule.matcher.path.clone(),
            service: IngressServicePayload::from_service(&rule.service),
            origin_request: rule.origin_request.clone(),
        }
    }
}

impl IngressServicePayload {
    pub(super) fn from_service(service: &IngressService) -> Self {
        match service {
            IngressService::Http(uri) => Self {
                kind: IngressServiceKind::Http,
                uri: Some(display_origin_url(uri)),
                path: None,
                name: None,
                status_code: None,
            },
            IngressService::TcpOverWebsocket(uri) => Self {
                kind: IngressServiceKind::TcpOverWebsocket,
                uri: Some(display_origin_url(uri)),
                path: None,
                name: None,
                status_code: None,
            },
            IngressService::UnixSocket(path) => Self {
                kind: IngressServiceKind::UnixSocket,
                uri: None,
                path: Some(path.display().to_string()),
                name: None,
                status_code: None,
            },
            IngressService::UnixSocketTls(path) => Self {
                kind: IngressServiceKind::UnixSocketTls,
                uri: None,
                path: Some(path.display().to_string()),
                name: None,
                status_code: None,
            },
            IngressService::HttpStatus(status_code) => Self {
                kind: IngressServiceKind::HttpStatus,
                uri: None,
                path: None,
                name: None,
                status_code: Some(*status_code),
            },
            IngressService::HelloWorld => Self {
                kind: IngressServiceKind::HelloWorld,
                uri: None,
                path: None,
                name: None,
                status_code: None,
            },
            IngressService::Bastion => Self {
                kind: IngressServiceKind::Bastion,
                uri: None,
                path: None,
                name: None,
                status_code: None,
            },
            IngressService::SocksProxy => Self {
                kind: IngressServiceKind::SocksProxy,
                uri: None,
                path: None,
                name: None,
                status_code: None,
            },
            IngressService::NamedToken(name) => Self {
                kind: IngressServiceKind::NamedToken,
                uri: None,
                path: None,
                name: Some(name.clone()),
                status_code: None,
            },
        }
    }
}

impl WarningPayload {
    pub(super) fn from_warning(warning: &NormalizationWarning) -> Self {
        match warning {
            NormalizationWarning::UnknownTopLevelKeys(keys) => Self {
                kind: WarningKind::UnknownTopLevelKeys,
                keys: keys.clone(),
            },
        }
    }
}

impl CredentialReportPayload {
    pub(super) fn from_origin_cert(source_path: &str, token: &OriginCertToken) -> Self {
        Self {
            kind: CredentialKind::OriginCertPem,
            source_path: source_path.to_owned(),
            zone_id: token.zone_id.clone(),
            account_id: token.account_id.clone(),
            api_token: token.api_token.clone(),
            endpoint: token.endpoint.clone(),
            is_fed_endpoint: token.is_fed_endpoint(),
        }
    }
}

impl IngressReportPayload {
    pub(super) fn from_ingress(source_kind: SourceKind, normalized: &NormalizedIngress) -> Self {
        Self {
            source_kind,
            rule_count: normalized.rules.len(),
            catch_all_rule_index: normalized.rules.len().saturating_sub(1),
            defaults: normalized.defaults.clone(),
            rules: normalized
                .rules
                .iter()
                .map(IngressRulePayload::from_rule)
                .collect(),
        }
    }
}

fn display_path(path: &Path, sandbox_root: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(sandbox_root) {
        format!("/{}", relative.display())
    } else {
        path.display().to_string()
    }
}

fn display_origin_url(url: &url::Url) -> String {
    let rendered = url.to_string();
    if url.path() == "/" && url.query().is_none() && url.fragment().is_none() {
        rendered.trim_end_matches('/').to_owned()
    } else {
        rendered
    }
}

fn config_source_to_kind(source: &ConfigSource) -> SourceKind {
    match source {
        ConfigSource::ExplicitPath(_) => SourceKind::ExplicitPath,
        ConfigSource::DiscoveredPath(_) => SourceKind::DiscoveredPath,
        ConfigSource::AutoCreatedPath(_) => SourceKind::AutoCreatedPath,
    }
}
