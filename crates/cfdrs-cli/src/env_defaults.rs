//! Environment-variable fallback and baseline defaults for [`GlobalFlags`].
//!
//! Go baseline: `urfave/cli` reads `EnvVars` bindings during flag setup.
//! This module provides the same fallback chain: CLI args > env vars >
//! defaults.
//!
//! The env-reading layer accepts a `reader` closure so tests can inject
//! mock lookups without `unsafe` `std::env::set_var`.

use std::path::PathBuf;

use crate::types::GlobalFlags;

// ---------------------------------------------------------------------------
// Env-reading helpers — parameterized by a reader closure
// ---------------------------------------------------------------------------

/// Parse a boolean value using Go's `strconv.ParseBool` rules.
fn parse_go_bool(val: &str) -> Option<bool> {
    match val {
        "1" | "t" | "T" | "TRUE" | "true" | "True" => Some(true),
        "0" | "f" | "F" | "FALSE" | "false" | "False" => Some(false),
        _ => None,
    }
}

/// Fill an `Option<String>` from an env var if the field is still `None`.
fn env_string(reader: &impl Fn(&str) -> Option<String>, name: &str, field: &mut Option<String>) {
    if field.is_none()
        && let Some(val) = reader(name)
    {
        *field = Some(val);
    }
}

/// Fill an `Option<String>` from the first set env var in a list.
///
/// Go baseline: some flags list multiple `EnvVars` — the first match wins.
fn env_string_first(reader: &impl Fn(&str) -> Option<String>, names: &[&str], field: &mut Option<String>) {
    if field.is_none() {
        for name in names {
            if let Some(val) = reader(name) {
                *field = Some(val);
                return;
            }
        }
    }
}

/// Fill an `Option<PathBuf>` from an env var if the field is still `None`.
fn env_path(reader: &impl Fn(&str) -> Option<String>, name: &str, field: &mut Option<PathBuf>) {
    if field.is_none()
        && let Some(val) = reader(name)
    {
        *field = Some(PathBuf::from(val));
    }
}

/// Set a bare `bool` flag from an env var if the flag is still `false`.
///
/// Go: `BoolFlag` env var values are parsed with `strconv.ParseBool`.
/// Invalid values are silently ignored.
fn env_bool_flag(reader: &impl Fn(&str) -> Option<String>, name: &str, field: &mut bool) {
    if !*field
        && let Some(val) = reader(name)
        && let Some(b) = parse_go_bool(&val)
    {
        *field = b;
    }
}

/// Fill an `Option<bool>` from an env var if the field is still `None`.
fn env_opt_bool(reader: &impl Fn(&str) -> Option<String>, name: &str, field: &mut Option<bool>) {
    if field.is_none()
        && let Some(val) = reader(name)
        && let Some(b) = parse_go_bool(&val)
    {
        *field = Some(b);
    }
}

/// Fill an `Option<u16>` from an env var if the field is still `None`.
fn env_u16(reader: &impl Fn(&str) -> Option<String>, name: &str, field: &mut Option<u16>) {
    if field.is_none()
        && let Some(val) = reader(name)
        && let Ok(n) = val.parse::<u16>()
    {
        *field = Some(n);
    }
}

/// Fill an `Option<u32>` from an env var if the field is still `None`.
fn env_u32(reader: &impl Fn(&str) -> Option<String>, name: &str, field: &mut Option<u32>) {
    if field.is_none()
        && let Some(val) = reader(name)
        && let Ok(n) = val.parse::<u32>()
    {
        *field = Some(n);
    }
}

/// Fill an `Option<u64>` from an env var if the field is still `None`.
fn env_u64(reader: &impl Fn(&str) -> Option<String>, name: &str, field: &mut Option<u64>) {
    if field.is_none()
        && let Some(val) = reader(name)
        && let Ok(n) = val.parse::<u64>()
    {
        *field = Some(n);
    }
}

/// Fill a `Vec<String>` from a comma-separated env var if the vec is empty.
///
/// Go baseline: `StringSliceFlag` env vars are split on `,`.
fn env_vec_csv(reader: &impl Fn(&str) -> Option<String>, name: &str, field: &mut Vec<String>) {
    if field.is_empty()
        && let Some(val) = reader(name)
        && !val.is_empty()
    {
        *field = val.split(',').map(|s| s.trim().to_owned()).collect();
    }
}

