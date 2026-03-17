//! Tunnel CRUD commands: create, list, delete, cleanup, token, info.
//!
//! Implements the behavioral dispatch for CLI-010 through CLI-015.
//! Each function loads origin cert → builds API client → calls API →
//! formats output, matching the Go baseline in `tunnel/subcommands.go`.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use uuid::Uuid;

use cfdrs_cdc::api::{ApiClientConfig, CloudflareApiClient, DEFAULT_API_BASE_URL};
use cfdrs_cdc::api_resources::TunnelFilter;
use cfdrs_cli::{CliOutput, GlobalFlags};
use cfdrs_shared::{TunnelCredentialsFile, TunnelSecret};

use crate::api_client::ReqwestApiClient;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Build an [`ApiClientConfig`] and [`ReqwestApiClient`] from an origin cert
/// loaded via `--origincert` or the default search path.
///
/// Matches Go `newSubcommandContext(c)` → `sc.client()`.
pub(crate) fn build_client(flags: &GlobalFlags) -> Result<ReqwestApiClient, String> {
    let cert = cfdrs_his::credentials::find_origin_cert(flags.origincert.as_deref())
        .map_err(|e| format!("error reading origin cert: {e}"))?;

    let base_url = flags
        .api_url
        .as_deref()
        .unwrap_or(DEFAULT_API_BASE_URL)
        .to_owned();

    let config = ApiClientConfig {
        base_url,
        account_tag: cert.account_id.clone(),
        zone_tag: cert.zone_id.clone(),
        auth_token: cert.api_token.clone(),
        user_agent: format!("cloudflared/{}", env!("CARGO_PKG_VERSION")),
    };

    ReqwestApiClient::new(config).map_err(|e| format!("error creating API client: {e}"))
}

/// Resolve a tunnel name-or-UUID to UUID via the API.
///
/// Matches Go `sc.findID(input)`.
pub(crate) fn resolve_tunnel_id(client: &ReqwestApiClient, input: &str) -> Result<Uuid, String> {
    // Try UUID parse first.
    if let Ok(id) = input.parse::<Uuid>() {
        return Ok(id);
    }

    // Fall back to Tunnelstore lookup by name.
    let filter = TunnelFilter::by_name(input);
    let tunnels = client
        .list_tunnels(&filter)
        .map_err(|e| format!("error looking up tunnel: {e}"))?;

    match tunnels.len() {
        1 => Ok(tunnels[0].id),
        0 => Err(format!(
            "{input} is neither the ID nor the name of any of your tunnels"
        )),
        n => Err(format!(
            "there should only be 1 non-deleted Tunnel named {input}, found {n}"
        )),
    }
}

/// Resolve a list of tunnel name-or-UUID strings into UUIDs.
///
/// Matches Go `sc.findIDs(inputs)`.
fn resolve_tunnel_ids(client: &ReqwestApiClient, inputs: &[String]) -> Result<Vec<Uuid>, String> {
    inputs
        .iter()
        .map(|input| resolve_tunnel_id(client, input))
        .collect()
}

/// Render a value as JSON or YAML to stdout.
///
/// Matches Go `renderOutput(format, v)`.
pub(crate) fn render_output(format: &str, value: &impl serde::Serialize) -> Result<String, String> {
    match format {
        "json" => serde_json::to_string_pretty(value)
            .map(|mut s| {
                s.push('\n');
                s
            })
            .map_err(|e| format!("error serializing JSON: {e}")),

        "yaml" => serde_yaml::to_string(value).map_err(|e| format!("error serializing YAML: {e}")),

        other => Err(format!("Unknown output format '{other}'")),
    }
}

/// Format tunnel connections as `Nx<COLO>, Nx<COLO>`.
///
/// Matches Go `fmtConnections()`.
fn fmt_connections(
    connections: &[cfdrs_cdc::api_resources::TunnelConnection],
    show_recently_disconnected: bool,
) -> String {
    let mut per_colo: BTreeMap<&str, u32> = BTreeMap::new();

    for conn in connections {
        if !conn.is_pending_reconnect || show_recently_disconnected {
            *per_colo.entry(&conn.colo_name).or_insert(0) += 1;
        }
    }

    per_colo
        .iter()
        .map(|(colo, count)| format!("{count}x{colo}"))
        .collect::<Vec<_>>()
        .join(", ")
}

