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
    pub(in crate::runtime) transport_failures: u32,
    pub(in crate::runtime) failure_events: u32,
    pub(in crate::runtime) restart_budget_max: u32,
    pub(in crate::runtime) protocol_bridge_present: bool,
    pub(in crate::runtime) timing: StageTiming,
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
            transport_failures: 0,
            failure_events: 0,
            restart_budget_max: 0,
            protocol_bridge_present,
            timing: StageTiming::new(),
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
        self.protocol_state = state;
        self.protocol_registrations += u32::from(state == ProtocolBridgeState::RegistrationObserved);

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
