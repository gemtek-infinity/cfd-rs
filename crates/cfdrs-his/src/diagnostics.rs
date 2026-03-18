//! Diagnostic collection and reporting.
//!
//! Covers HIS-032 through HIS-040.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::metrics_server::KNOWN_METRICS_PORTS;

mod bundle;
mod http;
mod network;
mod system;

pub use self::bundle::run_diagnostic;
pub use self::http::{
    DiagnosticHttpClient, diagnostics_http_client, find_metrics_server_http,
    probe_metrics_server_tunnel_state,
};
pub use self::network::{NetworkHop, NetworkTrace, collect_network_traces};
pub use self::system::collect_system_information;

/// Disk volume information matching Go `DiskVolumeInformation`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiskVolumeInformation {
    pub name: String,
    pub size_maximum: u64,
    pub size_current: u64,
}

/// Structured collector failure matching Go `SystemInformationError`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemInformationError {
    pub error: String,
    pub raw_info: String,
}

/// Structured collector failures matching Go `SystemInformationGeneralError`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemInformationErrors {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operating_system_information_error: Option<SystemInformationError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_information_error: Option<SystemInformationError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_descriptors_information_error: Option<SystemInformationError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_volume_information_error: Option<SystemInformationError>,
}

/// System information matching Go `SystemInformation`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemInformation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_maximum: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_current: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_descriptor_maximum: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_descriptor_current: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_release: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloudflared_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub go_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub go_arch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk: Option<Vec<DiskVolumeInformation>>,
}

/// Wrapper matching Go `SystemInformationResponse` served at `/diag/system`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemInformationResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<SystemInformation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<SystemInformationErrors>,
}

/// Per-connection diagnostics matching Go `IndexedConnectionInfo`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IndexedConnectionInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_connected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_address: Option<String>,
}

/// Tunnel state for the diagnostics `/diag/tunnel` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TunnelState {
    #[serde(rename = "tunnelID", skip_serializing_if = "Option::is_none")]
    pub tunnel_id: Option<String>,
    #[serde(rename = "connectorID", skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connections: Option<Vec<IndexedConnectionInfo>>,
    #[serde(rename = "icmp_sources", skip_serializing_if = "Option::is_none")]
    pub icmp_sources: Option<Vec<String>>,
}

/// Configuration diagnostics for `/diag/configuration`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigDiagnostics {
    pub uid: String,
    #[serde(rename = "logfile", skip_serializing_if = "Option::is_none")]
    pub log_file: Option<String>,
    #[serde(rename = "log-directory", skip_serializing_if = "Option::is_none")]
    pub log_directory: Option<String>,
}

/// Network check regions.
pub const DIAGNOSTIC_REGIONS: &[&str] = &["region1.v2.argotunnel.com", "region2.v2.argotunnel.com"];

/// Trait for the diagnostic handler.
pub trait DiagnosticHandler: Send + Sync {
    fn system_info(&self) -> cfdrs_shared::Result<SystemInformation>;

    fn tunnel_state(&self) -> cfdrs_shared::Result<TunnelState>;

    fn config_diagnostics(&self) -> cfdrs_shared::Result<ConfigDiagnostics>;
}

/// Stub diagnostic handler retained for contract compatibility.
pub struct StubDiagnosticHandler;

impl DiagnosticHandler for StubDiagnosticHandler {
    fn system_info(&self) -> cfdrs_shared::Result<SystemInformation> {
        Err(cfdrs_shared::ConfigError::deferred("diagnostic system info"))
    }

    fn tunnel_state(&self) -> cfdrs_shared::Result<TunnelState> {
        Err(cfdrs_shared::ConfigError::deferred("diagnostic tunnel state"))
    }

    fn config_diagnostics(&self) -> cfdrs_shared::Result<ConfigDiagnostics> {
        Err(cfdrs_shared::ConfigError::deferred("diagnostic config details"))
    }
}

/// Tunnel state paired with the metrics address it was found at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressableTunnelState {
    pub state: TunnelState,
    pub address: String,
}

/// Discovery-specific error.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum DiscoveryError {
    /// No running cloudflared instance found on any known port.
    #[error("metrics server not found")]
    MetricsServerNotFound,
    /// Multiple running instances detected; caller must disambiguate.
    #[error("multiple metrics server found")]
    MultipleMetricsServersFound { instances: Vec<AddressableTunnelState> },
}

