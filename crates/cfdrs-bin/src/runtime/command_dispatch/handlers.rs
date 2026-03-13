use tokio::time;

use crate::protocol::ProtocolBridgeState;
use crate::proxy::ProxySeamState;
use crate::transport::TransportLifecycleStage;

use super::super::{ApplicationRuntime, LifecycleState, RuntimeExit, RuntimeServiceFactory, ShutdownReason};

impl<F> ApplicationRuntime<F>
where
    F: RuntimeServiceFactory,
{
    pub(super) fn handle_service_ready(&mut self, service: &'static str) -> Option<RuntimeExit> {
        let is_resumed = self.status.restart_attempts() > 0;
        self.status.record_timing_service_ready(is_resumed);

        if self.status.lifecycle_state() == LifecycleState::Starting {
            self.status
                .record_state(LifecycleState::Running, format!("service ready: {service}"));
        }
        self.status.refresh_readiness(format!("{service} reported ready"));
        None
    }

    pub(super) fn handle_service_status(
        &mut self,
        service: &'static str,
        detail: String,
    ) -> Option<RuntimeExit> {
        self.status.record_service_status(service, detail);
        None
    }

    pub(super) fn handle_transport_stage(
        &mut self,
        service: &'static str,
        stage: TransportLifecycleStage,
        detail: String,
    ) -> Option<RuntimeExit> {
        self.status.record_timing_transport_stage(stage);
        self.status.record_transport_stage(service, stage, detail);
        self.status
            .refresh_readiness(format!("transport reached {stage}"));
        None
    }

    pub(super) fn handle_protocol_state(
        &mut self,
        state: ProtocolBridgeState,
        detail: String,
    ) -> Option<RuntimeExit> {
        if state == ProtocolBridgeState::RegistrationObserved {
            self.status.record_timing_protocol_registration();
        }
        self.status.record_protocol_state(state, detail);
        self.status
            .refresh_readiness(format!("protocol bridge reached {state}"));
        None
    }

    pub(super) fn handle_proxy_state(
        &mut self,
        state: ProxySeamState,
        detail: String,
    ) -> Option<RuntimeExit> {
        if state == ProxySeamState::Admitted {
            self.status.record_timing_proxy_admitted();
        }
        self.status.record_proxy_state(state, detail);
        self.status
            .refresh_readiness(format!("proxy seam reached {state}"));
        None
    }

    pub(super) fn handle_shutdown_requested(&mut self, reason: ShutdownReason) -> Option<RuntimeExit> {
        self.status.record_shutdown_reason(&reason);
        Some(RuntimeExit::Clean)
    }

    pub(super) fn handle_control_plane_failure(&mut self, detail: String) -> Option<RuntimeExit> {
        self.status
            .record_failure_boundary("runtime-control-plane", "fatal", &detail);
        Some(RuntimeExit::Failed { detail })
    }

    pub(super) async fn handle_retryable_service_exit(
        &mut self,
        service: &'static str,
        detail: String,
    ) -> Option<RuntimeExit> {
        if self.status.restart_attempts() >= self.policy.max_restart_attempts {
            return Some(RuntimeExit::Failed {
                detail: format!(
                    "{service} exhausted restart policy after {} attempts: {detail}",
                    self.status.restart_attempts()
                ),
            });
        }

        let attempt = self.status.record_restart_attempt(service, &detail);
        self.status.record_timing_restart();
        self.status.record_state(
            LifecycleState::Starting,
            format!("restarting {service} after retryable failure"),
        );
        self.status
            .refresh_readiness(format!("runtime restarting {service} after retryable failure"));
        time::sleep(self.policy.restart_backoff).await;
        self.spawn_primary_service(attempt);
        None
    }
}
