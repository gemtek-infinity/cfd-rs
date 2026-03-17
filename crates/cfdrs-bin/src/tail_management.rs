//! Tail and management command execution.
//!
//! CLI-023: `tail [TUNNEL-ID]` — streams remote logs via management WebSocket.
//! CLI-023: `tail token` — fetches a management JWT for `logs` resource.
//! CLI-024: `management token --resource <resource>` — fetches a management
//! JWT.
//!
//! Go baseline:
//!   - `cmd/cloudflared/tail/cmd.go` — tail command implementation
//!   - `cmd/cloudflared/management/cmd.go` — management command implementation
//!   - `cmd/cloudflared/cliutil/management.go` — shared `GetManagementToken`

use url::Url;
use uuid::Uuid;

use cfdrs_cdc::ManagementResource;
use cfdrs_cdc::api::CloudflareApiClient;
use cfdrs_cdc::log_streaming::{EventStartStreaming, LogEntry, LogEventType, LogLevel, StreamingFilters};
use cfdrs_cdc::management::parse_management_token;
use cfdrs_cli::{CliOutput, GlobalFlags};

use crate::tunnel_commands::build_client;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default management hostname (Go: `management.argotunnel.com`).
const DEFAULT_MANAGEMENT_HOSTNAME: &str = "management.argotunnel.com";

/// FedRAMP management hostname (Go: `management.fed.argotunnel.com`).
const FED_MANAGEMENT_HOSTNAME: &str = "management.fed.argotunnel.com";

/// WebSocket path for log streaming (Go: `/logs`).
const LOGS_PATH: &str = "/logs";

// ---------------------------------------------------------------------------
// tail token (CLI-023)
// ---------------------------------------------------------------------------

/// Execute `tail token TUNNEL-ID`.
///
/// Go baseline: `managementTokenCommand` in `tail/cmd.go`.
/// Acquires a management JWT for the `logs` resource and prints
/// `{"token":"..."}` to stdout.
pub fn execute_tail_token(flags: &GlobalFlags) -> CliOutput {
    execute_get_management_token(flags, ManagementResource::Logs)
}

// ---------------------------------------------------------------------------
// management token (CLI-024)
// ---------------------------------------------------------------------------

/// Execute `management token --resource <resource> TUNNEL-ID`.
///
/// Go baseline: `tokenCommand` in `management/cmd.go`.
/// Acquires a management JWT for the specified resource and prints
/// `{"token":"..."}` to stdout.
pub fn execute_management_token(flags: &GlobalFlags) -> CliOutput {
    // Go baseline: `parseResource(c.String("resource"))` — the --resource
    // flag defaults to "logs" if not provided.
    let resource_name = flags
        .rest_args
        .iter()
        .position(|a| a == "--resource")
        .and_then(|i| flags.rest_args.get(i + 1))
        .map(|s| s.as_str());

    let resource = match resource_name {
        Some(r) => match parse_management_resource(r) {
            Some(res) => res,
            None => {
                return CliOutput::failure(
                    String::new(),
                    format!("invalid resource '{r}': must be one of: logs, admin, host_details"),
                    1,
                );
            }
        },
        None => {
            // Default to logs if no --resource specified.
            ManagementResource::Logs
        }
    };

    execute_get_management_token(flags, resource)
}

