use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ConfigError, Result};

pub const DEFAULT_ORIGIN_CERT_FILE: &str = "cert.pem";

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TunnelReference {
    pub raw: String,
    pub uuid: Option<Uuid>,
}

impl TunnelReference {
    pub fn from_raw(raw: String) -> Self {
        let uuid = Uuid::parse_str(&raw).ok();
        Self { raw, uuid }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum OriginCertLocator {
    ConfiguredPath(PathBuf),
    DefaultSearchPath(PathBuf),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct CredentialSurface {
    pub credentials_file: Option<PathBuf>,
    pub origin_cert: Option<OriginCertLocator>,
    pub tunnel: Option<TunnelReference>,
}

impl CredentialSurface {
    pub fn configured(
        credentials_file: Option<PathBuf>,
        origin_cert: Option<PathBuf>,
        tunnel: Option<TunnelReference>,
    ) -> Self {
        Self {
            credentials_file,
            origin_cert: origin_cert.map(OriginCertLocator::ConfiguredPath),
            tunnel,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TunnelCredentialsFile {
    #[serde(rename = "AccountTag")]
    pub account_tag: String,
    #[serde(rename = "TunnelSecret")]
    pub tunnel_secret: String,
    #[serde(rename = "TunnelID")]
    pub tunnel_id: Uuid,
    #[serde(rename = "Endpoint", default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

impl TunnelCredentialsFile {
    pub fn from_json_str(contents: &str) -> Result<Self> {
        serde_json::from_str(contents).map_err(|source| ConfigError::json_parse("tunnel credentials", source))
    }

    pub fn from_json_path(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path).map_err(|source| ConfigError::read(path, source))?;
        Self::from_json_str(&contents)
    }

    pub fn to_pretty_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|source| ConfigError::json_serialize("tunnel credentials", source))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct OriginCertToken {
    #[serde(rename = "zoneID")]
    pub zone_id: String,
    #[serde(rename = "accountID")]
    pub account_id: String,
    #[serde(rename = "apiToken")]
    pub api_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

impl OriginCertToken {
    pub fn from_json_str(contents: &str) -> Result<Self> {
        let mut token: Self = serde_json::from_str(contents)
            .map_err(|source| ConfigError::json_parse("origin cert token", source))?;
        token.endpoint = token.endpoint.map(|value| value.to_ascii_lowercase());
        Ok(token)
    }

    pub fn from_json_path(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path).map_err(|source| ConfigError::read(path, source))?;
        Self::from_json_str(&contents)
    }

    pub fn from_pem_blocks(_blocks: &[u8]) -> Result<Self> {
        Err(ConfigError::deferred("origin cert PEM decoding"))
    }
}

#[cfg(test)]
mod tests {
    use super::{OriginCertToken, TunnelCredentialsFile};

    fn ok<T, E: std::fmt::Display>(result: std::result::Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("unexpected error: {error}"),
        }
    }

    #[test]
    fn tunnel_credentials_json_round_trips() {
        let creds = ok(TunnelCredentialsFile::from_json_str(
            r#"{"AccountTag":"account","TunnelSecret":"secret","TunnelID":"11111111-1111-1111-1111-111111111111"}"#,
        ));
        let serialized = ok(creds.to_pretty_json());

        assert!(serialized.contains("AccountTag"));
        assert!(serialized.contains("11111111-1111-1111-1111-111111111111"));
    }

    #[test]
    fn origin_cert_json_normalizes_endpoint_case() {
        let token = ok(OriginCertToken::from_json_str(
            r#"{"zoneID":"zone","accountID":"account","apiToken":"token","endpoint":"FED"}"#,
        ));

        assert_eq!(token.endpoint.as_deref(), Some("fed"));
    }
}
