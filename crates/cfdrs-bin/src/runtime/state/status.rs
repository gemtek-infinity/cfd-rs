use std::fmt;

use crate::protocol::ProtocolBridgeState;
use crate::proxy::ProxySeamState;
use crate::startup::config_source_label;
use crate::transport::TransportLifecycleStage;

use tracing::{error, info, warn};

use super::super::{READINESS_SCOPE, RuntimeConfig, RuntimePolicy, ShutdownReason};
use super::timing::StageTiming;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::runtime) enum LifecycleState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

impl LifecycleState {
    pub(in crate::runtime) fn as_str(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }

    pub(in crate::runtime) fn is_shutting_down(self) -> bool {
        matches!(self, Self::Stopping | Self::Stopped)
    }
}

impl fmt::Display for LifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::runtime) enum ReadinessState {
    Starting,
    WaitingForProxyAdmission,
    WaitingForTransport,
    WaitingForProtocolBridge,
    Ready,
    Stopping,
    Failed,
}

impl ReadinessState {
    pub(in crate::runtime) fn as_str(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::WaitingForProxyAdmission => "waiting-for-proxy-admission",
            Self::WaitingForTransport => "waiting-for-transport",
            Self::WaitingForProtocolBridge => "waiting-for-protocol-bridge",
            Self::Ready => "ready",
            Self::Stopping => "stopping",
            Self::Failed => "failed",
        }
    }
}

impl fmt::Display for ReadinessState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub(in crate::runtime) struct RuntimeStatus {
    pub(in crate::runtime) summary_lines: Vec<String>,
    pub(in crate::runtime) lifecycle_state: LifecycleState,
    pub(in crate::runtime) readiness_state: ReadinessState,
    pub(in crate::runtime) restart_attempts: u32,
    pub(in crate::runtime) transport_stage: Option<TransportLifecycleStage>,
    pub(in crate::runtime) protocol_state: ProtocolBridgeState,
    pub(in crate::runtime) proxy_state: Option<ProxySeamState>,
    pub(in crate::runtime) proxy_admissions: u32,
    pub(in crate::runtime) protocol_registrations: u32,
    /// Number of currently active tunnel connections.
    ///
    /// Matches Go `ConnTracker.CountActiveConns()` semantics:
    /// incremented on `RegistrationObserved`, decremented on
    /// `Reconnecting`, `Unregistering`, or `BridgeClosed`.
    pub(in crate::runtime) active_connections: u32,
    pub(in crate::runtime) transport_failures: u32,
    pub(in crate::runtime) failure_events: u32,
    pub(in crate::runtime) restart_budget_max: u32,
    pub(in crate::runtime) protocol_bridge_present: bool,
    pub(in crate::runtime) timing: StageTiming,
    pub(in crate::runtime) deployment: DeploymentState,
}

/// Deployment contract validation state collected during startup.
///
/// Tracks whether the host passed deployment-contract checks so the
/// evidence emitter can report accurate deployment proof at finish.
pub(in crate::runtime) struct DeploymentState {
    pub(in crate::runtime) host_validated: bool,
    pub(in crate::runtime) glibc_present: bool,
    pub(in crate::runtime) systemd_detected: bool,
    pub(in crate::runtime) config_path: Option<String>,
}

impl RuntimeStatus {
    pub(in crate::runtime) fn new(protocol_bridge_present: bool) -> Self {
        Self {
            summary_lines: Vec::new(),
            lifecycle_state: LifecycleState::Starting,
            readiness_state: ReadinessState::Starting,
            restart_attempts: 0,
            transport_stage: None,
            protocol_state: initial_protocol_state(protocol_bridge_present),
            proxy_state: None,
            proxy_admissions: 0,
            protocol_registrations: 0,
            active_connections: 0,
            transport_failures: 0,
            failure_events: 0,
            restart_budget_max: 0,
            protocol_bridge_present,
            timing: StageTiming::new(),
            deployment: DeploymentState {
                host_validated: false,
                glibc_present: false,
                systemd_detected: false,
                config_path: None,
            },
        }
    }

    pub(in crate::runtime) fn into_summary_lines(self) -> Vec<String> {
        self.summary_lines
    }

    pub(in crate::runtime) fn push_summary(&mut self, line: impl Into<String>) {
        self.summary_lines.push(line.into());
    }