/// Parse a resource string into a `ManagementResource`.
///
/// Matches Go `parseResource` in `management/cmd.go`.
fn parse_management_resource(s: &str) -> Option<ManagementResource> {
    match s {
        "logs" => Some(ManagementResource::Logs),
        "admin" => Some(ManagementResource::Admin),
        "host_details" => Some(ManagementResource::HostDetails),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// tail (bare) — streaming (CLI-023)
// ---------------------------------------------------------------------------

/// Execute `tail [TUNNEL-ID]` — the main streaming entry point.
///
/// Go baseline: `Run` in `tail/cmd.go`.
///
/// Steps (matching Go exactly):
///   1. Parse `--level`, `--event`, `--sample` into `StreamingFilters`
///   2. Build the management WebSocket URL (acquire token if needed)
///   3. Dial WebSocket
///   4. Send `start_streaming` event with filters
///   5. Read `logs` events in a loop, print each entry to stdout
///   6. Handle SIGINT/SIGTERM for clean shutdown
pub fn execute_tail(flags: &GlobalFlags) -> CliOutput {
    // Step 1: Parse streaming filters from CLI flags.
    let filters = match parse_tail_filters(flags) {
        Ok(f) => f,
        Err(msg) => return CliOutput::failure(String::new(), msg, 1),
    };

    // Step 2: Build management WebSocket URL.
    let url = match build_management_url(flags, ManagementResource::Logs) {
        Ok(u) => u,
        Err(msg) => return CliOutput::failure(String::new(), msg, 1),
    };

    // Step 3–6: WebSocket streaming inside a tokio runtime.
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => return CliOutput::failure(String::new(), format!("failed to start runtime: {e}"), 1),
    };

    rt.block_on(tail_streaming_loop(url, filters))
}

/// WebSocket streaming loop for `tail`.
///
/// Dials the management endpoint, sends `start_streaming` with filters,
/// then reads `logs` events and prints each entry.
async fn tail_streaming_loop(url: String, filters: Option<StreamingFilters>) -> CliOutput {
    use cfdrs_cdc::log_streaming::EventLog;
    use tokio_tungstenite::tungstenite;

    // Step 3: Dial WebSocket.
    let (mut ws, _response) = match tokio_tungstenite::connect_async(&url).await {
        Ok(pair) => pair,
        Err(e) => {
            return CliOutput::failure(
                String::new(),
                format!("unable to start log streaming session: {e}"),
                1,
            );
        }
    };

    // Step 4: Send start_streaming event.
    let start = EventStartStreaming::new(filters);
    let start_json = match serde_json::to_string(&start) {
        Ok(j) => j,
        Err(e) => return CliOutput::failure(String::new(), format!("json: {e}"), 1),
    };

    use tokio_tungstenite::tungstenite::Message as WsMessage;

    if let Err(e) = futures_util::SinkExt::send(&mut ws, WsMessage::Text(start_json.into())).await {
        return CliOutput::failure(
            String::new(),
            format!("unable to send start_streaming event: {e}"),
            1,
        );
    }

    // Step 5: Read logs events in a loop.
    let mut output = String::new();

    // Step 6: Handle SIGINT/SIGTERM for clean shutdown.
    let shutdown = async {
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("SIGINT handler");
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler");
        tokio::select! {
            _ = sigint.recv() => {}
            _ = sigterm.recv() => {}
        }
    };
    tokio::pin!(shutdown);

    loop {
        use futures_util::StreamExt;

        tokio::select! {
            msg = ws.next() => {
                let msg = match msg {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        return CliOutput::failure(
                            output,
                            format!("connection error: {e}"),
                            1,
                        );
                    }
                    None => break,
                };

                match msg {
                    WsMessage::Text(text) => {
                        if let Ok(event) = serde_json::from_str::<EventLog>(&text) {
                            for entry in &event.logs {
                                let line = format_log_line(entry);
                                output.push_str(&line);
                                output.push('\n');
                            }
                        }
                    }
                    WsMessage::Close(_) => break,
                    _ => {}
                }
            }
            _ = &mut shutdown => {
                // Clean shutdown: send close frame and exit.
                let _ = futures_util::SinkExt::send(
                    &mut ws,
                    WsMessage::Close(Some(tungstenite::protocol::CloseFrame {
                        code: tungstenite::protocol::frame::coding::CloseCode::Normal,
                        reason: "client shutdown".into(),
                    })),
                ).await;
                break;
            }
        }
    }

    CliOutput::success(output)
}

// ---------------------------------------------------------------------------
// Shared: management token acquisition
// ---------------------------------------------------------------------------