// ---------------------------------------------------------------------------
// CLI-010: tunnel create
// ---------------------------------------------------------------------------

/// Execute `tunnel create NAME`.
///
/// Matches Go `createCommand()` in `tunnel/subcommands.go`.
pub fn execute_tunnel_create(flags: &GlobalFlags) -> CliOutput {
    match tunnel_create_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn tunnel_create_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let name = &flags.rest_args[0];
    let client = build_client(flags)?;

    // Generate or decode tunnel secret (32 bytes).
    let tunnel_secret = match flags.tunnel_secret.as_deref() {
        Some(encoded) => {
            use base64::Engine;
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|e| format!("Couldn't decode tunnel secret from base64: {e}"))?;

            if decoded.len() < 32 {
                return Err("Decoded tunnel secret must be at least 32 bytes long".to_owned());
            }
            decoded
        }
        None => {
            // Concatenate two UUID v4 (128-bit random each) to get 32 bytes.
            let a = Uuid::new_v4();
            let b = Uuid::new_v4();
            let mut secret = Vec::with_capacity(32);
            secret.extend_from_slice(a.as_bytes());
            secret.extend_from_slice(b.as_bytes());
            secret
        }
    };

    let result = client
        .create_tunnel(name, &tunnel_secret)
        .map_err(|e| format!("Create Tunnel API call failed: {e}"))?;

    // Load origin cert to determine cert path for default credential file
    // location and to get account info for the credential file.
    let cert = cfdrs_his::credentials::find_origin_cert(flags.origincert.as_deref())
        .map_err(|e| format!("error reading origin cert: {e}"))?;

    let cred = TunnelCredentialsFile {
        account_tag: cert.account_id,
        tunnel_secret: TunnelSecret::from_bytes(tunnel_secret),
        tunnel_id: result.tunnel.id,
        endpoint: cert.endpoint,
    };

    // Determine credential file path.
    let cred_path = match flags.credentials_file.as_deref() {
        Some(p) => p.to_path_buf(),
        None => {
            let origincert_path = cfdrs_his::credentials::find_default_origin_cert_path()
                .ok_or_else(|| "could not find a cert.pem in default directories".to_owned())?;
            let cert_dir = origincert_path.parent().unwrap_or_else(|| Path::new("."));
            cert_dir.join(format!("{}.json", result.tunnel.id))
        }
    };

    let used_cert_path = flags.credentials_file.is_none();

    if let Err(write_err) = cfdrs_his::credentials::write_credential_file(&cred_path, &cred) {
        // Go baseline: on write failure, try to delete the tunnel, then report.
        let mut lines = vec![format!(
            "Your tunnel '{}' was created with ID {}. However, cloudflared couldn't write tunnel \
             credentials to {}.",
            result.tunnel.name,
            result.tunnel.id,
            cred_path.display()
        )];
        lines.push(format!("The file-writing error is: {write_err}"));

        if let Err(del_err) = client.delete_tunnel(result.tunnel.id, true) {
            lines.push(format!(
                "Cloudflared tried to delete the tunnel for you, but encountered an error. You should use \
                 `cloudflared tunnel delete {}` to delete the tunnel yourself, because the tunnel can't be \
                 run without the tunnelfile.",
                result.tunnel.id
            ));
            lines.push(format!("The delete tunnel error is: {del_err}"));
        } else {
            lines.push(
                "The tunnel was deleted, because the tunnel can't be run without the credentials file"
                    .to_owned(),
            );
        }

        return Err(lines.join("\n"));
    }

    // If --output was set, render JSON/YAML instead of human text.
    if let Some(ref fmt) = flags.output_format {
        let output = render_output(fmt, &result)?;
        return Ok(CliOutput::success(output));
    }

    let mut stdout = String::new();
    let _ = write!(stdout, "Tunnel credentials written to {}.", cred_path.display());

    if used_cert_path {
        stdout.push_str(" cloudflared chose this file based on where your origin certificate was found.");
    }

    stdout.push_str(" Keep this file secret. To revoke these credentials, delete the tunnel.\n");
    let _ = write!(
        stdout,
        "\nCreated tunnel {} with id {}\n",
        result.tunnel.name, result.tunnel.id
    );

    Ok(CliOutput::success(stdout))
}

// ---------------------------------------------------------------------------
// CLI-011: tunnel list
// ---------------------------------------------------------------------------

