use super::parse_args;
use crate::surface_contract;
use std::ffi::OsString;

fn parse(parts: &[&str]) -> crate::Cli {
    let args = std::iter::once(OsString::from(surface_contract::PROGRAM_NAME))
        .chain(parts.iter().map(OsString::from))
        .collect::<Vec<_>>();
    parse_args(args).expect("arguments should parse")
}

// -----------------------------------------------------------------------
// Subcommand-specific flags
// -----------------------------------------------------------------------

#[test]
fn list_output_format_flag() {
    // NOTE: --output is consumed by the global logging flag (log_format_output);
    // subcommand output format uses only the -o short alias in our flat parser.
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "list", "-o", "json"]);
    assert_eq!(cli.flags.output_format.as_deref(), Some("json"));
}

#[test]
fn list_output_format_short_alias() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "list", "-o", "yaml"]);
    assert_eq!(cli.flags.output_format.as_deref(), Some("yaml"));
}

#[test]
fn list_show_deleted_flag() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "list", "--show-deleted"]);
    assert!(cli.flags.show_deleted);
}

#[test]
fn list_show_deleted_short_alias() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "list", "-d"]);
    assert!(cli.flags.show_deleted);
}

#[test]
fn list_name_prefix_flag() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "list", "--name-prefix", "prod-"]);
    assert_eq!(cli.flags.name_prefix.as_deref(), Some("prod-"));
}

#[test]
fn list_name_prefix_short_alias() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "list", "-np", "dev-"]);
    assert_eq!(cli.flags.name_prefix.as_deref(), Some("dev-"));
}

#[test]
fn list_exclude_name_prefix_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "list",
        "--exclude-name-prefix",
        "test-",
    ]);
    assert_eq!(cli.flags.exclude_name_prefix.as_deref(), Some("test-"));
}

#[test]
fn list_exclude_name_prefix_short_alias() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "list", "-enp", "tmp-"]);
    assert_eq!(cli.flags.exclude_name_prefix.as_deref(), Some("tmp-"));
}

#[test]
fn list_filter_when_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "list",
        "--when",
        "2024-01-01T00:00:00Z",
    ]);
    assert_eq!(cli.flags.filter_when.as_deref(), Some("2024-01-01T00:00:00Z"));
}

#[test]
fn list_filter_id_flag() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "list", "--id", "abc-123"]);
    assert_eq!(cli.flags.tunnel_id.as_deref(), Some("abc-123"));
}

#[test]
fn list_filter_id_short_alias() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "list", "-i", "def-456"]);
    assert_eq!(cli.flags.tunnel_id.as_deref(), Some("def-456"));
}

#[test]
fn list_show_recently_disconnected_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "list",
        "--show-recently-disconnected",
    ]);
    assert!(cli.flags.show_recently_disconnected);
}

#[test]
fn list_show_recently_disconnected_short_alias() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "list", "-rd"]);
    assert!(cli.flags.show_recently_disconnected);
}

#[test]
fn list_sort_by_flag() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "list", "--sort-by", "createdAt"]);
    assert_eq!(cli.flags.sort_by.as_deref(), Some("createdAt"));
}

#[test]
fn list_invert_sort_flag() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "list", "--invert-sort"]);
    assert!(cli.flags.invert_sort);
}

#[test]
fn create_secret_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "create",
        "--secret",
        "base64secret",
    ]);
    assert_eq!(cli.flags.tunnel_secret.as_deref(), Some("base64secret"));
}

#[test]
fn delete_force_flag() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "delete", "--force"]);
    assert!(cli.flags.force);
}

#[test]
fn delete_force_short_alias() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "delete", "-f"]);
    assert!(cli.flags.force);
}

#[test]
fn cleanup_connector_id_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "cleanup",
        "--connector-id",
        "uuid-123",
    ]);
    assert_eq!(cli.flags.connector_id.as_deref(), Some("uuid-123"));
}

#[test]
fn cleanup_connector_id_short_alias() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "cleanup", "-c", "uuid-456"]);
    assert_eq!(cli.flags.connector_id.as_deref(), Some("uuid-456"));
}

#[test]
fn login_fedramp_flag() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "login", "--fedramp"]);
    assert!(cli.flags.fedramp);
}

