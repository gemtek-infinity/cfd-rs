use std::fs;
use std::path::{Path, PathBuf};

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
        for block in parse_pem_blocks(blocks) {
            match block.block_type.as_str() {
                "PRIVATE KEY" | "CERTIFICATE" => {}
                "ARGO TUNNEL TOKEN" => {
                    if token.as_ref().is_some_and(|current: &OriginCertToken| {
                        !current.zone_id.is_empty() || !current.api_token.is_empty()
                    }) {
                        return Err(ConfigError::OriginCertMultipleTokens);
                    }

                    if let Ok(decoded) = Self::from_json_bytes(&block.bytes) {
                        token = Some(decoded);
                    }
                }
                other => return Err(ConfigError::origin_cert_unknown_block(other)),
            }
        }

        let token = token.ok_or(ConfigError::OriginCertMissingToken)?;
        if token.zone_id.is_empty() || token.api_token.is_empty() {
            return Err(ConfigError::OriginCertMissingToken);
        }

        Ok(token)
    }

    pub fn from_pem_path(path: &Path) -> Result<Self> {
        let contents = fs::read(path).map_err(|source| ConfigError::read(path, source))?;
        Self::from_pem_blocks(&contents)
    }

    pub fn encode_pem(&self) -> Result<Vec<u8>> {
        let json = serde_json::to_vec(self)
            .map_err(|source| ConfigError::json_serialize("origin cert token", source))?;

        let mut pem = String::from("-----BEGIN ARGO TUNNEL TOKEN-----\n");
        let encoded = encode_base64(&json);
        for chunk in encoded.as_bytes().chunks(64) {
            pem.push_str(std::str::from_utf8(chunk).expect("base64 output should be utf-8"));
            pem.push('\n');
        }
        pem.push_str("-----END ARGO TUNNEL TOKEN-----\n");
        Ok(pem.into_bytes())
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

#[derive(Debug, Clone, Eq, PartialEq)]
struct PemBlock {
    block_type: String,
    bytes: Vec<u8>,
}

fn parse_pem_blocks(blocks: &[u8]) -> Vec<PemBlock> {
    let mut parsed = Vec::new();
    let text = String::from_utf8_lossy(blocks);
    let mut remaining = text.as_ref();

    while let Some(begin_offset) = remaining.find("-----BEGIN ") {
        remaining = &remaining[begin_offset + "-----BEGIN ".len()..];
        let Some(type_end) = remaining.find("-----") else {
            break;
        };

        let block_type = remaining[..type_end].to_owned();
        remaining = &remaining[type_end + "-----".len()..];
        remaining = remaining.strip_prefix("\r\n").unwrap_or(remaining);
        remaining = remaining.strip_prefix('\n').unwrap_or(remaining);

        let end_marker = format!("-----END {block_type}-----");
        let Some(end_offset) = remaining.find(&end_marker) else {
            break;
        };

        let body = &remaining[..end_offset];
        parsed.push(PemBlock {
            block_type,
            bytes: decode_base64(body).unwrap_or_default(),
        });
        remaining = &remaining[end_offset + end_marker.len()..];
    }

    parsed
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let filtered: Vec<u8> = input.bytes().filter(|byte| !byte.is_ascii_whitespace()).collect();
    if filtered.is_empty() {
        return Some(Vec::new());
    }
    if !filtered.len().is_multiple_of(4) {
        return None;
    }

    let mut output = Vec::with_capacity(filtered.len() / 4 * 3);
    for chunk in filtered.chunks(4) {
        let mut values = [0u8; 4];
        let mut padding = 0usize;

        for (index, byte) in chunk.iter().copied().enumerate() {
            if byte == b'=' {
                values[index] = 0;
                padding += 1;
            } else {
                values[index] = base64_value(byte)?;
            }
        }

        output.push((values[0] << 2) | (values[1] >> 4));
        if padding < 2 {
            output.push((values[1] << 4) | (values[2] >> 2));
        }
        if padding == 0 {
            output.push((values[2] << 6) | values[3]);
        }
    }

    Some(output)
}

fn encode_base64(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);

        output.push(ALPHABET[(first >> 2) as usize] as char);
        output.push(ALPHABET[(((first & 0x03) << 4) | (second >> 4)) as usize] as char);

        if chunk.len() > 1 {
            output.push(ALPHABET[(((second & 0x0f) << 2) | (third >> 6)) as usize] as char);
        } else {
            output.push('=');
        }

        if chunk.len() > 2 {
            output.push(ALPHABET[(third & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
    }

    output
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{FED_ENDPOINT, OriginCertToken, OriginCertUser, TunnelCredentialsFile};

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

        assert_eq!(token.endpoint.as_deref(), Some(FED_ENDPOINT));
    }

    #[test]
    fn origin_cert_pem_round_trips() {
        let token = OriginCertToken {
            zone_id: "zone".to_owned(),
            account_id: "account".to_owned(),
            api_token: "token".to_owned(),
            endpoint: Some("FED".to_owned()),
        };

        let pem = ok(token.encode_pem());
        let decoded = ok(OriginCertToken::from_pem_blocks(&pem));

        assert_eq!(decoded.zone_id, "zone");
        assert_eq!(decoded.account_id, "account");
        assert_eq!(decoded.api_token, "token");
        assert_eq!(decoded.endpoint.as_deref(), Some(FED_ENDPOINT));
        assert!(decoded.is_fed_endpoint());
    }

    #[test]
    fn origin_cert_unknown_block_is_rejected() {
        let pem = b"-----BEGIN RSA PRIVATE KEY-----\nZm9v\n-----END RSA PRIVATE KEY-----\n";
        let error = OriginCertToken::from_pem_blocks(pem).expect_err("unknown block should fail");

        assert_eq!(
            error.to_string(),
            "unknown block RSA PRIVATE KEY in the certificate"
        );
        assert_eq!(error.category(), "origin-cert-unknown-block");
    }

    #[test]
    fn origin_cert_missing_token_is_rejected() {
        let pem = concat!(
            "-----BEGIN PRIVATE KEY-----\n",
            "Zm9v\n",
            "-----END PRIVATE KEY-----\n",
            "-----BEGIN CERTIFICATE-----\n",
            "YmFy\n",
            "-----END CERTIFICATE-----\n"
        );

        let error = OriginCertToken::from_pem_blocks(pem.as_bytes()).expect_err("missing token should fail");
        assert_eq!(error.to_string(), "missing token in the certificate");
        assert_eq!(error.category(), "origin-cert-missing-token");
    }

    #[test]
    fn origin_cert_multiple_tokens_is_rejected() {
        let token = OriginCertToken {
            zone_id: "zone".to_owned(),
            account_id: "account".to_owned(),
            api_token: "token".to_owned(),
            endpoint: None,
        };
        let mut pem = ok(token.encode_pem());
        pem.extend(ok(token.encode_pem()));

        let error = OriginCertToken::from_pem_blocks(&pem).expect_err("multiple tokens should fail");
        assert_eq!(error.to_string(), "found multiple tokens in the certificate");
        assert_eq!(error.category(), "origin-cert-multiple-tokens");
    }

    #[test]
    fn origin_cert_user_requires_account_id() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cloudflared-origin-cert-{unique}.pem"));
        let token = OriginCertToken {
            zone_id: "zone".to_owned(),
            account_id: String::new(),
            api_token: "token".to_owned(),
            endpoint: None,
        };
        std::fs::write(&path, ok(token.encode_pem())).expect("pem should be written");

        let error = OriginCertUser::read(&path).expect_err("empty account id should fail");
        assert_eq!(error.category(), "origin-cert-needs-refresh");

        let _ = std::fs::remove_file(path);
    }
}