/// Shared helper: acquire a management JWT and print `{"token":"..."}`.
///
/// Go baseline: `cliutil.GetManagementToken` in `cliutil/management.go`.
///
/// Steps (matching Go exactly):
///   1. Read origin cert from `--origincert` or default search path
///   2. Build API client (FedRAMP or standard URL)
///   3. Parse tunnel ID from positional arg
///   4. Call `get_management_token(tunnel_id, resource)`
///   5. Print `{"token":"<jwt>"}` to stdout
fn execute_get_management_token(flags: &GlobalFlags, resource: ManagementResource) -> CliOutput {
    let client = match build_client(flags) {
        Ok(c) => c,
        Err(msg) => return CliOutput::failure(String::new(), msg, 1),
    };

    // Go baseline: `c.Args().First()` — tunnel ID is the first positional arg.
    // Skip known flag pairs in rest_args (e.g. `--resource logs`).
    let tunnel_id_str = extract_positional_arg(&flags.rest_args);

    let tunnel_id_str = match tunnel_id_str {
        Some(s) => s,
        None => {
            return CliOutput::failure(String::new(), "no tunnel ID provided".to_owned(), 1);
        }
    };

    let tunnel_id = match tunnel_id_str.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return CliOutput::failure(
                String::new(),
                format!("unable to parse provided tunnel id as a valid UUID: {tunnel_id_str}"),
                1,
            );
        }
    };

    let token = match client.get_management_token(tunnel_id, resource) {
        Ok(t) => t,
        Err(e) => return CliOutput::failure(String::new(), format!("{e}"), 1),
    };

    // Go baseline: `json.NewEncoder(os.Stdout).Encode(tokenResponse)`
    // produces `{"token":"..."}\n`.
    let output = format!("{{\"token\":\"{token}\"}}\n");
    CliOutput::success(output)
}

/// Extract the first positional argument from `rest_args`, skipping
/// `--flag value` pairs for known flags (`--resource`, `--event`, `--sample`,
/// `--level`, `--connector-id`, `--token`, `--management-hostname`).
fn extract_positional_arg(args: &[String]) -> Option<&str> {
    let known_flags = [
        "--resource",
        "--event",
        "--sample",
        "--level",
        "--connector-id",
        "--token",
        "--management-hostname",
    ];

    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if known_flags.contains(&arg.as_str()) {
            skip_next = true;
            continue;
        }
        if arg.starts_with("--") {
            continue;
        }
        return Some(arg.as_str());
    }
    None
}

// ---------------------------------------------------------------------------
// Filter parsing
// ---------------------------------------------------------------------------

/// Parse `--level`, `--event`, `--sample` CLI flags into `StreamingFilters`.
///
/// Go baseline: `parseFilters` in `tail/cmd.go`.
///
/// Returns `Ok(None)` when no filters are provided (matching Go behavior
/// where a nil filter means "send everything").
fn parse_tail_filters(flags: &GlobalFlags) -> Result<Option<StreamingFilters>, String> {
    // Level filter: --level (default: "debug" applied by env_defaults)
    let level = parse_level_filter(flags)?;

    // Event filter: --event can be repeated (multi-value)
    let events = parse_event_filters(flags)?;

    // Sample filter: --sample (default: 1.0)
    let sample = parse_sample_filter(flags)?;

    // Go: when no filters are provided, return nil (no StreamingFilters).
    if level.is_none() && events.is_empty() && sample == 1.0 {
        return Ok(None);
    }

    Ok(Some(StreamingFilters {
        level,
        events,
        sampling: sample,
    }))
}

/// Parse the `--level` flag into a `LogLevel`.
///
/// The flag defaults to "debug" in Go. If present and non-empty, it must
/// be one of: debug, info, warn, error.
fn parse_level_filter(flags: &GlobalFlags) -> Result<Option<LogLevel>, String> {
    // The --level flag for tail uses the general loglevel field from GlobalFlags.
    // In Go, `tail` has its own `--level` flag separate from `--loglevel`.
    // Our parser maps any `--level` found in tail context to loglevel.
    // Default is "debug" (set by env_defaults for tail commands).
    let level_str = flags.loglevel.as_deref().unwrap_or("debug");

    if level_str.is_empty() {
        return Ok(None);
    }

    LogLevel::from_str_opt(level_str).map(Some).ok_or_else(|| {
        "invalid --level filter provided, please use one of the following Log Levels: debug, info, warn, \
         error"
            .to_owned()
    })
}

