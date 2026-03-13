use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use super::super::super::{ChildTask, RuntimeCommand, RuntimeService, ServiceExit};
use super::TestBehavior;

pub(super) struct TestService {
    pub(super) behavior: TestBehavior,
}

impl RuntimeService for TestService {
    fn name(&self) -> &'static str {
        "test-service"
    }

    fn spawn(
        self: Box<Self>,
        command_tx: mpsc::Sender<RuntimeCommand>,
        shutdown: CancellationToken,
        child_tasks: &mut JoinSet<ChildTask>,
    ) {
        let behavior = self.behavior.clone();
        child_tasks.spawn(async move {
            let _ = command_tx
                .send(RuntimeCommand::ServiceReady {
                    service: "test-service",
                })
                .await;

            match behavior {
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
}