    fn push_labeled_summary(&mut self, label: &str, value: impl std::fmt::Display) {
        self.summary_lines.push(format!("{label}: {value}"));
    }

    pub(in crate::runtime) fn lifecycle_state(&self) -> LifecycleState {
        self.lifecycle_state
    }

    pub(in crate::runtime) fn protocol_state(&self) -> ProtocolBridgeState {
        self.protocol_state
    }

    pub(in crate::runtime) fn protocol_bridge_is_present(&self) -> bool {
        self.protocol_bridge_present
    }

    pub(in crate::runtime) fn restart_attempts(&self) -> u32 {
        self.restart_attempts
    }

    pub(in crate::runtime) fn increment_transport_failures(&mut self) {
        self.transport_failures += 1;
    }

    pub(in crate::runtime) fn record_restart_attempt(&mut self, service: &'static str, detail: &str) -> u32 {
        self.restart_attempts += 1;
        self.summary_lines.push(format!(
            "supervision-restart-attempt: {} service={} detail={detail}",
            self.restart_attempts, service
        ));
        warn!(
            "runtime-restart service={service} attempt={} detail={detail}",
            self.restart_attempts
        );
        self.restart_attempts
    }

    pub(in crate::runtime) fn record_runtime_owner(&mut self) {
        self.push_summary("runtime-owner: initialized");
        self.push_summary("config-ownership: runtime-owned");
    }

    pub(in crate::runtime) fn record_runtime_config(&mut self, config: &RuntimeConfig) {
        self.push_labeled_summary(
            "runtime-config-source",
            config_source_label(config.config_source()),
        );
        self.push_labeled_summary("runtime-config-path", config.config_path().display());
        self.push_labeled_summary("runtime-ingress-rules", config.normalized().ingress.len());
    }

    pub(in crate::runtime) fn record_supervision_policy(&mut self, policy: &RuntimePolicy) {
        self.push_summary(format!(
            "supervision-policy: primary-service={} max-restarts={} restart-backoff-ms={} \
             shutdown-grace-ms={}",
            super::super::PRIMARY_SERVICE_NAME,
            policy.max_restart_attempts,
            policy.restart_backoff.as_millis(),
            policy.shutdown_grace_period.as_millis()
        ));
    }

    pub(in crate::runtime) fn record_readiness_scope(&mut self) {
        self.push_labeled_summary("readiness-scope", READINESS_SCOPE);
    }

    pub(in crate::runtime) fn record_service_status(&mut self, service: &'static str, detail: String) {
        self.summary_lines
            .push(format!("service-status[{service}]: {detail}"));
        info!("service-status service={service} detail={detail}");
    }

    pub(in crate::runtime) fn record_transport_stage(
        &mut self,
        service: &'static str,
        stage: TransportLifecycleStage,
        detail: String,
    ) {
        self.transport_stage = Some(stage);
        self.summary_lines
            .push(format!("transport-stage[{service}]: {stage}"));
        self.summary_lines
            .push(format!("transport-detail[{service}]: {detail}"));
        info!("transport-stage service={service} stage={stage} detail={detail}",);
    }

    pub(in crate::runtime) fn record_proxy_state(&mut self, state: ProxySeamState, detail: String) {
        self.proxy_state = Some(state);
        self.proxy_admissions += u32::from(state == ProxySeamState::Admitted);

        self.summary_lines.push(format!("proxy-state: {state}"));
        self.summary_lines.push(format!("proxy-detail: {detail}"));
        info!("proxy-state state={state} detail={detail}");
    }

    pub(in crate::runtime) fn record_protocol_state(
        &mut self,
        state: ProtocolBridgeState,
        detail: impl Into<String>,
    ) {
        let was_connected = self.protocol_state == ProtocolBridgeState::RegistrationObserved;
        let is_connected = state == ProtocolBridgeState::RegistrationObserved;

        self.protocol_state = state;
        self.protocol_registrations += u32::from(is_connected);

        // Track active connection count matching Go ConnTracker semantics.
        if !was_connected && is_connected {
            self.active_connections = self.active_connections.saturating_add(1);
        } else if was_connected && !is_connected {
            self.active_connections = self.active_connections.saturating_sub(1);
        }

        let detail = detail.into();
        self.summary_lines.push(format!("protocol-state: {state}"));
        self.summary_lines.push(format!("protocol-detail: {detail}"));
        info!("protocol-state state={state} detail={detail}");
    }