/// Execute `tunnel list`.
///
/// Matches Go `listCommand()` in `tunnel/subcommands.go`.
pub fn execute_tunnel_list(flags: &GlobalFlags) -> CliOutput {
    match tunnel_list_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn tunnel_list_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;

    let mut filter = TunnelFilter::default();

    if !flags.show_deleted {
        filter.is_deleted = Some(false);
    }

    if let Some(ref name) = flags.tunnel_name {
        filter.name = Some(name.clone());
    }

    if let Some(ref prefix) = flags.name_prefix {
        filter.name_prefix = Some(prefix.clone());
    }

    if let Some(ref exclude) = flags.exclude_name_prefix {
        filter.exclude_prefix = Some(exclude.clone());
    }

    if let Some(ref when) = flags.filter_when {
        filter.existed_at = Some(when.clone());
    }

    if let Some(ref id_str) = flags.tunnel_id {
        let id = id_str
            .parse::<Uuid>()
            .map_err(|e| format!("{id_str} is not a valid tunnel ID: {e}"))?;
        filter.tunnel_id = Some(id);
    }

    let tunnels = client
        .list_tunnels(&filter)
        .map_err(|e| format!("error listing tunnels: {e}"))?;

    // If --output was set, render JSON/YAML.
    if let Some(ref fmt) = flags.output_format {
        let output = render_output(fmt, &tunnels)?;
        return Ok(CliOutput::success(output));
    }

    // Tab-separated output matching Go formatAndPrintTunnelList.
    let show_disconnected = flags.show_recently_disconnected;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "You can obtain more detailed information for each tunnel with `cloudflared tunnel info <name/uuid>`"
    );
    let _ = writeln!(out, "ID\tNAME\tCREATED\tCONNECTIONS");

    for t in &tunnels {
        let conns = fmt_connections(&t.connections, show_disconnected);
        let _ = writeln!(out, "{}\t{}\t{}\t{}", t.id, t.name, t.created_at, conns);
    }

    Ok(CliOutput::success(out))
}

// ---------------------------------------------------------------------------
// CLI-013: tunnel delete
// ---------------------------------------------------------------------------

/// Execute `tunnel delete TUNNEL [TUNNEL ...]`.
///
/// Matches Go `deleteCommand()` in `tunnel/subcommands.go`.
pub fn execute_tunnel_delete(flags: &GlobalFlags) -> CliOutput {
    match tunnel_delete_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn tunnel_delete_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;
    let tunnel_ids = resolve_tunnel_ids(&client, &flags.rest_args)?;

    for id in &tunnel_ids {
        // Validate tunnel exists and is not already deleted.
        let tunnel = client
            .get_tunnel(*id)
            .map_err(|e| format!("Can't get tunnel information. Please check tunnel id: {id}: {e}"))?;

        if !tunnel.deleted_at.is_empty() {
            return Err(format!("Tunnel {} has already been deleted", tunnel.id));
        }

        client
            .delete_tunnel(*id, flags.force)
            .map_err(|e| format!("Error deleting tunnel {id}: {e}"))?;

        // Remove local credentials file (non-fatal if missing).
        let origincert_path = cfdrs_his::credentials::find_default_origin_cert_path();

        if let Some(ref cert_path) = origincert_path {
            let cred_path = cert_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(format!("{id}.json"));

            if cred_path.exists()
                && let Err(e) = std::fs::remove_file(&cred_path)
            {
                // Go baseline: log warning but don't fail.
                eprintln!(
                    "Tunnel {id} was deleted, but we could not remove its credentials file {}: {e}. \
                     Consider deleting this file manually.",
                    cred_path.display()
                );
            }
        }
    }

    Ok(CliOutput::success(String::new()))
}

// ---------------------------------------------------------------------------
// CLI-014: tunnel cleanup
// ---------------------------------------------------------------------------

/// Execute `tunnel cleanup TUNNEL [TUNNEL ...]`.
///
/// Matches Go `cleanupCommand()` in `tunnel/subcommands.go`.
pub fn execute_tunnel_cleanup(flags: &GlobalFlags) -> CliOutput {
    match tunnel_cleanup_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn tunnel_cleanup_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;
    let tunnel_ids = resolve_tunnel_ids(&client, &flags.rest_args)?;

    let connector_id = match flags.connector_id.as_deref() {
        Some(s) => {
            let id = s
                .parse::<Uuid>()
                .map_err(|e| format!("{s} is not a valid client ID (must be a UUID): {e}"))?;
            Some(id)
        }
        None => None,
    };

    let mut stderr = String::new();

    for id in &tunnel_ids {
        let extra = connector_id
            .map(|c| format!(" for connector-id {c}"))
            .unwrap_or_default();

        let _ = writeln!(stderr, "Cleanup connection for tunnel {id}{extra}");

        if let Err(e) = client.cleanup_connections(*id, connector_id) {
            let _ = writeln!(
                stderr,
                "Error cleaning up connections for tunnel {id}, error: {e}"
            );
        }
    }

    Ok(CliOutput {
        stdout: String::new(),
        stderr,
        exit_code: 0,
    })
}