/// Build the list of known metrics addresses to scan.
pub fn known_metrics_addresses(is_virtual: bool) -> Vec<String> {
    let host = if is_virtual { "0.0.0.0" } else { "localhost" };
    KNOWN_METRICS_PORTS
        .iter()
        .map(|port| format!("{host}:{port}"))
        .collect()
}

/// Scan the given addresses for a running metrics server.
pub fn find_metrics_server<F>(
    addresses: &[String],
    probe: F,
) -> std::result::Result<AddressableTunnelState, DiscoveryError>
where
    F: Fn(&str) -> Option<TunnelState>,
{
    let mut instances: Vec<AddressableTunnelState> = Vec::new();

    for address in addresses {
        if let Some(state) = probe(address) {
            instances.push(AddressableTunnelState {
                state,
                address: address.clone(),
            });
        }
    }

    match instances.len() {
        0 => Err(DiscoveryError::MetricsServerNotFound),
        1 => Ok(instances.into_iter().next().expect("single instance")),
        _ => Err(DiscoveryError::MultipleMetricsServersFound { instances }),
    }
}

/// Toggle flags for `tunnel diag`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticToggles {
    pub no_diag_logs: bool,
    pub no_diag_metrics: bool,
    pub no_diag_system: bool,
    pub no_diag_runtime: bool,
    pub no_diag_network: bool,
}

/// Options for the end-to-end diagnostic bundle runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticOptions {
    pub known_addresses: Vec<String>,
    pub address: Option<String>,
    pub container_id: Option<String>,
    pub pod_id: Option<String>,
    pub toggles: DiagnosticToggles,
    pub output_dir: Option<PathBuf>,
}

impl DiagnosticOptions {
    pub fn new(known_addresses: Vec<String>) -> Self {
        Self {
            known_addresses,
            address: None,
            container_id: None,
            pod_id: None,
            toggles: DiagnosticToggles::default(),
            output_dir: None,
        }
    }
}

/// Task result written into `task-result.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticTaskResult {
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl DiagnosticTaskResult {
    pub fn success() -> Self {
        Self {
            result: "success".to_owned(),
            error: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            result: "failure".to_owned(),
            error: Some(error.into()),
        }
    }
}

/// Successful diagnostic bundle outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticBundle {
    pub selected_address: String,
    pub zip_path: PathBuf,
    pub task_results: BTreeMap<String, DiagnosticTaskResult>,
}

impl DiagnosticBundle {
    pub fn had_errors(&self) -> bool {
        self.task_results
            .values()
            .any(|result| result.result == "failure")
    }

    pub fn contains_error_text(&self, needle: &str) -> bool {
        self.task_results.values().any(|result| {
            result
                .error
                .as_deref()
                .is_some_and(|error| error.contains(needle))
        })
    }
}

/// Fatal diagnostic execution failures.
#[derive(Debug, Error)]
pub enum DiagnosticRunError {
    #[error("provided address is not valid: {0}")]
    InvalidAddress(String),
    #[error("metrics server not found")]
    MetricsServerNotFound,
    #[error("multiple metrics server found")]
    MultipleMetricsServersFound { instances: Vec<AddressableTunnelState> },
    #[error("{0}")]
    Fatal(String),
}

