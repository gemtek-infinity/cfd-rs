mod handlers;

use super::{ApplicationRuntime, RuntimeCommand, RuntimeExit, ServiceExit};

impl ApplicationRuntime {
    pub(super) async fn handle_command(&mut self, command: RuntimeCommand) -> Option<RuntimeExit> {
        match command {
            RuntimeCommand::ServiceReady { service } => self.handle_service_ready(service),
            RuntimeCommand::ServiceStatus { service, detail } => self.handle_service_status(service, detail),
            RuntimeCommand::TransportStage {
                service,
                stage,
                detail,
            } => self.handle_transport_stage(service, stage, detail),
            RuntimeCommand::ProtocolState { state, detail } => self.handle_protocol_state(state, detail),
            RuntimeCommand::ProxyState { state, detail } => self.handle_proxy_state(state, detail),
            RuntimeCommand::TunnelConnectionObserved {
                index,
                protocol,
                edge_address,
            } => self.handle_tunnel_connection_observed(index, protocol, edge_address),
            RuntimeCommand::ServiceExited(service_exit) => self.handle_service_exit(service_exit).await,
            RuntimeCommand::ShutdownRequested(reason) => self.handle_shutdown_requested(reason),
            RuntimeCommand::ControlPlaneFailure { detail } => self.handle_control_plane_failure(detail),
            RuntimeCommand::ConfigFileChanged { path } => self.handle_config_file_changed(path),
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
                self.handle_retryable_service_exit(service, detail).await
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