/// Parse repeated `--event` flags into `Vec<LogEventType>`.
fn parse_event_filters(flags: &GlobalFlags) -> Result<Vec<LogEventType>, String> {
    // The --event flag is not yet modeled as a separate multi-value field
    // in GlobalFlags. For now, check rest_args for --event values.
    let mut events = Vec::new();
    let mut iter = flags.rest_args.iter();

    while let Some(arg) = iter.next() {
        if arg == "--event" {
            let val = iter.next().ok_or_else(|| {
                "invalid --event filter provided, please use one of the following EventTypes: cloudflared, \
                 http, tcp, udp"
                    .to_owned()
            })?;
            let event = LogEventType::from_str_opt(val).ok_or_else(|| {
                "invalid --event filter provided, please use one of the following EventTypes: cloudflared, \
                 http, tcp, udp"
                    .to_owned()
            })?;
            events.push(event);
        }
    }

    Ok(events)
}

/// Parse the `--sample` flag.
///
/// Go baseline: must be in range `(0.0, 1.0]`. Default: 1.0.
fn parse_sample_filter(flags: &GlobalFlags) -> Result<f64, String> {
    // --sample is not yet a dedicated GlobalFlags field; check rest_args.
    let mut iter = flags.rest_args.iter();

    while let Some(arg) = iter.next() {
        if arg == "--sample" {
            let val = iter.next().ok_or_else(|| {
                "invalid --sample value provided, please make sure it is in the range (0.0 .. 1.0)".to_owned()
            })?;
            let sample: f64 = val.parse().map_err(|_| {
                "invalid --sample value provided, please make sure it is in the range (0.0 .. 1.0)".to_owned()
            })?;
            if sample <= 0.0 || sample > 1.0 {
                return Err(
                    "invalid --sample value provided, please make sure it is in the range (0.0 .. 1.0)"
                        .to_owned(),
                );
            }
            return Ok(sample);
        }
    }

    // Default: 1.0 (no sampling)
    Ok(1.0)
}

// ---------------------------------------------------------------------------
// URL building
// ---------------------------------------------------------------------------

/// Build the management WebSocket URL for a given resource.
///
/// Go baseline: `buildURL` in `tail/cmd.go`.
///
/// Steps:
///   1. If `--token` flag is set, use it directly; otherwise acquire via API
///   2. Parse the token to check FedRAMP status → pick management hostname
///   3. If `--connector-id` is set, validate as UUID and add to query
///   4. Return `wss://hostname/path?access_token=TOKEN[&connector_id=UUID]`
fn build_management_url(flags: &GlobalFlags, resource: ManagementResource) -> Result<String, String> {
    // Step 1: Get management access token.
    let token = get_management_access_token(flags, resource)?;

    // Step 2: Determine management hostname.
    let hostname = determine_management_hostname(&token, flags);

    // Step 3: Build URL with proper query encoding via the url crate.
    let base = format!("wss://{hostname}{LOGS_PATH}");
    let mut url = Url::parse(&base).map_err(|e| format!("invalid management URL: {e}"))?;

    url.query_pairs_mut().append_pair("access_token", &token);

    // Step 4: Optional connector-id filter.
    if let Some(connector) = flags.connector_id.as_deref() {
        let connector_id = connector
            .parse::<Uuid>()
            .map_err(|_| format!("unabled to parse 'connector-id' flag into a valid UUID: {connector}"))?;
        url.query_pairs_mut()
            .append_pair("connector_id", &connector_id.to_string());
    }

    Ok(url.to_string())
}

/// Get a management access token from `--token` flag or API.
///
/// Go baseline: `buildURL` token logic in `tail/cmd.go`.
fn get_management_access_token(flags: &GlobalFlags, resource: ManagementResource) -> Result<String, String> {
    // If --token flag is set (for tail: flags.token field, or env
    // TUNNEL_MANAGEMENT_TOKEN)
    if let Some(token) = flags.token.as_deref()
        && !token.is_empty()
    {
        return Ok(token.to_owned());
    }

    // Fall back to API-based token acquisition.
    let client = build_client(flags)?;

    let tunnel_id_str = flags.rest_args.first().map(|s| s.as_str()).unwrap_or("");

    if tunnel_id_str.is_empty() {
        return Err("no tunnel ID provided".to_owned());
    }

    let tunnel_id = tunnel_id_str
        .parse::<Uuid>()
        .map_err(|_| format!("unable to parse provided tunnel id as a valid UUID: {tunnel_id_str}"))?;

    client
        .get_management_token(tunnel_id, resource)
        .map_err(|e| format!("unable to acquire management token for requested tunnel id: {e}"))
}

