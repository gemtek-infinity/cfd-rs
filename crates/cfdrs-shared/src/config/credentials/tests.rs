use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use super::{
    FED_ENDPOINT, OriginCertToken, OriginCertUser, TunnelCredentialsFile, TunnelSecret, TunnelToken,
};

fn ok<T, E: std::fmt::Display>(result: std::result::Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("unexpected error: {error}"),
    }
}

#[test]
fn tunnel_credentials_json_round_trips() {
    let creds = ok(TunnelCredentialsFile::from_json_str(
        r#"{"AccountTag":"account","TunnelSecret":"c2VjcmV0","TunnelID":"11111111-1111-1111-1111-111111111111"}"#,
    ));
    let serialized = ok(creds.to_pretty_json());

    assert_eq!(creds.tunnel_secret.as_bytes(), b"secret");
    assert!(serialized.contains("AccountTag"));
    assert!(serialized.contains("c2VjcmV0"));
    assert!(serialized.contains("11111111-1111-1111-1111-111111111111"));
}

#[test]
fn tunnel_secret_serializes_as_base64() {
    let secret = TunnelSecret::from_bytes(b"secret".to_vec());
    let json = serde_json::to_string(&secret).expect("secret should serialize");

    assert_eq!(json, "\"c2VjcmV0\"");
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
    assert_eq!(error.category().to_string(), "origin-cert-unknown-block");
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
    assert_eq!(error.category().to_string(), "origin-cert-missing-token");
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
    assert_eq!(error.category().to_string(), "origin-cert-multiple-tokens");
}

#[test]
fn malformed_origin_cert_pem_is_rejected_explicitly() {
    let pem = concat!(
        "-----BEGIN ARGO TUNNEL TOKEN-----\n",
        "not base64$$$\n",
        "-----END ARGO TUNNEL TOKEN-----\n"
    );

    let error = OriginCertToken::from_pem_blocks(pem.as_bytes()).expect_err("malformed pem should fail");
    assert_eq!(error.category().to_string(), "origin-cert-invalid-pem");
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
    assert_eq!(error.category().to_string(), "origin-cert-needs-refresh");

    let _ = std::fs::remove_file(path);
}

// --- CDC-042: tunnel token encoding parity ---

#[test]
fn tunnel_token_uses_single_letter_json_keys() {
    let token = TunnelToken {
        account_tag: "acct".to_owned(),
        tunnel_secret: TunnelSecret::from_bytes(b"secret".to_vec()),
        tunnel_id: Uuid::nil(),
        endpoint: None,
    };
    let json = serde_json::to_string(&token).expect("token should serialize");

    assert!(json.contains(r#""a":"acct"#));
    assert!(json.contains(r#""s":"#));
    assert!(json.contains(r#""t":"00000000-0000-0000-0000-000000000000"#));
    assert!(!json.contains("endpoint"), "omitted field should not appear");
}

#[test]
fn tunnel_token_encode_decode_round_trips() {
    let token = TunnelToken {
        account_tag: "account-tag".to_owned(),
        tunnel_secret: TunnelSecret::from_bytes(b"tunnel-secret".to_vec()),
        tunnel_id: Uuid::parse_str("22222222-2222-2222-2222-222222222222").expect("uuid should parse"),
        endpoint: Some("https://example.com".to_owned()),
    };

    let encoded = ok(token.encode());
    let decoded = ok(TunnelToken::decode(&encoded));

    assert_eq!(decoded.account_tag, "account-tag");
    assert_eq!(decoded.tunnel_secret.as_bytes(), b"tunnel-secret");
    assert_eq!(
        decoded.tunnel_id.to_string(),
        "22222222-2222-2222-2222-222222222222"
    );
    assert_eq!(decoded.endpoint.as_deref(), Some("https://example.com"));
}

#[test]
fn tunnel_token_credentials_file_conversion_round_trips() {
    let token = TunnelToken {
        account_tag: "acct".to_owned(),
        tunnel_secret: TunnelSecret::from_bytes(b"sec".to_vec()),
        tunnel_id: Uuid::nil(),
        endpoint: None,
    };

    let creds = token.to_credentials_file();
    let back = TunnelToken::from_credentials_file(&creds);

    assert_eq!(back, token);
}

// --- CDC-043: origin cert JSON field names match baseline ---

#[test]
fn origin_cert_json_field_names_match_baseline() {
    let token = OriginCertToken {
        zone_id: "z".to_owned(),
        account_id: "a".to_owned(),
        api_token: "t".to_owned(),
        endpoint: Some("e".to_owned()),
    };
    let json = serde_json::to_string(&token).expect("token should serialize");

    assert!(json.contains(r#""zoneID":"z"#));
    assert!(json.contains(r#""accountID":"a"#));
    assert!(json.contains(r#""apiToken":"t"#));
    assert!(json.contains(r#""endpoint":"e"#));
}