/// Set an `Option<T>` to a default value if it is still `None`.
fn set_default<T>(field: &mut Option<T>, default: T) {
    if field.is_none() {
        *field = Some(default);
    }
}

/// Set an `Option<String>` to a default `&str` if it is still `None`.
fn set_default_string(field: &mut Option<String>, default: &str) {
    if field.is_none() {
        *field = Some(default.to_owned());
    }
}

// ---------------------------------------------------------------------------
// GlobalFlags env + default methods
// ---------------------------------------------------------------------------

impl GlobalFlags {
    const DEFAULT_API_URL: &str = "https://api.cloudflare.com/client/v4";
    const DEFAULT_EDGE_IP_VERSION: &str = "4";
    const DEFAULT_GRACE_PERIOD: &str = "30s";
    // --- Go baseline default constants ---
    // Sources: cmd/cloudflared/tunnel/cmd.go, cliutil/logger.go

    const DEFAULT_LOGLEVEL: &str = "info";
    const DEFAULT_LOG_FORMAT: &str = "default";
    const DEFAULT_MANAGEMENT_HOSTNAME: &str = "management.argotunnel.com";
    const DEFAULT_METRICS_UPDATE_FREQ: &str = "5s";
    const DEFAULT_PROXY_ADDRESS: &str = "127.0.0.1";
    /// 30 MB — Go baseline: `quicConnLevelFlowControlLimit`
    const DEFAULT_QUIC_CONN_FLOW_CONTROL: u64 = 31_457_280;
    /// 6 MB — Go baseline: `quicStreamLevelFlowControlLimit`
    const DEFAULT_QUIC_STREAM_FLOW_CONTROL: u64 = 6_291_456;
    const DEFAULT_RETRIES: u32 = 5;
    const DEFAULT_SERVICE_OP_IP: &str = "198.41.200.113:80";

    /// Read environment variables as fallbacks for fields not already set by
    /// CLI args.
    ///
    /// Matches the frozen Go baseline `EnvVars` binding on each flag.
    /// CLI args always take precedence over environment variables.
    ///
    /// # Sources
    ///
    /// - `cmd/cloudflared/tunnel/cmd.go` `configureCloudflaredFlags()`,
    ///   `tunnelFlags()`, `configureProxyFlags()`
    /// - `cmd/cloudflared/cliutil/logger.go` `ConfigureLoggingFlags()`
    /// - `cmd/cloudflared/tunnel/subcommands.go` credential and subcommand
    ///   flags
    pub fn apply_env_defaults(&mut self) {
        self.apply_env_with(&|name| std::env::var(name).ok());
    }