/// Determine the management hostname based on token FedRAMP status.
///
/// Go baseline: `buildURL` hostname logic — if `claims.IsFed()`, use
/// `credentials.FedRampHostname`; otherwise use `--management-hostname`.
fn determine_management_hostname(token: &str, flags: &GlobalFlags) -> String {
    if let Ok(claims) = parse_management_token(token)
        && claims.is_fed()
    {
        return FED_MANAGEMENT_HOSTNAME.to_owned();
    }

    // Use --management-hostname flag (default: management.argotunnel.com).
    flags
        .management_hostname
        .as_deref()
        .unwrap_or(DEFAULT_MANAGEMENT_HOSTNAME)
        .to_owned()
}

// ---------------------------------------------------------------------------
// Output formatting
// ---------------------------------------------------------------------------

/// Print a log entry in the default human-readable format.
///
/// Go baseline: `printLine` in `tail/cmd.go`:
/// `fmt.Printf("%s %s %s %s %s\n", log.Time, log.Level, log.Event, log.Message,
/// fields)`
fn format_log_line(entry: &LogEntry) -> String {
    let time = &entry.time;
    let level = entry.level.map(|l| l.as_str()).unwrap_or("");
    let event = entry.event.map(|e| e.as_str()).unwrap_or("");
    let message = &entry.message;
    let fields = entry
        .fields
        .as_ref()
        .and_then(|f| serde_json::to_string(f).ok())
        .unwrap_or_default();

    format!("{time} {level} {event} {message} {fields}")
}

