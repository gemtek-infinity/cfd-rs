use std::fs;
use std::path::{Path, PathBuf};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use pem::{Pem, encode, parse_many};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ConfigError, Result};

pub const DEFAULT_ORIGIN_CERT_FILE: &str = "cert.pem";
pub const FED_ENDPOINT: &str = "fed";

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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TunnelSecret(Vec<u8>);

impl TunnelSecret {
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Serialize for TunnelSecret {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&BASE64_STANDARD.encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for TunnelSecret {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        BASE64_STANDARD
            .decode(encoded.as_bytes())
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TunnelCredentialsFile {
    #[serde(rename = "AccountTag")]
    pub account_tag: String,
    #[serde(rename = "TunnelSecret")]
    pub tunnel_secret: TunnelSecret,
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct OriginCertUser {
    pub cert: OriginCertToken,
    pub cert_path: PathBuf,
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

    pub fn from_pem_blocks(blocks: &[u8]) -> Result<Self> {
        if blocks.is_empty() {
            return Err(ConfigError::OriginCertEmpty);
        }

        let mut token = None;
        for block in origin_cert_pem::parse_blocks(blocks)? {
            Self::handle_pem_block(&mut token, &block)?;
        }

        Self::validate_parsed_token(token)
    }

    pub fn from_pem_path(path: &Path) -> Result<Self> {
        let contents = fs::read(path).map_err(|source| ConfigError::read(path, source))?;
        Self::from_pem_blocks(&contents)
    }

    pub fn encode_pem(&self) -> Result<Vec<u8>> {
        let json = serde_json::to_vec(self)
            .map_err(|source| ConfigError::json_serialize("origin cert token", source))?;

        Ok(origin_cert_pem::encode_token(json).into_bytes())
    }

    pub fn is_fed_endpoint(&self) -> bool {
        self.endpoint.as_deref() == Some(FED_ENDPOINT)
    }

    fn from_json_bytes(contents: &[u8]) -> Result<Self> {
        let mut token: Self = serde_json::from_slice(contents)
            .map_err(|source| ConfigError::json_parse("origin cert token", source))?;
        token.endpoint = token.endpoint.map(|value| value.to_ascii_lowercase());
        Ok(token)
    }

    fn handle_pem_block(token: &mut Option<Self>, block: &origin_cert_pem::OriginCertPemBlock) -> Result<()> {
        match block.block_type() {
            "PRIVATE KEY" | "CERTIFICATE" => Ok(()),
            "ARGO TUNNEL TOKEN" => Self::decode_token_block(token, block.contents()),
            other => Err(ConfigError::origin_cert_unknown_block(other)),
        }
    }

    fn decode_token_block(token: &mut Option<Self>, contents: &[u8]) -> Result<()> {
        if token_contains_credentials(token.as_ref()) {
            return Err(ConfigError::OriginCertMultipleTokens);
        }

        if let Ok(decoded) = Self::from_json_bytes(contents) {
            *token = Some(decoded);
        }

        Ok(())
    }

    fn validate_parsed_token(token: Option<Self>) -> Result<Self> {
        let token = token.ok_or(ConfigError::OriginCertMissingToken)?;
        if token.zone_id.is_empty() || token.api_token.is_empty() {
            return Err(ConfigError::OriginCertMissingToken);
        }

        Ok(token)
    }
}

fn token_contains_credentials(token: Option<&OriginCertToken>) -> bool {
    token.is_some_and(|current| !current.zone_id.is_empty() || !current.api_token.is_empty())
}

impl OriginCertUser {
    pub fn read(path: &Path) -> Result<Self> {
        let cert = OriginCertToken::from_pem_path(path)?;

        if cert.account_id.is_empty() {
            return Err(ConfigError::origin_cert_needs_refresh(path));
        }

        Ok(Self {
            cert,
            cert_path: path.to_path_buf(),
        })
    }
}

mod origin_cert_pem {
    use super::{ConfigError, Pem, encode, parse_many};
    use crate::error::Result;

    const ARGO_TUNNEL_TOKEN_BLOCK: &str = "ARGO TUNNEL TOKEN";

    pub(super) struct OriginCertPemBlock(Pem);

    impl OriginCertPemBlock {
        pub(super) fn block_type(&self) -> &str {
            self.0.tag()
        }

        pub(super) fn contents(&self) -> &[u8] {
            self.0.contents()
        }
    }

    pub(super) fn parse_blocks(blocks: &[u8]) -> Result<Vec<OriginCertPemBlock>> {
        parse_many(blocks)
            .map(|blocks| blocks.into_iter().map(OriginCertPemBlock).collect())
            .map_err(|source| ConfigError::origin_cert_invalid_pem(source.to_string()))
    }

    pub(super) fn encode_token(json: Vec<u8>) -> String {
        encode(&Pem::new(ARGO_TUNNEL_TOKEN_BLOCK, json))
    }
}

#[cfg(test)]
mod tests;