    /// Apply env-var fallbacks using a provided lookup function.
    ///
    /// Production callers use [`apply_env_defaults`](Self::apply_env_defaults).
    /// Tests pass a mock reader to avoid `unsafe` `std::env::set_var`.
    fn apply_env_with(&mut self, reader: &impl Fn(&str) -> Option<String>) {
        // --- Config and credentials ---
        env_path(reader, "TUNNEL_ORIGIN_CERT", &mut self.origincert);
        env_path(reader, "TUNNEL_CRED_FILE", &mut self.credentials_file);
        env_string(reader, "TUNNEL_CRED_CONTENTS", &mut self.credentials_contents);
        env_string(reader, "TUNNEL_TOKEN", &mut self.token);
        env_path(reader, "TUNNEL_TOKEN_FILE", &mut self.token_file);

        // --- Logging ---
        env_string(reader, "TUNNEL_LOGLEVEL", &mut self.loglevel);
        env_string_first(
            reader,
            &["TUNNEL_PROTO_LOGLEVEL", "TUNNEL_TRANSPORT_LOGLEVEL"],
            &mut self.transport_loglevel,
        );
        env_path(reader, "TUNNEL_LOGFILE", &mut self.logfile);
        env_path(reader, "TUNNEL_LOGDIRECTORY", &mut self.log_directory);
        env_string_first(
            reader,
            &["TUNNEL_MANAGEMENT_OUTPUT", "TUNNEL_LOG_OUTPUT"],
            &mut self.log_format_output,
        );
        env_string(reader, "TUNNEL_TRACE_OUTPUT", &mut self.trace_output);

        // --- Process ---
        env_bool_flag(reader, "NO_AUTOUPDATE", &mut self.no_autoupdate);
        env_string(reader, "TUNNEL_METRICS", &mut self.metrics);
        env_path(reader, "TUNNEL_PIDFILE", &mut self.pidfile);

        // --- Edge connection ---
        env_vec_csv(reader, "TUNNEL_EDGE", &mut self.edge);
        env_string(reader, "TUNNEL_REGION", &mut self.region);
        env_string(reader, "TUNNEL_EDGE_IP_VERSION", &mut self.edge_ip_version);
        env_string(reader, "TUNNEL_EDGE_BIND_ADDRESS", &mut self.edge_bind_address);

        // --- Tunnel identity ---
        env_string(reader, "TUNNEL_NAME", &mut self.tunnel_name);
        env_string(reader, "TUNNEL_HOSTNAME", &mut self.hostname);
        env_string(reader, "TUNNEL_ID", &mut self.tunnel_id);
        env_string(reader, "TUNNEL_LB_POOL", &mut self.lb_pool);
        env_vec_csv(reader, "TUNNEL_TAG", &mut self.tag);

        // --- Tunnel behavior ---
        env_string(reader, "TUNNEL_GRACE_PERIOD", &mut self.grace_period);
        env_string(reader, "TUNNEL_TRANSPORT_PROTOCOL", &mut self.protocol);
        env_u32(reader, "TUNNEL_RETRIES", &mut self.retries);
        env_string(reader, "TUNNEL_URL", &mut self.url);
        env_bool_flag(reader, "TUNNEL_HELLO_WORLD", &mut self.hello_world);
        env_opt_bool(reader, "TUNNEL_POST_QUANTUM", &mut self.post_quantum);
        env_opt_bool(
            reader,
            "TUNNEL_MANAGEMENT_DIAGNOSTICS",
            &mut self.management_diagnostics,
        );
        env_string(
            reader,
            "TUNNEL_MANAGEMENT_HOSTNAME",
            &mut self.management_hostname,
        );
        env_string(reader, "TUNNEL_API_URL", &mut self.api_url);
        env_string(
            reader,
            "TUNNEL_METRICS_UPDATE_FREQ",
            &mut self.metrics_update_freq,
        );
        env_string(
            reader,
            "TUNNEL_STREAM_WRITE_TIMEOUT",
            &mut self.write_stream_timeout,
        );
        env_bool_flag(reader, "TUNNEL_DISABLE_QUIC_PMTU", &mut self.quic_disable_pmtu);
        env_u64(
            reader,
            "TUNNEL_QUIC_CONN_LEVEL_FLOW_CONTROL_LIMIT",
            &mut self.quic_conn_flow_control,
        );
        env_u64(
            reader,
            "TUNNEL_QUIC_STREAM_LEVEL_FLOW_CONTROL_LIMIT",
            &mut self.quic_stream_flow_control,
        );

        // --- Proxy / origin ---
        env_string(reader, "TUNNEL_UNIX_SOCKET", &mut self.unix_socket);
        env_string(reader, "TUNNEL_HTTP_HOST_HEADER", &mut self.http_host_header);
        env_string(reader, "TUNNEL_ORIGIN_SERVER_NAME", &mut self.origin_server_name);
        env_string(reader, "TUNNEL_ORIGIN_CA_POOL", &mut self.origin_ca_pool);
        env_bool_flag(reader, "NO_TLS_VERIFY", &mut self.no_tls_verify);
        env_bool_flag(
            reader,
            "TUNNEL_NO_CHUNKED_ENCODING",
            &mut self.no_chunked_encoding,
        );
        env_bool_flag(reader, "TUNNEL_ORIGIN_ENABLE_HTTP2", &mut self.http2_origin);
        env_bool_flag(reader, "TUNNEL_BASTION", &mut self.bastion);
        env_bool_flag(reader, "TUNNEL_SOCKS", &mut self.socks5);
        env_string(reader, "TUNNEL_PROXY_ADDRESS", &mut self.proxy_address);
        env_u16(reader, "TUNNEL_PROXY_PORT", &mut self.proxy_port);
        env_string(reader, "TUNNEL_SERVICE_OP_IP", &mut self.service_op_ip);

        // --- ICMP ---
        env_string(reader, "TUNNEL_ICMPV4_SRC", &mut self.icmpv4_src);
        env_string(reader, "TUNNEL_ICMPV6_SRC", &mut self.icmpv6_src);
        env_u64(reader, "TUNNEL_MAX_ACTIVE_FLOWS", &mut self.max_active_flows);

        // --- Deprecated API flags ---
        env_string(reader, "TUNNEL_API_KEY", &mut self.api_key);
        env_string(reader, "TUNNEL_API_EMAIL", &mut self.api_email);
        env_string(reader, "TUNNEL_API_CA_KEY", &mut self.api_ca_key);
    }