// ---------------------------------------------------------------------------
// CLI-015: tunnel token
// ---------------------------------------------------------------------------

/// Execute `tunnel token TUNNEL`.
///
/// Matches Go `tokenCommand()` in `tunnel/subcommands.go`.
pub fn execute_tunnel_token(flags: &GlobalFlags) -> CliOutput {
    match tunnel_token_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn tunnel_token_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;
    let tunnel_id = resolve_tunnel_id(&client, &flags.rest_args[0])?;

    let token_str = client
        .get_tunnel_token(tunnel_id)
        .map_err(|e| format!("error fetching tunnel token: {e}"))?;

    // Parse the token so we can either write a creds file or re-encode.
    let token = cfdrs_shared::TunnelToken::decode(&token_str)
        .map_err(|e| format!("error parsing tunnel token: {e}"))?;

    // If --credentials-file was given, write to file instead of stdout.
    if let Some(ref path) = flags.credentials_file {
        let cred = token.to_credentials_file();
        cfdrs_his::credentials::write_credential_file(path, &cred).map_err(|e| {
            format!(
                "error writing token credentials to JSON file in path {}: {e}",
                path.display()
            )
        })?;

        return Ok(CliOutput::success(String::new()));
    }

    // Otherwise print the token to stdout.
    let encoded = token
        .encode()
        .map_err(|e| format!("error encoding tunnel token: {e}"))?;

    Ok(CliOutput::success(format!("{encoded}\n")))
}

// ---------------------------------------------------------------------------
// CLI-016: tunnel info (stub — deferred to Command Family Closure)
// ---------------------------------------------------------------------------

/// Execute `tunnel info TUNNEL`.
///
/// Matches Go `tunnelInfo()` in `tunnel/subcommands.go`.
pub fn execute_tunnel_info(flags: &GlobalFlags) -> CliOutput {
    match tunnel_info_inner(flags) {
        Ok(output) => output,
        Err(msg) => CliOutput::failure(String::new(), msg, 1),
    }
}

fn tunnel_info_inner(flags: &GlobalFlags) -> Result<CliOutput, String> {
    let client = build_client(flags)?;
    let tunnel_id = resolve_tunnel_id(&client, &flags.rest_args[0])?;

    let clients = client
        .list_active_clients(tunnel_id)
        .map_err(|e| format!("error listing active clients: {e}"))?;

    let tunnel = client
        .get_tunnel(tunnel_id)
        .map_err(|e| format!("error fetching tunnel: {e}"))?;

    // If --output was set, render JSON/YAML.
    if let Some(ref fmt) = flags.output_format {
        #[derive(serde::Serialize)]
        struct Info {
            id: Uuid,
            name: String,
            created_at: String,
            connectors: Vec<cfdrs_cdc::api_resources::ActiveClient>,
        }

        let info = Info {
            id: tunnel.id,
            name: tunnel.name,
            created_at: tunnel.created_at,
            connectors: clients,
        };

        let output = render_output(fmt, &info)?;
        return Ok(CliOutput::success(output));
    }

    let out = render_tunnel_info_table(&tunnel, &clients, flags.show_recently_disconnected);
    Ok(CliOutput::success(out))
}

