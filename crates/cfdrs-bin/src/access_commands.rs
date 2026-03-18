//! Access subtree behavioral handlers.
//!
//! CLI-022 closes the generic placeholder boundary for `cloudflared access`
//! by routing each subcommand to an explicit behavior surface. Most
//! subcommands remain honest deferred boundaries for the current Rust lane;
//! `ssh-config` is self-contained and implemented directly.

use std::env;
use std::fs;
use std::path::Path;

use cfdrs_cli::{CliOutput, GlobalFlags};
use cfdrs_his::environment::current_executable;

const ACCESS_DEFERRED_TEMPLATE: &str = "error: `cloudflared {command}` is not available in the Rust rewrite \
                                        yet.\nThis Access subcommand depends on {detail}, which remains \
                                        outside the current admitted Rust runtime surface.\nUse the Go \
                                        baseline `cloudflared {command}` if you need this behavior today.\n";

const ACCESS_LOGIN_DETAIL: &str = "the browser-based Access token flow and local token storage";
const ACCESS_CURL_DETAIL: &str = "the curl wrapper, Access token flow, and JWT header injection path";
const ACCESS_TOKEN_DETAIL: &str = "Access application token storage and retrieval";
const ACCESS_TCP_DETAIL: &str = "the carrier WebSocket proxy/client path used by access tcp/rdp/ssh/smb";
const ACCESS_SSH_GEN_DETAIL: &str = "short-lived SSH certificate generation and Access token retrieval";

pub fn execute_access_login(_flags: &GlobalFlags) -> CliOutput {
    access_deferred("access login", ACCESS_LOGIN_DETAIL)
}

pub fn execute_access_curl(_flags: &GlobalFlags) -> CliOutput {
    access_deferred("access curl", ACCESS_CURL_DETAIL)
}

pub fn execute_access_token(_flags: &GlobalFlags) -> CliOutput {
    access_deferred("access token", ACCESS_TOKEN_DETAIL)
}

pub fn execute_access_tcp(_flags: &GlobalFlags) -> CliOutput {
    access_deferred("access tcp", ACCESS_TCP_DETAIL)
}

pub fn execute_access_ssh_gen(_flags: &GlobalFlags) -> CliOutput {
    access_deferred("access ssh-gen", ACCESS_SSH_GEN_DETAIL)
}

pub fn execute_access_ssh_config(flags: &GlobalFlags) -> CliOutput {
    CliOutput::success(render_ssh_config(flags))
}

fn access_deferred(command: &str, detail: &str) -> CliOutput {
    CliOutput::failure(String::new(), deferred_message(command, detail), 1)
}

fn deferred_message(command: &str, detail: &str) -> String {
    ACCESS_DEFERRED_TEMPLATE
        .replace("{command}", command)
        .replace("{detail}", detail)
}

fn render_ssh_config(flags: &GlobalFlags) -> String {
    let hostname = access_hostname(flags).unwrap_or("[your hostname]");
    let cloudflared = cloudflared_path();
    let home = env::var("HOME").unwrap_or_default();
    let short_lived_certs = access_flag_present(flags, &["--short-lived-cert"]);

    if short_lived_certs {
        format!(
            "Add to your {home}/.ssh/config:\n\nMatch host {hostname} exec \"{cloudflared} access ssh-gen \
             --hostname %h\"\n  ProxyCommand {cloudflared} access ssh --hostname %h\n  IdentityFile \
             ~/.cloudflared/%h-cf_key\n  CertificateFile ~/.cloudflared/%h-cf_key-cert.pub\n"
        )
    } else {
        format!(
            "Add to your {home}/.ssh/config:\n\nHost {hostname}\n  ProxyCommand {cloudflared} access ssh \
             --hostname %h\n"
        )
    }
}

fn access_hostname(flags: &GlobalFlags) -> Option<&str> {
    flags
        .hostname
        .as_deref()
        .filter(|value| !value.is_empty())
        .or_else(|| access_flag_value(flags, &["--hostname", "--tunnel-host", "-T"]))
}

fn access_flag_present(flags: &GlobalFlags, names: &[&str]) -> bool {
    flags.rest_args.iter().any(|arg| names.contains(&arg.as_str()))
}

fn access_flag_value<'a>(flags: &'a GlobalFlags, names: &[&str]) -> Option<&'a str> {
    let mut iter = flags.rest_args.iter();
    while let Some(arg) = iter.next() {
        if names.contains(&arg.as_str()) {
            return iter
                .next()
                .map(|value| value.as_str())
                .filter(|value| !value.is_empty());
        }
    }
    None
}

fn cloudflared_path() -> String {
    if let Ok(path) = current_executable()
        && is_regular_file(&path)
    {
        return path.display().to_string();
    }

    if let Some(path_env) = env::var_os("PATH") {
        for dir in env::split_paths(&path_env) {
            let candidate = dir.join("cloudflared");
            if is_regular_file(&candidate) {
                return candidate.display().to_string();
            }
        }
    }

    "cloudflared".to_owned()
}

fn is_regular_file(path: &Path) -> bool {
    fs::metadata(path).map(|meta| meta.is_file()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_deferred_message_mentions_browser_flow() {
        let output = execute_access_login(&GlobalFlags::default());
        assert_eq!(output.exit_code, 1);
        assert!(output.stderr.contains("browser-based Access token flow"));
        assert!(output.stderr.contains("cloudflared access login"));
    }

    #[test]
    fn ssh_config_uses_placeholder_hostname_by_default() {
        let output = execute_access_ssh_config(&GlobalFlags::default());
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("Host [your hostname]"));
        assert!(output.stdout.contains("ProxyCommand"));
    }

    #[test]
    fn ssh_config_renders_short_lived_cert_variant() {
        let mut flags = GlobalFlags {
            hostname: Some("ssh.example.com".to_owned()),
            ..GlobalFlags::default()
        };
        flags.rest_args.push("--short-lived-cert".to_owned());

        let output = execute_access_ssh_config(&flags);
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("Match host ssh.example.com"));
        assert!(output.stdout.contains("access ssh-gen --hostname %h"));
        assert!(output.stdout.contains("IdentityFile ~/.cloudflared/%h-cf_key"));
    }

    #[test]
    fn ssh_config_reads_hostname_alias_from_rest_args() {
        let flags = GlobalFlags {
            rest_args: vec!["-T".to_owned(), "alias.example.com".to_owned()],
            ..GlobalFlags::default()
        };

        let output = execute_access_ssh_config(&flags);
        assert!(output.stdout.contains("Host alias.example.com"));
    }
}