    /// Apply frozen Go baseline default values for fields not set by CLI args
    /// or environment variables.
    ///
    /// Call this after [`apply_env_defaults`](Self::apply_env_defaults) so the
    /// precedence chain is: CLI args > env vars > baseline defaults.
    ///
    /// # Sources
    ///
    /// - `cmd/cloudflared/tunnel/cmd.go` `Flags()` `Value` fields
    /// - `cmd/cloudflared/cliutil/logger.go` `ConfigureLoggingFlags()`
    pub fn apply_defaults(&mut self) {
        // Logging defaults
        set_default_string(&mut self.loglevel, Self::DEFAULT_LOGLEVEL);
        set_default_string(&mut self.transport_loglevel, Self::DEFAULT_LOGLEVEL);
        set_default_string(&mut self.log_format_output, Self::DEFAULT_LOG_FORMAT);

        // Connection defaults
        set_default_string(&mut self.edge_ip_version, Self::DEFAULT_EDGE_IP_VERSION);
        set_default(&mut self.retries, Self::DEFAULT_RETRIES);
        set_default_string(&mut self.grace_period, Self::DEFAULT_GRACE_PERIOD);
        set_default_string(&mut self.metrics_update_freq, Self::DEFAULT_METRICS_UPDATE_FREQ);

        // QUIC flow control
        set_default(
            &mut self.quic_conn_flow_control,
            Self::DEFAULT_QUIC_CONN_FLOW_CONTROL,
        );
        set_default(
            &mut self.quic_stream_flow_control,
            Self::DEFAULT_QUIC_STREAM_FLOW_CONTROL,
        );

        // Management and API
        set_default_string(&mut self.management_hostname, Self::DEFAULT_MANAGEMENT_HOSTNAME);
        set_default_string(&mut self.api_url, Self::DEFAULT_API_URL);
        set_default(&mut self.management_diagnostics, true);
        set_default(&mut self.post_quantum, false);

        // Proxy defaults
        // Note: `url` is intentionally NOT defaulted here.  The tunnel
        // dispatch logic checks `url.is_some()` (Go: `c.IsSet("url")`)
        // to detect explicit user intent.  The DEFAULT_ORIGIN_URL
        // constant is available for point-of-use fallback.
        set_default_string(&mut self.proxy_address, Self::DEFAULT_PROXY_ADDRESS);
        set_default_string(&mut self.service_op_ip, Self::DEFAULT_SERVICE_OP_IP);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Build a mock env reader from key-value pairs.
    fn mock_env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |name: &str| map.get(name).cloned()
    }

    // --- parse_go_bool ---

    #[test]
    fn parse_go_bool_truthy_values() {
        for val in &["1", "t", "T", "TRUE", "true", "True"] {
            assert_eq!(parse_go_bool(val), Some(true), "expected true for {val:?}");
        }
    }

    #[test]
    fn parse_go_bool_falsy_values() {
        for val in &["0", "f", "F", "FALSE", "false", "False"] {
            assert_eq!(parse_go_bool(val), Some(false), "expected false for {val:?}");
        }
    }

    #[test]
    fn parse_go_bool_invalid_returns_none() {
        assert_eq!(parse_go_bool("yes"), None);
        assert_eq!(parse_go_bool("no"), None);
        assert_eq!(parse_go_bool(""), None);
        assert_eq!(parse_go_bool("2"), None);
    }

    // --- env_string ---

