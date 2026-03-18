use tracing::{error, info, warn};

use cfdrs_his::diagnostics::IndexedConnectionInfo;

use crate::protocol::ProtocolBridgeState;
use crate::proxy::ProxySeamState;
use crate::startup::config_source_label;
use crate::transport::TransportLifecycleStage;

use super::super::{READINESS_SCOPE, RuntimeConfig, RuntimePolicy};
use super::status::{LifecycleState, RuntimeStatus};
use crate::runtime::ShutdownReason;

impl RuntimeStatus {
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

    pub(in crate::runtime) fn record_tunnel_connection_observed(
        &mut self,
        index: u8,
        protocol: String,
        edge_address: String,
    ) {
        self.connection_info.insert(
            index,
            IndexedConnectionInfo {
                index: Some(index),
                is_connected: Some(true),
                protocol: Some(protocol),
                edge_address: Some(edge_address),
            },
        );
    }

    pub(in crate::runtime) fn clear_active_tunnel_connections(&mut self) {
        for info in self.connection_info.values_mut() {
            info.is_connected = Some(false);
        }
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
