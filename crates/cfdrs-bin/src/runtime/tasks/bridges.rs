use tokio::time;

#[cfg(target_family = "unix")]
use tokio::signal::unix::{SignalKind, signal};

use super::super::{ApplicationRuntime, ChildTask, RuntimeCommand, ShutdownReason};

impl ApplicationRuntime {
    pub(in crate::runtime) fn spawn_signal_bridge(&mut self) {
        if !self.harness.enable_signals {
            return;
        }

        let command_tx = self.command_tx.clone();
        let shutdown = self.shutdown.child_token();
        self.child_tasks.spawn(async move {
            #[cfg(target_family = "unix")]
            unix_signal_task(command_tx, shutdown).await;

            ChildTask::SignalBridge
        });
    }
}

#[cfg(target_family = "unix")]
async fn unix_signal_task(
    command_tx: tokio::sync::mpsc::Sender<RuntimeCommand>,
    shutdown: tokio_util::sync::CancellationToken,
) {
    let Ok(mut sigint) = signal(SignalKind::interrupt()) else {
        let _ = command_tx
            .send(RuntimeCommand::ControlPlaneFailure {
                detail: "failed to register SIGINT handler".to_owned(),
            })
            .await;
        return;
    };
    let Ok(mut sigterm) = signal(SignalKind::terminate()) else {
        let _ = command_tx
            .send(RuntimeCommand::ControlPlaneFailure {
                detail: "failed to register SIGTERM handler".to_owned(),
            })
            .await;
        return;
    };

    tokio::select! {
        _ = shutdown.cancelled() => {}
        _ = sigint.recv() => {
            let _ = command_tx.send(RuntimeCommand::ShutdownRequested(ShutdownReason::Signal("SIGINT"))).await;
        }
        _ = sigterm.recv() => {
            let _ = command_tx.send(RuntimeCommand::ShutdownRequested(ShutdownReason::Signal("SIGTERM"))).await;
        }
    }
}

impl ApplicationRuntime {
    pub(in crate::runtime) fn spawn_harness_shutdown(&mut self) {
        let Some(duration) = self.harness.injected_shutdown_after else {
            return;
        };

        let command_tx = self.command_tx.clone();
        let shutdown = self.shutdown.child_token();
        self.child_tasks.spawn(async move {
            tokio::select! {
                _ = shutdown.cancelled() => {}
                _ = time::sleep(duration) => {
                    let _ = command_tx.send(RuntimeCommand::ShutdownRequested(ShutdownReason::Harness)).await;
                }
            }

            ChildTask::HarnessBridge
        });
    }
}