    #[test]
    fn env_string_fills_none() {
        let reader = mock_env(&[("MY_VAR", "hello")]);
        let mut field: Option<String> = None;
        env_string(&reader, "MY_VAR", &mut field);
        assert_eq!(field, Some("hello".to_owned()));
    }

    #[test]
    fn env_string_preserves_cli_value() {
        let reader = mock_env(&[("MY_VAR", "from-env")]);
        let mut field = Some("from-cli".to_owned());
        env_string(&reader, "MY_VAR", &mut field);
        assert_eq!(field, Some("from-cli".to_owned()));
    }

    #[test]
    fn env_string_skips_when_unset() {
        let reader = mock_env(&[]);
        let mut field: Option<String> = None;
        env_string(&reader, "MY_VAR", &mut field);
        assert_eq!(field, None);
    }

    // --- env_string_first ---

    #[test]
    fn env_string_first_uses_first_match() {
        let reader = mock_env(&[("A", "alpha"), ("B", "beta")]);
        let mut field: Option<String> = None;
        env_string_first(&reader, &["A", "B"], &mut field);
        assert_eq!(field, Some("alpha".to_owned()));
    }

    #[test]
    fn env_string_first_falls_through_to_second() {
        let reader = mock_env(&[("B", "beta")]);
        let mut field: Option<String> = None;
        env_string_first(&reader, &["A", "B"], &mut field);
        assert_eq!(field, Some("beta".to_owned()));
    }

    // --- env_path ---

    #[test]
    fn env_path_fills_none() {
        let reader = mock_env(&[("CERT", "/tmp/cert.pem")]);
        let mut field: Option<PathBuf> = None;
        env_path(&reader, "CERT", &mut field);
        assert_eq!(field, Some(PathBuf::from("/tmp/cert.pem")));
    }

    // --- env_bool_flag ---

    #[test]
    fn env_bool_flag_sets_true() {
        let reader = mock_env(&[("FLAG", "true")]);
        let mut field = false;
        env_bool_flag(&reader, "FLAG", &mut field);
        assert!(field);
    }

    #[test]
    fn env_bool_flag_preserves_cli_true() {
        let reader = mock_env(&[("FLAG", "false")]);
        let mut field = true;
        env_bool_flag(&reader, "FLAG", &mut field);
        assert!(field, "CLI true should not be overridden by env false");
    }

    #[test]
    fn env_bool_flag_ignores_invalid() {
        let reader = mock_env(&[("FLAG", "maybe")]);
        let mut field = false;
        env_bool_flag(&reader, "FLAG", &mut field);
        assert!(!field, "invalid bool string should leave field unchanged");
    }

    // --- env_opt_bool ---

    #[test]
    fn env_opt_bool_fills_none() {
        let reader = mock_env(&[("FLAG", "1")]);
        let mut field: Option<bool> = None;
        env_opt_bool(&reader, "FLAG", &mut field);
        assert_eq!(field, Some(true));
    }

    // --- env_u32 ---

    #[test]
    fn env_u32_fills_none() {
        let reader = mock_env(&[("N", "42")]);
        let mut field: Option<u32> = None;
        env_u32(&reader, "N", &mut field);
        assert_eq!(field, Some(42));
    }

    #[test]
    fn env_u32_ignores_bad_parse() {
        let reader = mock_env(&[("N", "not-a-number")]);
        let mut field: Option<u32> = None;
        env_u32(&reader, "N", &mut field);
        assert_eq!(field, None);
    }

    // --- env_u64 ---

    #[test]
    fn env_u64_fills_none() {
        let reader = mock_env(&[("N", "31457280")]);
        let mut field: Option<u64> = None;
        env_u64(&reader, "N", &mut field);
        assert_eq!(field, Some(31_457_280));
    }

    // --- env_u16 ---

    #[test]
    fn env_u16_fills_none() {
        let reader = mock_env(&[("P", "8080")]);
        let mut field: Option<u16> = None;
        env_u16(&reader, "P", &mut field);
        assert_eq!(field, Some(8080));
    }

    // --- env_vec_csv ---

    #[test]
    fn env_vec_csv_splits_on_comma() {
        let reader = mock_env(&[("LIST", "a, b, c")]);
        let mut field: Vec<String> = Vec::new();
        env_vec_csv(&reader, "LIST", &mut field);
        assert_eq!(field, vec!["a", "b", "c"]);
    }