impl From<DiscoveryError> for DiagnosticRunError {
    fn from(value: DiscoveryError) -> Self {
        match value {
            DiscoveryError::MetricsServerNotFound => Self::MetricsServerNotFound,
            DiscoveryError::MultipleMetricsServersFound { instances } => {
                Self::MultipleMetricsServersFound { instances }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_handler_returns_deferred() {
        let handler = StubDiagnosticHandler;
        assert!(handler.system_info().is_err());
        assert!(handler.tunnel_state().is_err());
        assert!(handler.config_diagnostics().is_err());
    }

    #[test]
    fn system_info_json_keys_match_go_baseline() {
        let info = SystemInformation {
            memory_maximum: Some(16_000_000),
            memory_current: Some(8_000_000),
            file_descriptor_maximum: Some(1024),
            file_descriptor_current: Some(256),
            os_system: Some("linux".into()),
            host_name: Some("test-host".into()),
            os_version: Some("Debian GNU/Linux 12".into()),
            os_release: Some("5.15.0".into()),
            architecture: Some("x86_64".into()),
            cloudflared_version: Some("2026.2.0".into()),
            go_version: Some("rustc 1.94.0".into()),
            go_arch: Some("x86_64".into()),
            disk: Some(vec![DiskVolumeInformation {
                name: "/dev/sda1".into(),
                size_maximum: 500_000_000,
                size_current: 250_000_000,
            }]),
        };
        let json = serde_json::to_string(&info).expect("serialize");
        assert!(json.contains("\"memoryMaximum\":"));
        assert!(json.contains("\"memoryCurrent\":"));
        assert!(json.contains("\"fileDescriptorMaximum\":"));
        assert!(json.contains("\"fileDescriptorCurrent\":"));
        assert!(json.contains("\"osSystem\":"));
        assert!(json.contains("\"hostName\":"));
        assert!(json.contains("\"osVersion\":"));
        assert!(json.contains("\"osRelease\":"));
        assert!(json.contains("\"architecture\":"));
        assert!(json.contains("\"cloudflaredVersion\":"));
        assert!(json.contains("\"goVersion\":"));
        assert!(json.contains("\"goArch\":"));
        assert!(json.contains("\"disk\":"));
    }

    #[test]
    fn system_info_response_json_shape_matches_go() {
        let resp = SystemInformationResponse {
            info: Some(SystemInformation {
                memory_maximum: Some(16_000_000),
                memory_current: None,
                file_descriptor_maximum: None,
                file_descriptor_current: None,
                os_system: Some("linux".into()),
                host_name: None,
                os_version: None,
                os_release: None,
                architecture: None,
                cloudflared_version: None,
                go_version: None,
                go_arch: None,
                disk: None,
            }),
            errors: Some(SystemInformationErrors::default()),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        assert!(json.contains("\"info\":"));
        assert!(json.contains("\"errors\":{}"));
    }

    #[test]
    fn system_info_errors_json_keys_match_go_baseline() {
        let errors = SystemInformationErrors {
            operating_system_information_error: Some(SystemInformationError {
                error: "uname failed".into(),
                raw_info: "bad output".into(),
            }),
            memory_information_error: None,
            file_descriptors_information_error: None,
            disk_volume_information_error: None,
        };
        let json = serde_json::to_string(&errors).expect("serialize");
        assert!(json.contains("\"operatingSystemInformationError\":"));
        assert!(json.contains("\"rawInfo\":"));
    }

    #[test]
    fn disk_volume_json_keys_match_go_baseline() {
        let vol = DiskVolumeInformation {
            name: "/dev/sda1".into(),
            size_maximum: 500_000_000,
            size_current: 250_000_000,
        };
        let json = serde_json::to_string(&vol).expect("serialize");
        assert!(json.contains("\"name\":"));
        assert!(json.contains("\"sizeMaximum\":"));
        assert!(json.contains("\"sizeCurrent\":"));
    }

    #[test]
    fn tunnel_state_json_keys_match_go_baseline() {
        let state = TunnelState {
            tunnel_id: Some("550e8400-e29b-41d4-a716-446655440000".into()),
            connector_id: Some("660e8400-e29b-41d4-a716-446655440000".into()),
            connections: Some(vec![IndexedConnectionInfo {
                index: Some(0),
                is_connected: Some(true),
                protocol: Some("quic".into()),
                edge_address: Some("198.41.200.1".into()),
            }]),
            icmp_sources: Some(vec!["192.168.1.1".into()]),
        };
        let json = serde_json::to_string(&state).expect("serialize");
        assert!(json.contains("\"tunnelID\":"));
        assert!(json.contains("\"connectorID\":"));
        assert!(json.contains("\"connections\":"));
        assert!(json.contains("\"icmp_sources\":"));
    }

    #[test]
    fn indexed_connection_info_json_keys_match_go_baseline() {
        let conn = IndexedConnectionInfo {
            index: Some(1),
            is_connected: Some(true),
            protocol: Some("quic".into()),
            edge_address: Some("198.41.200.1".into()),
        };
        let json = serde_json::to_string(&conn).expect("serialize");
        assert!(json.contains("\"index\":"));
        assert!(json.contains("\"isConnected\":"));
        assert!(json.contains("\"protocol\":"));
        assert!(json.contains("\"edgeAddress\":"));
    }

    #[test]
    fn tunnel_state_omitempty_matches_go() {
        let state = TunnelState {
            tunnel_id: None,
            connector_id: None,
            connections: None,
            icmp_sources: None,
        };
        let json = serde_json::to_string(&state).expect("serialize");
        assert_eq!(json, "{}");
    }

    #[test]
    fn diagnostic_regions_match_go_baseline() {
        assert_eq!(
            DIAGNOSTIC_REGIONS,
            &["region1.v2.argotunnel.com", "region2.v2.argotunnel.com"]
        );
    }

    #[test]
    fn config_diagnostics_json_keys_match_go_baseline() {
        let diag = ConfigDiagnostics {
            uid: "1000".into(),
            log_file: Some("/var/log/cloudflared.log".into()),
            log_directory: None,
        };
        let json = serde_json::to_string(&diag).expect("serialize");
        assert!(json.contains("\"uid\":"));
        assert!(json.contains("\"logfile\":"));
        assert!(!json.contains("log-directory"));
    }

    fn sample_tunnel_state() -> TunnelState {
        TunnelState {
            tunnel_id: Some("550e8400-e29b-41d4-a716-446655440000".into()),
            connector_id: Some("660e8400-e29b-41d4-a716-446655440000".into()),
            connections: None,
            icmp_sources: None,
        }
    }

    #[test]
    fn known_metrics_addresses_host_mode() {
        let addrs = known_metrics_addresses(false);
        assert_eq!(addrs.len(), 5);
        assert_eq!(addrs[0], "localhost:20241");
        assert_eq!(addrs[4], "localhost:20245");
    }

    #[test]
    fn known_metrics_addresses_virtual_mode() {
        let addrs = known_metrics_addresses(true);
        assert_eq!(addrs.len(), 5);
        assert_eq!(addrs[0], "0.0.0.0:20241");
        assert_eq!(addrs[4], "0.0.0.0:20245");
    }

    #[test]
    fn find_metrics_server_no_instance_returns_not_found() {
        let addrs = known_metrics_addresses(false);
        let result = find_metrics_server(&addrs, |_| None);
        assert!(matches!(
            result.expect_err("should be not-found"),
            DiscoveryError::MetricsServerNotFound
        ));
    }

    #[test]
    fn find_metrics_server_single_instance_returns_state() {
        let addrs = known_metrics_addresses(false);
        let result = find_metrics_server(&addrs, |addr| {
            if addr == "localhost:20243" {
                Some(sample_tunnel_state())
            } else {
                None
            }
        });
        let found = result.expect("should find single instance");
        assert_eq!(found.address, "localhost:20243");
        assert_eq!(
            found.state.tunnel_id.as_deref(),
            Some("550e8400-e29b-41d4-a716-446655440000")
        );
    }

    #[test]
    fn find_metrics_server_multiple_instances_returns_all() {
        let addrs = known_metrics_addresses(false);
        let result = find_metrics_server(&addrs, |addr| {
            if addr == "localhost:20241" || addr == "localhost:20244" {
                Some(sample_tunnel_state())
            } else {
                None
            }
        });
        match result.expect_err("should be multiple-found") {
            DiscoveryError::MultipleMetricsServersFound { instances } => {
                assert_eq!(instances.len(), 2);
                assert_eq!(instances[0].address, "localhost:20241");
                assert_eq!(instances[1].address, "localhost:20244");
            }
            other => panic!("expected MultipleMetricsServersFound, got {other}"),
        }
    }

    #[test]
    fn find_metrics_server_preserves_scan_order() {
        let addrs = vec![
            "localhost:20245".to_owned(),
            "localhost:20241".to_owned(),
            "localhost:20243".to_owned(),
        ];
        let result = find_metrics_server(&addrs, |_| Some(sample_tunnel_state()));
        match result.expect_err("should be multiple-found") {
            DiscoveryError::MultipleMetricsServersFound { instances } => {
                assert_eq!(instances[0].address, "localhost:20245");
                assert_eq!(instances[1].address, "localhost:20241");
                assert_eq!(instances[2].address, "localhost:20243");
            }
            other => panic!("expected MultipleMetricsServersFound, got {other}"),
        }
    }

    #[test]
    fn discovery_error_display_matches_go_messages() {
        assert_eq!(
            DiscoveryError::MetricsServerNotFound.to_string(),
            "metrics server not found"
        );
        assert_eq!(
            DiscoveryError::MultipleMetricsServersFound {
                instances: vec![AddressableTunnelState {
                    state: sample_tunnel_state(),
                    address: "localhost:20241".into(),
                }],
            }
            .to_string(),
            "multiple metrics server found"
        );
    }

    #[test]
    fn diagnostic_task_result_shape_matches_go_baseline() {
        let json = serde_json::to_string(&DiagnosticTaskResult::failure("boom")).expect("serialize");
        assert_eq!(json, "{\"result\":\"failure\",\"error\":\"boom\"}");
    }
}
