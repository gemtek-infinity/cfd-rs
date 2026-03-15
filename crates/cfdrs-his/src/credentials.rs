//! Credential and origin cert filesystem lookup.
//!
//! Covers HIS-008 (credential search-by-ID), HIS-009 (origin cert search),
//! HIS-010 (tunnel token — type lives in cfdrs-shared), and HIS-011
//! (credential file write with mode 0400).

use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use cfdrs_shared::config::discovery::default_nix_search_directories;
use cfdrs_shared::{ConfigError, DEFAULT_ORIGIN_CERT_FILE, OriginCertToken, Result, TunnelCredentialsFile};
use uuid::Uuid;

// --- HIS-008: credential search-by-ID ---

/// Search for `{tunnel_id}.json` in the origin cert directory (if provided)
/// and then in default config search directories.
///
/// Matches Go `credential_finder.go` `searchByID`.
pub fn search_credential_by_id(tunnel_id: Uuid, origincert_dir: Option<&Path>) -> Result<PathBuf> {
    let filename = format!("{tunnel_id}.json");

    if let Some(dir) = origincert_dir {
        let candidate = dir.join(&filename);

        if candidate.exists() {
            return Ok(candidate);
        }
    }

    for dir in default_nix_search_directories() {
        let candidate = dir.join(&filename);

        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(ConfigError::invariant(format!(
        "could not find credentials file {filename} in any search directory. Use --credentials-file to \
         specify the path explicitly."
    )))
}

/// Load tunnel credentials by searching for `{tunnel_id}.json`.
pub fn load_credentials_by_id(
    tunnel_id: Uuid,
    origincert_dir: Option<&Path>,
) -> Result<TunnelCredentialsFile> {
    let path = search_credential_by_id(tunnel_id, origincert_dir)?;
    TunnelCredentialsFile::from_json_path(&path)
}

// --- HIS-009: origin cert search across dirs ---

/// Search default config directories for `cert.pem`.
///
/// Matches Go `credentials/origin_cert.go` `FindDefaultOriginCertPath()`.
pub fn find_default_origin_cert_path() -> Option<PathBuf> {
    for dir in default_nix_search_directories() {
        let candidate = dir.join(DEFAULT_ORIGIN_CERT_FILE);

        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

/// Load an origin cert from an explicit path or the default search.
///
/// Matches Go `FindOriginCert(originCertPath, log)`.
pub fn find_origin_cert(explicit_path: Option<&Path>) -> Result<OriginCertToken> {
    let path = match explicit_path {
        Some(p) if p.exists() => p.to_path_buf(),
        Some(p) => {
            return Err(ConfigError::invariant(format!(
                "origin certificate not found at {}. Run \"cloudflared login\" to obtain a cert.",
                p.display()
            )));
        }
        None => find_default_origin_cert_path().ok_or_else(|| {
            ConfigError::invariant(
                "could not find a cert.pem in default directories. Run \"cloudflared login\" to create one."
                    .to_owned(),
            )
        })?,
    };

    OriginCertToken::from_pem_path(&path)
}

// --- HIS-011: credential file write with mode 0400 ---

/// Write a credentials JSON file with mode 0400, failing if the file
/// already exists (O_CREATE | O_EXCL).
///
/// Matches Go `tunnel/subcommands.go` credential file creation.
pub fn write_credential_file(path: &Path, cred: &TunnelCredentialsFile) -> Result<()> {
    let json = serde_json::to_string_pretty(cred)
        .map_err(|source| ConfigError::json_serialize("tunnel credentials", source))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::create_directory(parent, source))?;
    }

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true) // O_CREATE | O_EXCL
        .mode(0o400)
        .open(path)
        .map_err(|source| ConfigError::create_file(path, source))?;

    file.write_all(json.as_bytes())
        .map_err(|source| ConfigError::write_file(path, source))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use cfdrs_shared::TunnelSecret;

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cfdrs-his-cred-{name}-{unique}"));
        fs::create_dir_all(&path).expect("temp directory should be created");
        path
    }

    #[test]
    fn search_credential_by_id_finds_file_in_dir() {
        let root = temp_dir("search");
        let id = Uuid::new_v4();
        let cred_path = root.join(format!("{id}.json"));

        let cred = TunnelCredentialsFile {
            account_tag: "test".into(),
            tunnel_secret: TunnelSecret::from_bytes(vec![1, 2, 3]),
            tunnel_id: id,
            endpoint: None,
        };

        let json = serde_json::to_string_pretty(&cred).expect("serialize");
        fs::write(&cred_path, json).expect("write");

        let found = search_credential_by_id(id, Some(&root)).expect("should find");
        assert_eq!(found, cred_path);

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn search_credential_by_id_not_found() {
        let id = Uuid::new_v4();
        let result = search_credential_by_id(id, None);
        assert!(result.is_err());
    }

    #[test]
    fn write_credential_file_creates_with_mode_0400() {
        let root = temp_dir("write");
        let path = root.join("test-cred.json");

        let cred = TunnelCredentialsFile {
            account_tag: "acct".into(),
            tunnel_secret: TunnelSecret::from_bytes(vec![4, 5, 6]),
            tunnel_id: Uuid::new_v4(),
            endpoint: None,
        };

        write_credential_file(&path, &cred).expect("write should succeed");
        assert!(path.exists());

        // Check that O_EXCL prevents overwriting.
        let result = write_credential_file(&path, &cred);
        assert!(result.is_err());

        // Verify content round-trips.
        let content = fs::read_to_string(&path).expect("read");
        let loaded: TunnelCredentialsFile = serde_json::from_str(&content).expect("parse");
        assert_eq!(loaded.account_tag, "acct");

        // Verify permissions (mode 0400).
        use std::os::unix::fs::PermissionsExt;
        let meta = fs::metadata(&path).expect("metadata");
        assert_eq!(meta.permissions().mode() & 0o777, 0o400);

        fs::remove_dir_all(root).expect("cleanup");
    }
}
