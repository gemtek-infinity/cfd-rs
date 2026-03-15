//! Diagnostic collection and reporting.
//!
//! Covers HIS-032 through HIS-040.
//!
//! Most diagnostic endpoints are deferred to the Host and Runtime Foundation
//! or Command Family Closure milestone. This module defines the types and
//! trait contracts that the diagnostic system must satisfy.

use serde::{Deserialize, Serialize};

// --- HIS-033: system information ---

/// System information response matching Go `SystemInformationResponse`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInformation {
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub kernel_version: String,
    pub memory_total_kb: u64,
    pub memory_available_kb: u64,
    pub file_descriptor_limit: u64,
}

// --- HIS-034: tunnel state ---

/// Tunnel state for the diagnostics `/diag/tunnel` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelState {
    pub connector_id: String,
    pub connections: Vec<ConnectionState>,
}

/// Per-connection diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionState {
    pub id: u8,
    pub location: String,
    pub is_connected: bool,
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
    fn system_info_serializes() {
        let info = SystemInformation {
            os: "linux".into(),
            arch: "x86_64".into(),
            hostname: "test-host".into(),
            kernel_version: "5.15.0".into(),
            memory_total_kb: 16_000_000,
            memory_available_kb: 8_000_000,
            file_descriptor_limit: 1024,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        assert!(json.contains("\"os\":\"linux\""));
    }
}
