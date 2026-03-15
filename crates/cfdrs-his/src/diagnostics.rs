//! Diagnostic collection and reporting.
//!
//! Covers HIS-032 through HIS-040.
//!
//! Most diagnostic endpoints are deferred to the Host and Runtime Foundation
//! or Command Family Closure milestone. This module defines the types and
//! trait contracts that the diagnostic system must satisfy.

use serde::{Deserialize, Serialize};

use crate::metrics_server::KNOWN_METRICS_PORTS;

// --- HIS-033: system information ---

/// Disk volume information matching Go `DiskVolumeInformation`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskVolumeInformation {
    pub name: String,
    pub size_maximum: u64,
    pub size_current: u64,
}

/// System information matching Go `SystemInformation`.
///
/// Go: `diagnostic/system_collector.go`
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub disk: Option<Vec<DiskVolumeInformation>>,
}

/// Wrapper matching Go `SystemInformationResponse` served at `/diag/system`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInformationResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<SystemInformation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<String>,
}

// --- HIS-034: tunnel state ---

/// Tunnel state for the diagnostics `/diag/tunnel` endpoint.
///
/// Go: `diagnostic/handlers.go` `TunnelState`
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Per-connection diagnostics matching Go `IndexedConnectionInfo`.
///
/// Go embeds `ConnectionInfo` and adds `Index`.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// --- HIS-035: configuration diagnostics ---

/// Configuration diagnostics for `/diag/configuration`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDiagnostics {
    pub uid: String,
    pub log_file: Option<String>,
    pub log_directory: Option<String>,
}

// --- HIS-037: network diagnostics ---

/// Network check regions.
pub const DIAGNOSTIC_REGIONS: &[&str] = &["region1.v2.argotunnel.com", "region2.v2.argotunnel.com"];

// --- HIS-032, HIS-038, HIS-039, HIS-040: diagnostic handler ---

/// Trait for the diagnostic handler.
///
/// Go: `diagnostic/handlers.go` installs endpoints:
/// - `/diag/system` (HIS-039)
/// - `/diag/tunnel` (HIS-040)
/// - `/diag/configuration` (HIS-035)
///
/// Full implementation is deferred to Host and Runtime Foundation.
pub trait DiagnosticHandler: Send + Sync {
    /// Collect system information.
    fn system_info(&self) -> cfdrs_shared::Result<SystemInformation>;

    /// Collect tunnel state.
    fn tunnel_state(&self) -> cfdrs_shared::Result<TunnelState>;

    /// Collect configuration diagnostics.
    fn config_diagnostics(&self) -> cfdrs_shared::Result<ConfigDiagnostics>;
}

/// Stub diagnostic handler for pre-alpha.
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

// --- HIS-038: diagnostic instance discovery ---

/// Tunnel state paired with the address it was found at.
///
/// Go: `diagnostic/diagnostic_utils.go` `AddressableTunnelState`
#[derive(Debug, Clone)]
pub struct AddressableTunnelState {
    pub state: TunnelState,
    pub address: String,
}