    #[test]
    fn env_vec_csv_preserves_nonempty() {
        let reader = mock_env(&[("LIST", "from-env")]);
        let mut field = vec!["from-cli".to_owned()];
        env_vec_csv(&reader, "LIST", &mut field);
        assert_eq!(field, vec!["from-cli"]);
    }

    #[test]
    fn env_vec_csv_skips_empty_string() {
        let reader = mock_env(&[("LIST", "")]);
        let mut field: Vec<String> = Vec::new();
        env_vec_csv(&reader, "LIST", &mut field);
        assert!(field.is_empty());
    }

    // --- apply_env_with integration ---

    #[test]
    fn apply_env_with_fills_loglevel() {
        let reader = mock_env(&[("TUNNEL_LOGLEVEL", "debug")]);
        let mut flags = GlobalFlags::default();
        flags.apply_env_with(&reader);
        assert_eq!(flags.loglevel, Some("debug".to_owned()));
    }

    #[test]
    fn apply_env_with_cli_overrides_env() {
        let reader = mock_env(&[("TUNNEL_LOGLEVEL", "debug")]);
        let mut flags = GlobalFlags {
            loglevel: Some("warn".to_owned()),
            ..GlobalFlags::default()
        };
        flags.apply_env_with(&reader);
        assert_eq!(flags.loglevel, Some("warn".to_owned()));
    }

    #[test]
    fn apply_env_with_multi_env_transport_loglevel() {
        let reader = mock_env(&[("TUNNEL_TRANSPORT_LOGLEVEL", "error")]);
        let mut flags = GlobalFlags::default();
        flags.apply_env_with(&reader);
        assert_eq!(flags.transport_loglevel, Some("error".to_owned()));
    }

    #[test]
    fn apply_env_with_multi_env_first_wins() {
        let reader = mock_env(&[
            ("TUNNEL_PROTO_LOGLEVEL", "warn"),
            ("TUNNEL_TRANSPORT_LOGLEVEL", "error"),
        ]);
        let mut flags = GlobalFlags::default();
        flags.apply_env_with(&reader);
        assert_eq!(flags.transport_loglevel, Some("warn".to_owned()));
    }

    #[test]
    fn apply_env_with_bool_no_autoupdate() {
        let reader = mock_env(&[("NO_AUTOUPDATE", "true")]);
        let mut flags = GlobalFlags::default();
        flags.apply_env_with(&reader);
        assert!(flags.no_autoupdate);
    }

    #[test]
    fn apply_env_with_token_from_env() {
        let reader = mock_env(&[("TUNNEL_TOKEN", "eyJhIjoiYWNjdC...")]);
        let mut flags = GlobalFlags::default();
        flags.apply_env_with(&reader);
        assert_eq!(flags.token, Some("eyJhIjoiYWNjdC...".to_owned()));
    }

    #[test]
    fn apply_env_with_edge_csv() {
        let reader = mock_env(&[("TUNNEL_EDGE", "10.0.0.1:7844, 10.0.0.2:7844")]);
        let mut flags = GlobalFlags::default();
        flags.apply_env_with(&reader);
        assert_eq!(flags.edge, vec!["10.0.0.1:7844", "10.0.0.2:7844"]);
    }

    #[test]
    fn apply_env_with_retries_u32() {
        let reader = mock_env(&[("TUNNEL_RETRIES", "10")]);
        let mut flags = GlobalFlags::default();
        flags.apply_env_with(&reader);
        assert_eq!(flags.retries, Some(10));
    }

    #[test]
    fn apply_env_with_quic_flow_control() {
        let reader = mock_env(&[
            ("TUNNEL_QUIC_CONN_LEVEL_FLOW_CONTROL_LIMIT", "1000000"),
            ("TUNNEL_QUIC_STREAM_LEVEL_FLOW_CONTROL_LIMIT", "500000"),
        ]);
        let mut flags = GlobalFlags::default();
        flags.apply_env_with(&reader);
        assert_eq!(flags.quic_conn_flow_control, Some(1_000_000));
        assert_eq!(flags.quic_stream_flow_control, Some(500_000));
    }

