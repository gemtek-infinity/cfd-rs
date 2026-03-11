use tokio::time;

use super::{
    ApplicationRuntime, LifecycleState, RuntimeCommand, RuntimeExit, RuntimeServiceFactory, ServiceExit,
};

impl<F> ApplicationRuntime<F>
where
    F: RuntimeServiceFactory,
{
    pub(super) async fn handle_command(&mut self, command: RuntimeCommand) -> Option<RuntimeExit> {
        match command {
            RuntimeCommand::ServiceReady { service } => {
                if self.status.lifecycle_state() == LifecycleState::Starting {
                    self.status
                        .record_state(LifecycleState::Running, format!("service ready: {service}"));
                }
                self.status.refresh_readiness(format!("{service} reported ready"));
                None
            }
            RuntimeCommand::ServiceStatus { service, detail } => {
                self.status.record_service_status(service, detail);
                None
            }
            RuntimeCommand::TransportStage {
                service,
                stage,
                detail,
            } => {
                self.status.record_transport_stage(service, stage, detail);
                self.status
                    .refresh_readiness(format!("transport reached {}", stage.as_str()));
                None
            }
            RuntimeCommand::ProtocolState { state, detail } => {
                self.status.record_protocol_state(state, detail);
                self.status
                    .refresh_readiness(format!("protocol bridge reached {}", state.as_str()));
                None
            }
            RuntimeCommand::ProxyState { state, detail } => {
                self.status.record_proxy_state(state, detail);
                self.status
                    .refresh_readiness(format!("proxy seam reached {}", state.as_str()));
                None
            }
            RuntimeCommand::ServiceExited(service_exit) => self.handle_service_exit(service_exit).await,
            RuntimeCommand::ShutdownRequested(reason) => {
                self.status.record_shutdown_reason(&reason);
                Some(RuntimeExit::Clean)
            }
            RuntimeCommand::ControlPlaneFailure { detail } => {
                self.status
                    .record_failure_boundary("runtime-control-plane", "fatal", &detail);
                Some(RuntimeExit::Failed { detail })
            }
        }
    }

    async fn handle_service_exit(&mut self, service_exit: ServiceExit) -> Option<RuntimeExit> {
        match service_exit {
            ServiceExit::Completed { service } => Some(RuntimeExit::Failed {
                detail: format!("{service} exited without a runtime shutdown request"),
            }),
            ServiceExit::RetryableFailure { service, detail } => {
                self.status.record_failure_boundary(service, "retryable", &detail);
                self.status.increment_transport_failures();

                if self.status.restart_attempts() < self.policy.max_restart_attempts {
                    let attempt = self.status.record_restart_attempt(service, &detail);
                    self.status.record_state(
                        LifecycleState::Starting,
                        format!("restarting {service} after retryable failure"),
                    );
                    self.status
                        .refresh_readiness(format!("runtime restarting {service} after retryable failure"));
                    time::sleep(self.policy.restart_backoff).await;
                    self.spawn_primary_service(attempt);
                    None
                } else {
                    Some(RuntimeExit::Failed {
                        detail: format!(
                            "{service} exhausted restart policy after {} attempts: {detail}",
                            self.status.restart_attempts()
                        ),
                    })
                }
            }
            ServiceExit::Deferred {
                service,
                phase,
                detail,
            } => {
                self.status.record_failure_boundary(service, "deferred", &detail);
                Some(RuntimeExit::Deferred {
                    phase,
                    detail: format!("{service}: {detail}"),
                })
            }
            ServiceExit::Fatal { service, detail } => {
                self.status.record_failure_boundary(service, "fatal", &detail);
                Some(RuntimeExit::Failed {
                    detail: format!("{service}: {detail}"),
                })
            }
        }
    }
}
