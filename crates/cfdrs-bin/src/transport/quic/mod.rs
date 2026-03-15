use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use super::TransportLifecycleStage;
use crate::protocol::{ProtocolSender, SharedStreamResponseReceiver};
use crate::runtime::{
    ChildTask, RuntimeCommand, RuntimeConfig, RuntimeService, RuntimeServiceFactory, ServiceExit,
};

mod datagram;
mod edge;
mod identity;
mod lifecycle;
mod reporting;
mod session;

#[cfg(test)]
mod tests;

use self::edge::{EDGE_DEFAULT_REGION, QuicEdgeTarget, resolve_edge_target};
use self::identity::TransportIdentity;

const QUIC_ESTABLISH_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_DATAGRAM_SIZE: usize = 1350;

#[derive(Debug, Clone)]
pub(crate) struct QuicTunnelServiceFactory {
    test_target: Option<QuicEdgeTarget>,
    protocol_sender: ProtocolSender,
    stream_response_rx: SharedStreamResponseReceiver,
}

impl QuicTunnelServiceFactory {
    pub(crate) fn production(
        protocol_sender: ProtocolSender,
        stream_response_rx: SharedStreamResponseReceiver,
    ) -> Self {
        Self {
            test_target: None,
            protocol_sender,
            stream_response_rx,
        }
    }

    #[cfg(test)]
    fn with_test_target(
        protocol_sender: ProtocolSender,
        stream_response_rx: SharedStreamResponseReceiver,
        target: QuicEdgeTarget,
    ) -> Self {
        Self {
            test_target: Some(target),
            protocol_sender,
            stream_response_rx,
        }
    }
}

impl RuntimeServiceFactory for QuicTunnelServiceFactory {
    fn create_primary(&self, config: Arc<RuntimeConfig>, attempt: u32) -> Box<dyn RuntimeService> {
        Box::new(QuicTunnelService {
            config,
            attempt,
            test_target: self.test_target.clone(),
            protocol_sender: self.protocol_sender.clone(),
            stream_response_rx: self.stream_response_rx.clone(),
        })
    }
}

struct QuicTunnelService {
    config: Arc<RuntimeConfig>,
    attempt: u32,
    test_target: Option<QuicEdgeTarget>,
    protocol_sender: ProtocolSender,
    stream_response_rx: SharedStreamResponseReceiver,
}

impl RuntimeService for QuicTunnelService {
    fn name(&self) -> &'static str {
        "quic-tunnel-core"
    }

    fn spawn(
        self: Box<Self>,
        command_tx: mpsc::Sender<RuntimeCommand>,
        shutdown: CancellationToken,
        child_tasks: &mut JoinSet<ChildTask>,
    ) {
        child_tasks.spawn(async move {
            let service_name = self.name();
            let exit = self.run(command_tx.clone(), shutdown).await;
            let _ = command_tx.send(RuntimeCommand::ServiceExited(exit)).await;
            ChildTask::Service(service_name)
        });
    }
}

impl QuicTunnelService {
    async fn run(self, command_tx: mpsc::Sender<RuntimeCommand>, shutdown: CancellationToken) -> ServiceExit {
        let service_name = self.name();
        let identity = match self.load_identity() {
            Ok(identity) => identity,
            Err(detail) => {
                return ServiceExit::Fatal {
                    service: service_name,
                    detail,
                };
            }
        };

        self.report_identity_status(&command_tx, service_name, &identity)
            .await;

        let target = match self.resolve_target(&identity).await {
            Ok(target) => target,
            Err(detail) => {
                return ServiceExit::RetryableFailure {
                    service: service_name,
                    detail,
                };
            }
        };

        send_status(
            &command_tx,
            service_name,
            format!(
                "transport-edge-target: host={} addr={} endpoint-hint={}",
                target.host_label,
                target.connect_addr,
                identity.endpoint_hint.as_deref().unwrap_or(EDGE_DEFAULT_REGION)
            ),
        )
        .await;
        send_transport_stage(
            &command_tx,
            service_name,
            TransportLifecycleStage::Dialing,
            format!("edge={}", target.connect_addr),
        )
        .await;
        send_status(
            &command_tx,
            service_name,
            "transport-session-state: dialing".to_owned(),
        )
        .await;

        match self
            .establish_quic_session(identity, target, &command_tx, shutdown)
            .await
        {
            Ok(exit) => exit,
            Err(detail) => ServiceExit::RetryableFailure {
                service: self.name(),
                detail,
            },
        }
    }

    fn load_identity(&self) -> Result<TransportIdentity, String> {
        TransportIdentity::from_runtime_config(&self.config)
    }

    async fn resolve_target(&self, identity: &TransportIdentity) -> Result<QuicEdgeTarget, String> {
        match self.test_target.as_ref() {
            Some(target) => Ok(target.clone()),
            None => resolve_edge_target(identity).await,
        }
    }
}

async fn send_status(command_tx: &mpsc::Sender<RuntimeCommand>, service: &'static str, detail: String) {
    let _ = command_tx
        .send(RuntimeCommand::ServiceStatus { service, detail })
        .await;
}

async fn send_transport_stage(
    command_tx: &mpsc::Sender<RuntimeCommand>,
    service: &'static str,
    stage: TransportLifecycleStage,
    detail: String,
) {
    let _ = command_tx
        .send(RuntimeCommand::TransportStage {
            service,
            stage,
            detail,
        })
        .await;
}