#[test]
fn login_url_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "login",
        "--loginURL",
        "https://custom.example.com",
    ]);
    assert_eq!(cli.flags.login_url.as_deref(), Some("https://custom.example.com"));
}

#[test]
fn login_callback_url_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "login",
        "--callbackURL",
        "https://callback.example.com",
    ]);
    assert_eq!(
        cli.flags.callback_url.as_deref(),
        Some("https://callback.example.com")
    );
}

#[test]
fn route_dns_overwrite_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "route",
        "dns",
        "--overwrite-dns",
    ]);
    assert!(cli.flags.overwrite_dns);
}

#[test]
fn route_ip_vnet_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "route",
        "ip",
        "add",
        "--vnet",
        "myvnet",
    ]);
    assert_eq!(cli.flags.vnet_id.as_deref(), Some("myvnet"));
}

#[test]
fn route_ip_vnet_short_alias() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "route",
        "ip",
        "delete",
        "-vn",
        "othervnet",
    ]);
    assert_eq!(cli.flags.vnet_id.as_deref(), Some("othervnet"));
}

#[test]
fn route_ip_show_filter_flags() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "route",
        "ip",
        "show",
        "--filter-is-deleted",
        "--filter-tunnel-id",
        "tid-1",
        "--filter-comment-is",
        "prod route",
        "-o",
        "json",
    ]);
    assert!(cli.flags.filter_is_deleted);
    assert_eq!(cli.flags.filter_tunnel_id.as_deref(), Some("tid-1"));
    assert_eq!(cli.flags.filter_comment_is.as_deref(), Some("prod route"));
    assert_eq!(cli.flags.output_format.as_deref(), Some("json"));
}

#[test]
fn route_ip_show_network_subset_filter() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "route",
        "ip",
        "show",
        "-nsub",
        "10.0.0.0/8",
    ]);
    assert_eq!(cli.flags.filter_network_subset.as_deref(), Some("10.0.0.0/8"));
}

#[test]
fn route_ip_show_network_superset_filter() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "route",
        "ip",
        "show",
        "-nsup",
        "192.168.0.0/16",
    ]);
    assert_eq!(
        cli.flags.filter_network_superset.as_deref(),
        Some("192.168.0.0/16")
    );
}

#[test]
fn vnet_add_default_flag() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "vnet", "add", "--default"]);
    assert!(cli.flags.vnet_default);
}

#[test]
fn vnet_delete_force_flag() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "vnet", "delete", "--force"]);
    assert!(cli.flags.force);
}

#[test]
fn vnet_update_comment_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "vnet",
        "update",
        "--comment",
        "new comment",
    ]);
    assert_eq!(cli.flags.vnet_comment.as_deref(), Some("new comment"));
}

#[test]
fn vnet_list_is_default_filter() {
    let cli = parse(&[surface_contract::TUNNEL_COMMAND, "vnet", "list", "--is-default"]);
    assert!(cli.flags.vnet_is_default_filter);
}

#[test]
fn ingress_validate_json_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "ingress",
        "validate",
        "--json",
        "{\"ingress\":[]}",
    ]);
    assert_eq!(cli.flags.ingress_json.as_deref(), Some("{\"ingress\":[]}"));
}

#[test]
fn ingress_validate_json_short_alias() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "ingress",
        "validate",
        "-j",
        "{}",
    ]);
    assert_eq!(cli.flags.ingress_json.as_deref(), Some("{}"));
}

#[test]
fn diag_container_id_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "diag",
        "--diag-container-id",
        "abc123",
    ]);
    assert_eq!(cli.flags.diag_container_id.as_deref(), Some("abc123"));
}

#[test]
fn diag_pod_id_flag() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "diag",
        "--diag-pod-id",
        "pod-xyz",
    ]);
    assert_eq!(cli.flags.diag_pod_id.as_deref(), Some("pod-xyz"));
}

#[test]
fn diag_exclusion_flags() {
    let cli = parse(&[
        surface_contract::TUNNEL_COMMAND,
        "diag",
        "--no-diag-logs",
        "--no-diag-metrics",
        "--no-diag-system",
        "--no-diag-runtime",
        "--no-diag-network",
    ]);
    assert!(cli.flags.no_diag_logs);
    assert!(cli.flags.no_diag_metrics);
    assert!(cli.flags.no_diag_system);
    assert!(cli.flags.no_diag_runtime);
    assert!(cli.flags.no_diag_network);
}