/// Discovery-specific error.
///
/// Go: `diagnostic/error.go` `ErrMetricsServerNotFound` /
/// `ErrMultipleMetricsServerFound`
#[derive(Debug)]
pub enum DiscoveryError {
    /// No running cloudflared instance found on any known port.
    MetricsServerNotFound,
    /// Multiple running instances detected; caller must disambiguate.
    MultipleMetricsServersFound { instances: Vec<AddressableTunnelState> },
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MetricsServerNotFound => write!(f, "metrics server not found"),
            Self::MultipleMetricsServersFound { instances } => {
                write!(f, "multiple metrics server found ({})", instances.len())
            }
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// Build the list of known metrics addresses to scan.
///
/// Go: `metrics/metrics.go` `GetMetricsKnownAddresses` builds
/// `localhost:<port>` (host mode) or `0.0.0.0:<port>` (container mode)
/// for ports 20241–20245.
pub fn known_metrics_addresses(is_virtual: bool) -> Vec<String> {
    let host = if is_virtual { "0.0.0.0" } else { "localhost" };
    KNOWN_METRICS_PORTS
        .iter()
        .map(|port| format!("{host}:{port}"))
        .collect()
}

/// Scan the given addresses for a running metrics server.
///
/// Go: `diagnostic/diagnostic_utils.go` `FindMetricsServer`
///
/// The `probe` closure is called for each address and should attempt
/// an HTTP GET to `/tunnel-state`. When a real HTTP client is available,
/// callers pass a closure that performs the request. For testing, the
/// probe returns pre-canned results.
///
/// Returns:
/// - `Ok(single)` when exactly one instance responds
/// - `Err(MetricsServerNotFound)` when no instance responds
/// - `Err(MultipleMetricsServersFound { .. })` when 2+ respond
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
        // Go: diagnostic/system_collector.go — camelCase JSON tags with omitempty
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
            disk: Some(vec![DiskVolumeInformation {
                name: "/dev/sda1".into(),
                size_maximum: 500_000_000,
                size_current: 250_000_000,
            }]),
        };
        let json = serde_json::to_string(&info).expect("serialize");
        // Verify Go baseline camelCase key names
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
        assert!(json.contains("\"disk\":"));
        // Verify omitempty: None fields are absent
        let sparse = SystemInformation {
            memory_maximum: Some(16_000_000),
            memory_current: None,
            file_descriptor_maximum: None,
            file_descriptor_current: None,
            os_system: None,
            host_name: None,
            os_version: None,
            os_release: None,
            architecture: None,
            cloudflared_version: None,
            disk: None,
        };
        let sparse_json = serde_json::to_string(&sparse).expect("serialize sparse");
        assert!(!sparse_json.contains("\"memoryCurrent\":"));
        assert!(!sparse_json.contains("\"disk\":"));
    }

    #[test]
    fn system_info_response_json_shape_matches_go() {
        // Go: diagnostic/handlers.go — { "info": ..., "errors": ... }
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
                disk: None,
            }),
            errors: None,
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        assert!(json.contains("\"info\":"));
        assert!(!json.contains("\"errors\":"));
    }

    #[test]
    fn disk_volume_json_keys_match_go_baseline() {
        // Go: diagnostic/system_collector.go — DiskVolumeInformation (camelCase)
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

    // --- HIS-034: tunnel state JSON shape ---

    #[test]
    fn tunnel_state_json_keys_match_go_baseline() {
        // Go: diagnostic/handlers.go — TunnelState
        // Note: icmp_sources uses snake_case in Go, not camelCase
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
        // Go: tunnelstate/conntracker.go — IndexedConnectionInfo embeds ConnectionInfo
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
        // All-None TunnelState should produce empty object
        let state = TunnelState {
            tunnel_id: None,
            connector_id: None,
            connections: None,
            icmp_sources: None,
        };
        let json = serde_json::to_string(&state).expect("serialize");
        assert_eq!(json, "{}");
    }

    // --- HIS-037: network diagnostic regions ---
    // Go: diagnostic/diagnostic.go lines 176-179

    #[test]
    fn diagnostic_regions_match_go_baseline() {
        assert_eq!(DIAGNOSTIC_REGIONS.len(), 2);
        assert_eq!(DIAGNOSTIC_REGIONS[0], "region1.v2.argotunnel.com");
        assert_eq!(DIAGNOSTIC_REGIONS[1], "region2.v2.argotunnel.com");
    }

    // --- HIS-035: config diagnostics ---

    #[test]
    fn config_diagnostics_json_keys_match_go_baseline() {
        // Go: diagnostic/handlers.go — map[string]string with "uid" key
        let diag = ConfigDiagnostics {
            uid: "1000".into(),
            log_file: Some("/var/log/cloudflared.log".into()),
            log_directory: None,
        };
        let json = serde_json::to_string(&diag).expect("serialize");
        assert!(json.contains("\"uid\":"));
        assert!(json.contains("\"log_file\":"));
        // None field should still be present (Go uses map, not omitempty)
        assert!(json.contains("\"log_directory\":null"));
    }

    // --- HIS-038: instance discovery tests ---

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
        // Go: metrics.GetMetricsKnownAddresses(false) → localhost:{20241..20245}
        let addrs = known_metrics_addresses(false);
        assert_eq!(addrs.len(), 5);
        assert_eq!(addrs[0], "localhost:20241");
        assert_eq!(addrs[4], "localhost:20245");
        for addr in &addrs {
            assert!(addr.starts_with("localhost:"));
        }
    }

    #[test]
    fn known_metrics_addresses_virtual_mode() {
        // Go: metrics.GetMetricsKnownAddresses(true) → 0.0.0.0:{20241..20245}
        let addrs = known_metrics_addresses(true);
        assert_eq!(addrs.len(), 5);
        assert_eq!(addrs[0], "0.0.0.0:20241");
        assert_eq!(addrs[4], "0.0.0.0:20245");
        for addr in &addrs {
            assert!(addr.starts_with("0.0.0.0:"));
        }
    }

    #[test]
    fn find_metrics_server_no_instance_returns_not_found() {
        // Go: FindMetricsServer returns ErrMetricsServerNotFound when 0 found
        let addrs = known_metrics_addresses(false);
        let result = find_metrics_server(&addrs, |_| None);
        assert!(result.is_err());
        assert!(matches!(
            result.expect_err("should be not-found"),
            DiscoveryError::MetricsServerNotFound
        ));
    }

    #[test]
    fn find_metrics_server_single_instance_returns_state() {
        // Go: FindMetricsServer returns the single AddressableTunnelState
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
        // Go: FindMetricsServer returns ErrMultipleMetricsServerFound
        // with the full list of found instances
        let addrs = known_metrics_addresses(false);
        let result = find_metrics_server(&addrs, |addr| {
            if addr == "localhost:20241" || addr == "localhost:20244" {
                Some(sample_tunnel_state())
            } else {
                None
            }
        });
        assert!(result.is_err());
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
        // Go scans addresses in order; verify we preserve that order
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
        // Go: "metrics server not found" / "multiple metrics server found"
        let not_found = DiscoveryError::MetricsServerNotFound;
        assert_eq!(not_found.to_string(), "metrics server not found");

        let multiple = DiscoveryError::MultipleMetricsServersFound {
            instances: vec![
                AddressableTunnelState {
                    state: sample_tunnel_state(),
                    address: "localhost:20241".into(),
                },
                AddressableTunnelState {
                    state: sample_tunnel_state(),
                    address: "localhost:20242".into(),
                },
            ],
        };
        assert!(multiple.to_string().contains("multiple metrics server found"));
        assert!(multiple.to_string().contains("2"));
    }

    #[test]
    fn addressable_tunnel_state_carries_both_fields() {
        let ats = AddressableTunnelState {
            state: sample_tunnel_state(),
            address: "localhost:20241".into(),
        };
        assert_eq!(ats.address, "localhost:20241");
        assert!(ats.state.tunnel_id.is_some());
    }
}
