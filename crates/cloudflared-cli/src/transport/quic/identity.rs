use std::fmt;
use std::path::PathBuf;

use cloudflared_config::{OriginCertLocator, OriginCertToken, TunnelCredentialsFile};
use uuid::Uuid;

use crate::runtime::RuntimeConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IdentitySource {
    CredentialsFile,
    OriginCert,
}

impl fmt::Display for IdentitySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::CredentialsFile => "credentials-file",
            Self::OriginCert => "origin-cert",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone)]
pub(super) struct TransportIdentity {
    pub(super) tunnel_id: Uuid,
    pub(super) identity_source: IdentitySource,
    pub(super) endpoint_hint: Option<String>,
    pub(super) resumption: ResumptionShape,
}

impl TransportIdentity {
    pub(super) fn from_runtime_config(config: &RuntimeConfig) -> Result<Self, String> {
        let normalized = config.normalized();
        let tunnel = normalized
            .tunnel
            .as_ref()
            .ok_or_else(|| String::from("quic tunnel core requires a configured tunnel reference"))?;
        let tunnel_id = tunnel.uuid.ok_or_else(|| {
            String::from("quic tunnel core requires the tunnel reference to be a UUID-backed named tunnel")
        })?;

        let credentials = &normalized.credentials;
        let (identity_source, endpoint_hint) = if let Some(path) = credentials.credentials_file.as_ref() {
            let tunnel_credentials = TunnelCredentialsFile::from_json_path(path).map_err(|error| {
                format!(
                    "failed to load tunnel credentials file {}: {error}",
                    path.display()
                )
            })?;

            if tunnel_credentials.tunnel_id != tunnel_id {
                return Err(format!(
                    "tunnel UUID {} does not match credentials file tunnel ID {}",
                    tunnel_id, tunnel_credentials.tunnel_id
                ));
            }

            (
                IdentitySource::CredentialsFile,
                tunnel_credentials
                    .endpoint
                    .map(|value| value.to_ascii_lowercase()),
            )
        } else if let Some(path) = origin_cert_path(credentials) {
            let origin_cert = OriginCertToken::from_pem_path(&path)
                .map_err(|error| format!("failed to read origin cert {}: {error}", path.display()))?;
            (IdentitySource::OriginCert, origin_cert.endpoint)
        } else {
            return Err(String::from(
                "quic tunnel core requires credentials-file or origincert to resolve edge interaction \
                 semantics",
            ));
        };

        Ok(Self {
            tunnel_id,
            identity_source,
            endpoint_hint,
            resumption: ResumptionShape::EarlyDataEnabled,
        })
    }
}

#[derive(Debug, Clone)]
pub(super) enum ResumptionShape {
    EarlyDataEnabled,
}

impl ResumptionShape {
    pub(super) fn policy_label(&self) -> &'static str {
        match self {
            Self::EarlyDataEnabled => "quiche early data enabled when session tickets are available",
        }
    }

    pub(super) fn shape_label(&self) -> &'static str {
        match self {
            Self::EarlyDataEnabled => "0-rtt-preserving",
        }
    }
}

fn origin_cert_path(credentials: &cloudflared_config::CredentialSurface) -> Option<PathBuf> {
    match credentials.origin_cert.as_ref() {
        Some(OriginCertLocator::ConfiguredPath(path)) | Some(OriginCertLocator::DefaultSearchPath(path)) => {
            Some(path.clone())
        }
        None => None,
    }
}
