use tokio::time;

#[cfg(target_family = "unix")]
use tokio::signal::unix::{SignalKind, signal};

use super::super::{ApplicationRuntime, ChildTask, RuntimeCommand, RuntimeServiceFactory, ShutdownReason};

impl<F> ApplicationRuntime<F>
where
    F: RuntimeServiceFactory,
{
    pub(in crate::runtime) fn spawn_signal_bridge(&mut self) {
        if !self.harness.enable_signals {
            return;
        }

        let command_tx = self.command_tx.clone();
        let shutdown = self.shutdown.child_token();
        self.child_tasks.spawn(async move {
            #[cfg(target_family = "unix")]
            {
                let mut sigint = match signal(SignalKind::interrupt()) {
                    Ok(signal) => signal,
                    Err(error) => {
                        let _ = command_tx
                            .send(RuntimeCommand::ControlPlaneFailure {
                                detail: format!("failed to register SIGINT handler: {error}"),
                            })
                            .await;
                        return ChildTask::SignalBridge;
                    }
                };
                let mut sigterm = match signal(SignalKind::terminate()) {
                    Ok(signal) => signal,
                    Err(error) => {
                        let _ = command_tx
                            .send(RuntimeCommand::ControlPlaneFailure {
                                detail: format!("failed to register SIGTERM handler: {error}"),
                            })
                            .await;
                        return ChildTask::SignalBridge;
                    }
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

            ChildTask::SignalBridge
        });
    }

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
