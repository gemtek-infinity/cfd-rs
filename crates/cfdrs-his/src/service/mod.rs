//! Linux service install/uninstall.
//!
//! Covers HIS-012 through HIS-023.

pub mod systemd;
pub mod sysv;

use std::path::{Path, PathBuf};

use cfdrs_shared::{ConfigError, Result};

/// Trait abstracting shell command execution so installs can be tested
/// without touching systemctl/service.
pub trait CommandRunner {
    /// Run an external command with args. Return `Ok(())` on success,
    /// `Err` on non-zero exit.
    fn run(&self, program: &str, args: &[&str]) -> Result<()>;
}

/// Default implementation that calls `std::process::Command`.
pub struct ProcessRunner;

impl CommandRunner for ProcessRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<()> {
        let status = std::process::Command::new(program)
            .args(args)
            .status()
            .map_err(|e| ConfigError::invariant(format!("failed to execute {program}: {e}")))?;

        if status.success() {
            Ok(())
        } else {
            Err(ConfigError::invariant(format!(
                "{program} exited with status {status}"
            )))
        }
    }
}

/// HIS-021: detect systemd by checking `/run/systemd/system`.
pub fn is_systemd() -> bool {
    Path::new("/run/systemd/system").exists()
}

/// Service config paths (HIS-019, HIS-020).
pub const SERVICE_CONFIG_DIR: &str = "/etc/cloudflared";
pub const SERVICE_CONFIG_FILE: &str = "config.yml";
pub const SERVICE_CONFIG_PATH: &str = "/etc/cloudflared/config.yml";

/// HIS-019: ensure `/etc/cloudflared/` exists.
pub fn ensure_config_dir_exists() -> Result<()> {
    let dir = Path::new(SERVICE_CONFIG_DIR);

    if !dir.exists() {
        std::fs::create_dir_all(dir).map_err(|e| ConfigError::create_directory(dir, e))?;
    }

    Ok(())
}

/// Arguments to fill service templates with.
#[derive(Debug, Clone)]
pub struct ServiceTemplateArgs {
    /// Path to the cloudflared binary.
    pub path: PathBuf,
    /// Extra arguments appended to ExecStart.
    pub extra_args: Vec<String>,
}

/// HIS-020: build args for config-file-based install.
///
/// Go: `buildArgsForConfig` returns `["--config",
/// "/etc/cloudflared/config.yml", "tunnel", "run"]`.
pub fn build_args_for_config() -> Vec<String> {
    vec![
        "--config".into(),
        SERVICE_CONFIG_PATH.into(),
        "tunnel".into(),
        "run".into(),
    ]
}

/// Build args for token-based install.
///
/// Go: `buildArgsForToken` returns `["tunnel", "run", "--token", <token>]`.
pub fn build_args_for_token(token: &str) -> Vec<String> {
    vec!["tunnel".into(), "run".into(), "--token".into(), token.into()]
}

/// HIS-012, HIS-013: install Linux service.
///
/// Dispatches to systemd or sysv based on `is_systemd()`.
pub fn install_linux_service(
    args: &ServiceTemplateArgs,
    auto_update: bool,
    runner: &dyn CommandRunner,
) -> Result<()> {
    ensure_config_dir_exists()?;

    if is_systemd() {
        systemd::install(args, auto_update, runner)
    } else {
        sysv::install(args, auto_update, runner)
    }
}

/// HIS-017: uninstall Linux service.
pub fn uninstall_linux_service(runner: &dyn CommandRunner) -> Result<()> {
    if is_systemd() {
        systemd::uninstall(runner)
    } else {
        sysv::uninstall(runner)
    }
}

/// Copy a file from `src` to `dest`.
pub fn copy_file(src: &Path, dest: &Path) -> Result<()> {
    std::fs::copy(src, dest).map_err(|e| {
        ConfigError::invariant(format!(
            "failed to copy {} to {}: {e}",
            src.display(),
            dest.display()
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_args_for_config_matches_go() {
        let args = build_args_for_config();
        assert_eq!(args, ["--config", "/etc/cloudflared/config.yml", "tunnel", "run"]);
    }

    #[test]
    fn build_args_for_token_matches_go() {
        let args = build_args_for_token("eyJhIjoiYWNjdCJ9");
        assert_eq!(args, ["tunnel", "run", "--token", "eyJhIjoiYWNjdCJ9"]);
    }

    // --- HIS-021: is_systemd detection ---

    #[test]
    fn is_systemd_checks_run_systemd_system() {
        // Go: isSystemd() checks for `/run/systemd/system` directory.
        // We just verify the function is callable and returns a bool
        // consistent with the presence of that path.
        let expected = Path::new("/run/systemd/system").exists();
        assert_eq!(is_systemd(), expected);
    }

    /// HIS-019: `SERVICE_CONFIG_DIR` and `SERVICE_CONFIG_PATH` constants
    /// match the Go baseline's hardcoded paths.
    #[test]
    fn service_config_paths_match_go() {
        assert_eq!(SERVICE_CONFIG_DIR, "/etc/cloudflared");
        assert_eq!(SERVICE_CONFIG_PATH, "/etc/cloudflared/config.yml");
    }

    /// HIS-020/013: token-based args never include `--config`.
    #[test]
    fn build_args_for_token_does_not_include_config() {
        let args = build_args_for_token("tok_abc");
        assert!(!args.contains(&"--config".to_string()));
        assert!(args.contains(&"--token".to_string()));
        assert!(args.contains(&"tok_abc".to_string()));
    }
}