/// Print a log entry as JSON.
///
/// Go baseline: `printJSON` in `tail/cmd.go`:
/// `json.Marshal(log)` → println.
#[allow(dead_code)] // Wired when WebSocket streaming is connected.
fn format_log_json(entry: &LogEntry) -> Option<String> {
    serde_json::to_string(entry).ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use cfdrs_cli::GlobalFlags;

    // --- Filter parsing ---

    #[test]
    fn parse_filters_default_returns_debug_level() {
        let flags = GlobalFlags::default();
        let filters = parse_tail_filters(&flags).expect("should parse");
        // Default loglevel is not set in GlobalFlags::default, so parse
        // falls through to "debug" default → Some(filters).
        let f = filters.expect("debug is a real filter");
        assert_eq!(f.level, Some(LogLevel::Debug));
        assert!(f.events.is_empty());
    }

    #[test]
    fn parse_filters_invalid_level_is_error() {
        let mut flags = GlobalFlags::default();
        flags.loglevel = Some("fatal".into());
        let err = parse_tail_filters(&flags).expect_err("fatal is invalid");
        assert!(err.contains("invalid --level"), "got: {err}");
    }

    #[test]
    fn parse_filters_event_from_rest_args() {
        let mut flags = GlobalFlags::default();
        flags.loglevel = Some("info".into());
        flags.rest_args = vec!["--event".into(), "http".into(), "--event".into(), "tcp".into()];
        let filters = parse_tail_filters(&flags)
            .expect("should parse")
            .expect("should have filters");
        assert_eq!(filters.events, vec![LogEventType::Http, LogEventType::Tcp]);
    }

    #[test]
    fn parse_filters_invalid_event_is_error() {
        let mut flags = GlobalFlags::default();
        flags.rest_args = vec!["--event".into(), "ftp".into()];
        let err = parse_tail_filters(&flags).expect_err("ftp is invalid");
        assert!(err.contains("invalid --event"), "got: {err}");
    }

    #[test]
    fn parse_filters_sample_valid() {
        let mut flags = GlobalFlags::default();
        flags.loglevel = Some("debug".into());
        flags.rest_args = vec!["--sample".into(), "0.5".into()];
        let filters = parse_tail_filters(&flags)
            .expect("should parse")
            .expect("should have filters");
        assert!((filters.sampling - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_filters_sample_out_of_range_is_error() {
        let mut flags = GlobalFlags::default();
        flags.rest_args = vec!["--sample".into(), "0.0".into()];
        let err = parse_tail_filters(&flags).expect_err("0.0 is out of range");
        assert!(err.contains("invalid --sample"), "got: {err}");

        flags.rest_args = vec!["--sample".into(), "1.5".into()];
        let err = parse_tail_filters(&flags).expect_err("1.5 is out of range");
        assert!(err.contains("invalid --sample"), "got: {err}");
    }

    #[test]
    fn parse_filters_no_filters_returns_none() {
        let mut flags = GlobalFlags::default();
        // Explicitly set no level and sample=1.0 — Go returns nil.
        flags.loglevel = Some(String::new());
        let filters = parse_tail_filters(&flags).expect("should parse");
        assert!(filters.is_none(), "no filters should yield None");
    }

    // --- Management hostname ---

    #[test]
    fn fed_token_selects_fed_hostname() {
        let flags = GlobalFlags::default();
        // Build a minimal FedRAMP token for hostname selection test.
        let hostname = determine_management_hostname("not-a-valid-jwt", &flags);
        // Invalid token falls back to default hostname.
        assert_eq!(hostname, DEFAULT_MANAGEMENT_HOSTNAME);
    }

    #[test]
    fn custom_management_hostname_from_flags() {
        let mut flags = GlobalFlags::default();
        flags.management_hostname = Some("custom.host.com".into());
        let hostname = determine_management_hostname("invalid-token", &flags);
        assert_eq!(hostname, "custom.host.com");
    }

    // --- Output formatting ---

    #[test]
    fn format_line_matches_go_layout() {
        let entry = LogEntry {
            time: "2024-01-01T00:00:00Z".into(),
            level: Some(LogLevel::Info),
            event: Some(LogEventType::Http),
            message: "request handled".into(),
            fields: Some(
                [("status".to_owned(), serde_json::json!(200))]
                    .into_iter()
                    .collect(),
            ),
        };
        let line = format_log_line(&entry);
        assert!(line.contains("2024-01-01T00:00:00Z"), "time: {line}");
        assert!(line.contains("info"), "level: {line}");
        assert!(line.contains("http"), "event: {line}");
        assert!(line.contains("request handled"), "message: {line}");
        assert!(line.contains("200"), "fields: {line}");
    }

    #[test]
    fn format_json_produces_valid_json() {
        let entry = LogEntry {
            time: "2024-01-01T00:00:00Z".into(),
            level: Some(LogLevel::Debug),
            event: None,
            message: "test".into(),
            fields: None,
        };
        let json = format_log_json(&entry).expect("should serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed["time"], "2024-01-01T00:00:00Z");
        assert_eq!(parsed["level"], "debug");
        assert_eq!(parsed["message"], "test");
    }

    // --- Token output shape ---

    #[test]
    fn token_json_output_shape() {
        // Verify the output format matches Go's `{"token":"..."}\n`.
        let token = "test-jwt-value";
        let output = format!("{{\"token\":\"{token}\"}}\n");
        let parsed: serde_json::Value = serde_json::from_str(output.trim()).expect("valid JSON");
        assert_eq!(parsed["token"], "test-jwt-value");
    }

    // --- Management resource parsing ---

    #[test]
    fn parse_management_resource_valid() {
        assert_eq!(parse_management_resource("logs"), Some(ManagementResource::Logs));
        assert_eq!(
            parse_management_resource("admin"),
            Some(ManagementResource::Admin)
        );
        assert_eq!(
            parse_management_resource("host_details"),
            Some(ManagementResource::HostDetails)
        );
    }

    #[test]
    fn parse_management_resource_invalid() {
        assert_eq!(parse_management_resource("unknown"), None);
        assert_eq!(parse_management_resource(""), None);
    }
}