    #[test]
    fn apply_env_with_proxy_port() {
        let reader = mock_env(&[("TUNNEL_PROXY_PORT", "3128")]);
        let mut flags = GlobalFlags::default();
        flags.apply_env_with(&reader);
        assert_eq!(flags.proxy_port, Some(3128));
    }

    #[test]
    fn apply_env_with_opt_bool_post_quantum() {
        let reader = mock_env(&[("TUNNEL_POST_QUANTUM", "true")]);
        let mut flags = GlobalFlags::default();
        flags.apply_env_with(&reader);
        assert_eq!(flags.post_quantum, Some(true));
    }

    #[test]
    fn apply_env_with_credentials_paths() {
        let reader = mock_env(&[
            ("TUNNEL_ORIGIN_CERT", "/etc/cloudflared/cert.pem"),
            ("TUNNEL_CRED_FILE", "/etc/cloudflared/creds.json"),
        ]);
        let mut flags = GlobalFlags::default();
        flags.apply_env_with(&reader);
        assert_eq!(flags.origincert, Some(PathBuf::from("/etc/cloudflared/cert.pem")));
        assert_eq!(
            flags.credentials_file,
            Some(PathBuf::from("/etc/cloudflared/creds.json"))
        );
    }

    // --- apply_defaults ---

    #[test]
    fn apply_defaults_fills_baseline_values() {
        let mut flags = GlobalFlags::default();
        flags.apply_defaults();

        assert_eq!(flags.loglevel.as_deref(), Some("info"));
        assert_eq!(flags.transport_loglevel.as_deref(), Some("info"));
        assert_eq!(flags.log_format_output.as_deref(), Some("default"));
        assert_eq!(flags.edge_ip_version.as_deref(), Some("4"));
        assert_eq!(flags.retries, Some(5));
        assert_eq!(flags.grace_period.as_deref(), Some("30s"));
        assert_eq!(flags.metrics_update_freq.as_deref(), Some("5s"));
        assert_eq!(
            flags.management_hostname.as_deref(),
            Some("management.argotunnel.com")
        );
        assert_eq!(
            flags.api_url.as_deref(),
            Some("https://api.cloudflare.com/client/v4")
        );
        assert_eq!(flags.management_diagnostics, Some(true));
        assert_eq!(flags.post_quantum, Some(false));
        // `url` is intentionally NOT defaulted — dispatch checks is_some()
        assert_eq!(flags.url, None);
        assert_eq!(flags.proxy_address.as_deref(), Some("127.0.0.1"));
        assert_eq!(flags.service_op_ip.as_deref(), Some("198.41.200.113:80"));
        assert_eq!(flags.quic_conn_flow_control, Some(31_457_280));
        assert_eq!(flags.quic_stream_flow_control, Some(6_291_456));
    }

    #[test]
    fn apply_defaults_does_not_overwrite_existing() {
        let mut flags = GlobalFlags {
            loglevel: Some("warn".to_owned()),
            retries: Some(10),
            ..GlobalFlags::default()
        };
        flags.apply_defaults();

        assert_eq!(flags.loglevel.as_deref(), Some("warn"));
        assert_eq!(flags.retries, Some(10));
    }

    // --- full precedence chain ---

    #[test]
    fn full_precedence_cli_over_env_over_defaults() {
        let reader = mock_env(&[("TUNNEL_RETRIES", "3")]);
        let mut flags = GlobalFlags {
            retries: Some(7),
            ..GlobalFlags::default()
        };
        flags.apply_env_with(&reader);
        flags.apply_defaults();
        assert_eq!(flags.retries, Some(7), "CLI value should win");
    }

    #[test]
    fn full_precedence_env_over_defaults() {
        let reader = mock_env(&[("TUNNEL_RETRIES", "3")]);
        let mut flags = GlobalFlags::default();
        flags.apply_env_with(&reader);
        flags.apply_defaults();
        assert_eq!(flags.retries, Some(3), "env value should beat default");
    }

    #[test]
    fn full_precedence_defaults_when_nothing_set() {
        let reader = mock_env(&[]);
        let mut flags = GlobalFlags::default();
        flags.apply_env_with(&reader);
        flags.apply_defaults();
        assert_eq!(flags.retries, Some(5), "default should apply");
        assert_eq!(flags.loglevel.as_deref(), Some("info"));
    }
}