    pub(in crate::runtime) fn record_shutdown_reason(&mut self, reason: &ShutdownReason) {
        self.push_labeled_summary("shutdown-reason", reason);
        info!("runtime-shutdown-request reason={reason}");
    }

    pub(in crate::runtime) fn record_failure_boundary(
        &mut self,
        owner: &'static str,
        class: &'static str,
        detail: &str,
    ) {
        self.failure_events += 1;
        self.summary_lines.push(format!(
            "failure-visibility: owner={owner} class={class} detail={detail}"
        ));
        error!("failure-boundary owner={owner} class={class} detail={detail}");
    }

    pub(in crate::runtime) fn record_state(&mut self, state: LifecycleState, reason: impl Into<String>) {
        self.lifecycle_state = state;
        let reason = reason.into();
        self.push_labeled_summary("lifecycle-state", state);
        self.push_labeled_summary("lifecycle-reason", &reason);
        info!("lifecycle-transition state={state} reason={reason}");
    }
}

fn initial_protocol_state(protocol_bridge_present: bool) -> ProtocolBridgeState {
    if protocol_bridge_present {
        ProtocolBridgeState::BridgeCreated
    } else {
        ProtocolBridgeState::BridgeUnavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- HIS-025: active_connections tracks Go ConnTracker semantics ---

    #[test]
    fn active_connections_increments_on_registration() {
        let mut status = RuntimeStatus::new(true);

        assert_eq!(status.active_connections, 0);

        status.record_protocol_state(ProtocolBridgeState::RegistrationSent, "sent");
        assert_eq!(status.active_connections, 0);

        status.record_protocol_state(ProtocolBridgeState::RegistrationObserved, "registered");
        assert_eq!(status.active_connections, 1);
    }

    #[test]
    fn active_connections_decrements_on_disconnect() {
        let mut status = RuntimeStatus::new(true);

        status.record_protocol_state(ProtocolBridgeState::RegistrationObserved, "registered");
        assert_eq!(status.active_connections, 1);

        status.record_protocol_state(ProtocolBridgeState::Reconnecting, "reconnecting");
        assert_eq!(status.active_connections, 0);
    }

    #[test]
    fn active_connections_decrements_on_unregistering() {
        let mut status = RuntimeStatus::new(true);

        status.record_protocol_state(ProtocolBridgeState::RegistrationObserved, "registered");
        assert_eq!(status.active_connections, 1);

        status.record_protocol_state(ProtocolBridgeState::Unregistering, "unregistering");
        assert_eq!(status.active_connections, 0);
    }

    #[test]
    fn active_connections_decrements_on_bridge_closed() {
        let mut status = RuntimeStatus::new(true);

        status.record_protocol_state(ProtocolBridgeState::RegistrationObserved, "registered");
        assert_eq!(status.active_connections, 1);

        status.record_protocol_state(ProtocolBridgeState::BridgeClosed, "closed");
        assert_eq!(status.active_connections, 0);
    }

    #[test]
    fn active_connections_does_not_underflow() {
        let mut status = RuntimeStatus::new(true);

        // Double disconnect should saturate at 0, not underflow.
        status.record_protocol_state(ProtocolBridgeState::RegistrationObserved, "registered");
        status.record_protocol_state(ProtocolBridgeState::Reconnecting, "reconnecting");
        status.record_protocol_state(ProtocolBridgeState::Reconnecting, "already disconnected");
        assert_eq!(status.active_connections, 0);
    }

    #[test]
    fn active_connections_register_disconnect_cycle() {
        let mut status = RuntimeStatus::new(true);

        // Register → disconnect → re-register matches Go's event cycle.
        status.record_protocol_state(ProtocolBridgeState::RegistrationObserved, "first");
        assert_eq!(status.active_connections, 1);

        status.record_protocol_state(ProtocolBridgeState::Reconnecting, "reconnect");
        assert_eq!(status.active_connections, 0);

        status.record_protocol_state(ProtocolBridgeState::RegistrationObserved, "second");
        assert_eq!(status.active_connections, 1);
        assert_eq!(status.protocol_registrations, 2);
    }
}
