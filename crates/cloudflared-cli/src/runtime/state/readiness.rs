use crate::protocol::ProtocolBridgeState;
use crate::proxy::ProxySeamState;
use crate::transport::TransportLifecycleStage;

use tracing::info;

use super::{LifecycleState, ReadinessState, RuntimeStatus};

impl RuntimeStatus {
    pub(in super::super) fn refresh_readiness(&mut self, reason: impl Into<String>) {
        let next = resolve_readiness_state(
            self.lifecycle_state,
            self.proxy_state,
            self.transport_stage,
            self.protocol_state,
        );
        if next != self.readiness_state {
            self.record_readiness(next, reason);
        }
    }

    pub(in super::super) fn record_readiness(&mut self, state: ReadinessState, reason: impl Into<String>) {
        let reason = reason.into();
        self.readiness_state = state;
        self.summary_lines
            .push(format!("readiness-state: {}", state.as_str()));
        self.summary_lines.push(format!("readiness-reason: {reason}"));
        info!(
            "readiness-transition state={} scope={} reason={reason}",
            state.as_str(),
            super::super::READINESS_SCOPE
        );
    }
}

/// Pure readiness resolution: lifecycle gates first, then subsystem
/// readiness in admission order (proxy -> transport -> protocol).
fn resolve_readiness_state(
    lifecycle: LifecycleState,
    proxy_state: Option<ProxySeamState>,
    transport_stage: Option<TransportLifecycleStage>,
    protocol_state: ProtocolBridgeState,
) -> ReadinessState {
    if lifecycle == LifecycleState::Failed {
        return ReadinessState::Failed;
    }

    if lifecycle.is_shutting_down() {
        return ReadinessState::Stopping;
    }

    if proxy_state.is_none() {
        return ReadinessState::WaitingForProxyAdmission;
    }

    if !transport_stage.is_some_and(|stage| stage.is_connected()) {
        return ReadinessState::WaitingForTransport;
    }

    if protocol_state != ProtocolBridgeState::RegistrationObserved {
        return ReadinessState::WaitingForProtocolBridge;
    }

    ReadinessState::Ready
}