/// Render the human-readable tunnel info table (header + connector rows).
fn render_tunnel_info_table(
    tunnel: &cfdrs_cdc::api_resources::Tunnel,
    clients: &[cfdrs_cdc::api_resources::ActiveClient],
    show_disconnected: bool,
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "NAME:\t{}", tunnel.name);
    let _ = writeln!(out, "ID:\t{}", tunnel.id);
    let _ = writeln!(out, "CREATED:\t{}\n", tunnel.created_at);

    let has_active = clients
        .iter()
        .any(|c| !fmt_connections(&c.connections, show_disconnected).is_empty());

    if !has_active {
        let _ = writeln!(out, "This tunnel has no active connectors.");
        return out;
    }

    let _ = writeln!(
        out,
        "CONNECTOR ID\tCREATED\tARCHITECTURE\tVERSION\tORIGIN IP\tEDGE"
    );

    for c in clients {
        let conns = fmt_connections(&c.connections, show_disconnected);
        if conns.is_empty() {
            continue;
        }
        let origin_ip = c
            .connections
            .first()
            .map(|conn| conn.origin_ip.as_str())
            .unwrap_or("");
        let _ = writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}",
            c.id, c.run_at, c.arch, c.version, origin_ip, conns
        );
    }

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- fmt_connections ---

    #[test]
    fn fmt_connections_empty() {
        assert_eq!(fmt_connections(&[], false), "");
    }

    #[test]
    fn fmt_connections_single_colo() {
        use cfdrs_cdc::api_resources::TunnelConnection;

        let conns = vec![TunnelConnection {
            colo_name: "IAD".to_owned(),
            id: Uuid::nil(),
            is_pending_reconnect: false,
            origin_ip: String::new(),
            opened_at: String::new(),
        }];

        assert_eq!(fmt_connections(&conns, false), "1xIAD");
    }

    #[test]
    fn fmt_connections_multiple_colos_sorted() {
        use cfdrs_cdc::api_resources::TunnelConnection;

        let conns = vec![
            TunnelConnection {
                colo_name: "SFO".to_owned(),
                id: Uuid::nil(),
                is_pending_reconnect: false,
                origin_ip: String::new(),
                opened_at: String::new(),
            },
            TunnelConnection {
                colo_name: "IAD".to_owned(),
                id: Uuid::nil(),
                is_pending_reconnect: false,
                origin_ip: String::new(),
                opened_at: String::new(),
            },
            TunnelConnection {
                colo_name: "SFO".to_owned(),
                id: Uuid::nil(),
                is_pending_reconnect: false,
                origin_ip: String::new(),
                opened_at: String::new(),
            },
        ];

        // BTreeMap sorts alphabetically: IAD before SFO.
        assert_eq!(fmt_connections(&conns, false), "1xIAD, 2xSFO");
    }

    #[test]
    fn fmt_connections_skips_pending_reconnect() {
        use cfdrs_cdc::api_resources::TunnelConnection;

        let conns = vec![
            TunnelConnection {
                colo_name: "IAD".to_owned(),
                id: Uuid::nil(),
                is_pending_reconnect: true,
                origin_ip: String::new(),
                opened_at: String::new(),
            },
            TunnelConnection {
                colo_name: "SFO".to_owned(),
                id: Uuid::nil(),
                is_pending_reconnect: false,
                origin_ip: String::new(),
                opened_at: String::new(),
            },
        ];

        assert_eq!(fmt_connections(&conns, false), "1xSFO");
        assert_eq!(fmt_connections(&conns, true), "1xIAD, 1xSFO");
    }

    // --- resolve_tunnel_id ---

    #[test]
    fn resolve_uuid_directly() {
        // A plain UUID should be returned without API call.
        let input = "00000000-0000-0000-0000-000000000001";
        let expected: Uuid = input.parse().expect("valid uuid");

        // We can't call resolve_tunnel_id without a real client, but we can
        // verify the UUID parsing branch works.
        let parsed: Result<Uuid, _> = input.parse();
        assert_eq!(parsed.expect("uuid parse"), expected);
    }

    // --- render_output ---

    #[test]
    fn render_json_output() {
        #[derive(serde::Serialize)]
        struct Sample {
            name: String,
        }

        let s = Sample {
            name: "test".to_owned(),
        };
        let json = render_output("json", &s).expect("json should render");
        assert!(json.contains("\"name\": \"test\""));
        assert!(json.ends_with('\n'));
    }

    #[test]
    fn render_yaml_output() {
        #[derive(serde::Serialize)]
        struct Sample {
            name: String,
        }

        let s = Sample {
            name: "test".to_owned(),
        };
        let yaml = render_output("yaml", &s).expect("yaml should render");
        assert!(yaml.contains("name: test"));
    }

    #[test]
    fn render_unknown_format_errors() {
        let s = "value";
        let err = render_output("xml", &s).expect_err("xml should error");
        assert!(err.contains("Unknown output format 'xml'"));
    }
}
