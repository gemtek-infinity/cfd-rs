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
fn malformed_origin_cert_pem_is_rejected_explicitly() {
    let pem = concat!(
        "-----BEGIN ARGO TUNNEL TOKEN-----\n",
        "not base64$$$\n",
        "-----END ARGO TUNNEL TOKEN-----\n"
    );

    let error = OriginCertToken::from_pem_blocks(pem.as_bytes()).expect_err("malformed pem should fail");
    assert_eq!(error.category(), "origin-cert-invalid-pem");
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
