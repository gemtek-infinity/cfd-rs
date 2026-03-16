use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::protocol::{ProtocolSender, SharedStreamResponseReceiver};
#[cfg(test)]
use crate::runtime::ServiceExit;
use crate::runtime::{ChildTask, RuntimeCommand, RuntimeConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransportLifecycleStage {
    IdentityLoaded,
    ResolvingEdge,
    Dialing,
    Handshaking,
    Established,
    ControlStreamOpened,
    ServingStreams,
    Teardown,
}

impl TransportLifecycleStage {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::IdentityLoaded => "identity-loaded",
            Self::ResolvingEdge => "resolving-edge",
            Self::Dialing => "dialing",
            Self::Handshaking => "handshaking",
            Self::Established => "established",
            Self::ControlStreamOpened => "control-stream-opened",
            Self::ServingStreams => "serving-streams",
            Self::Teardown => "teardown",
        }
    }

    pub(crate) fn is_connected(self) -> bool {
        matches!(
            self,
            Self::Established | Self::ControlStreamOpened | Self::ServingStreams | Self::Teardown
        )
    }
}

impl std::fmt::Display for TransportLifecycleStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

mod quic;

// ---------------------------------------------------------------------------
// TransportService — concrete service enum replacing Box<dyn RuntimeService>
// ---------------------------------------------------------------------------

pub(crate) enum TransportService {
    QuicTunnel(quic::QuicTunnelService),
    #[cfg(test)]
    Test(TestServiceState),
}

impl TransportService {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::QuicTunnel(svc) => svc.name(),
            #[cfg(test)]
            Self::Test(_) => "test-service",
        }
    }

    pub(crate) fn spawn(
        self,
        command_tx: mpsc::Sender<RuntimeCommand>,
        shutdown: CancellationToken,
        child_tasks: &mut JoinSet<ChildTask>,
    ) {
        match self {
            Self::QuicTunnel(svc) => svc.spawn(command_tx, shutdown, child_tasks),
            #[cfg(test)]
            Self::Test(state) => spawn_test_service(state, command_tx, shutdown, child_tasks),
        }
    }
}

// ---------------------------------------------------------------------------
// TransportServiceSource — concrete factory enum replacing dyn factory trait
// ---------------------------------------------------------------------------

pub(crate) enum TransportServiceSource {
    Quic {
        test_target: Option<quic::QuicEdgeTarget>,
        protocol_sender: ProtocolSender,
        stream_response_rx: SharedStreamResponseReceiver,
    },
    #[cfg(test)]
    Test {
        behaviors: Arc<std::sync::Mutex<std::collections::VecDeque<TestBehavior>>>,
    },
}

impl TransportServiceSource {
    pub(crate) fn production(
        protocol_sender: ProtocolSender,
        stream_response_rx: SharedStreamResponseReceiver,
    ) -> Self {
        Self::Quic {
            test_target: None,
            protocol_sender,
            stream_response_rx,
        }
    }

    pub(crate) fn create_service(&self, config: Arc<RuntimeConfig>, attempt: u32) -> TransportService {
        match self {
            Self::Quic {
                test_target,
                protocol_sender,
                stream_response_rx,
            } => TransportService::QuicTunnel(quic::QuicTunnelService {
                config,
                attempt,
                test_target: test_target.clone(),
                protocol_sender: protocol_sender.clone(),
                stream_response_rx: stream_response_rx.clone(),
            }),
            #[cfg(test)]
            Self::Test { behaviors } => {
                let behavior = behaviors
                    .lock()
                    .expect("test service source lock should not be poisoned")
                    .pop_front()
                    .unwrap_or(TestBehavior::WaitForShutdown);
                TransportService::Test(TestServiceState { behavior })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Test-only types and spawn logic
// ---------------------------------------------------------------------------

#[cfg(test)]
#[derive(Clone)]
pub(crate) enum TestBehavior {
    WaitForShutdown,
    RetryableFailure,
    FatalFailure,
    DeferredExit,
    ControlPlaneFailure,
}

#[cfg(test)]
pub(crate) struct TestServiceState {
    behavior: TestBehavior,
}

#[cfg(test)]
impl TransportServiceSource {
    pub(crate) fn test(behaviors: impl IntoIterator<Item = TestBehavior>) -> Self {
        Self::Test {
            behaviors: Arc::new(std::sync::Mutex::new(behaviors.into_iter().collect())),
        }
    }
}

#[cfg(test)]
fn spawn_test_service(
    state: TestServiceState,
    command_tx: mpsc::Sender<RuntimeCommand>,
    shutdown: CancellationToken,
    child_tasks: &mut JoinSet<ChildTask>,
) {
    child_tasks.spawn(async move {
        let _ = command_tx
            .send(RuntimeCommand::ServiceReady {
                service: "test-service",
            })
            .await;

        match state.behavior {
            TestBehavior::WaitForShutdown => {
                shutdown.cancelled().await;
                let _ = command_tx
                    .send(RuntimeCommand::ServiceExited(ServiceExit::Completed {
                        service: "test-service",
                    }))
                    .await;
            }
            TestBehavior::RetryableFailure => {
                let _ = command_tx
                    .send(RuntimeCommand::ServiceExited(ServiceExit::RetryableFailure {
                        service: "test-service",
                        detail: "retry requested by lifecycle policy".to_owned(),
                    }))
                    .await;
            }
            TestBehavior::FatalFailure => {
                let _ = command_tx
                    .send(RuntimeCommand::ServiceExited(ServiceExit::Fatal {
                        service: "test-service",
                        detail: "fatal lifecycle boundary triggered".to_owned(),
                    }))
                    .await;
            }
            TestBehavior::DeferredExit => {
                let _ = command_tx
                    .send(RuntimeCommand::ServiceExited(ServiceExit::Deferred {
                        service: "test-service",
                        phase: "later-subsystem",
                        detail: "deferred boundary reached in test".to_owned(),
                    }))
                    .await;
            }
            TestBehavior::ControlPlaneFailure => {
                let _ = command_tx
                    .send(RuntimeCommand::ControlPlaneFailure {
                        detail: "simulated control-plane failure in test".to_owned(),
                    })
                    .await;
            }
        }

        ChildTask::Service("test-service")
    });
}
