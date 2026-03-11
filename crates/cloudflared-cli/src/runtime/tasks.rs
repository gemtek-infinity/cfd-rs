use tokio::time;

#[cfg(target_family = "unix")]
use tokio::signal::unix::{SignalKind, signal};

use crate::proxy::PingoraProxySeam;

use super::{ApplicationRuntime, ChildTask, RuntimeCommand, RuntimeServiceFactory, ShutdownReason};

impl<F> ApplicationRuntime<F>
where
    F: RuntimeServiceFactory,
{
    pub(super) fn spawn_proxy_seam(&mut self) {
        let ingress = self.config.normalized().ingress.clone();
        let seam = PingoraProxySeam::new(ingress);
        let protocol_rx = self.protocol_receiver.take();
        self.status.push_summary(format!(
            "proxy-seam: origin-proxy admitted, ingress-rules={}",
            seam.ingress_count()
        ));
        seam.spawn(
            self.command_tx.clone(),
            protocol_rx,
            self.shutdown.child_token(),
            &mut self.child_tasks,
        );
    }

    pub(super) fn spawn_primary_service(&mut self, attempt: u32) {
        let service = self.factory.create_primary(self.config.clone(), attempt);
        self.status.push_summary(format!(
            "primary-service-attempt: {} service={}",
            attempt + 1,
            service.name()
        ));
        service.spawn(
            self.command_tx.clone(),
            self.shutdown.child_token(),
            &mut self.child_tasks,
        );
    }

    pub(super) fn spawn_signal_bridge(&mut self) {
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

    pub(super) fn spawn_harness_shutdown(&mut self) {
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

    pub(super) async fn drain_child_tasks(&mut self) {
        loop {
            let joined = time::timeout(self.policy.shutdown_grace_period, self.child_tasks.join_next()).await;

            match joined {
                Ok(Some(Ok(child_task))) => match child_task {
                    ChildTask::Service(name) => {
                        self.status
                            .push_summary(format!("child-task-stopped: service={name}"));
                    }
                    ChildTask::ProxySeam => {
                        self.status.push_summary("child-task-stopped: proxy-seam");
                    }
                    ChildTask::SignalBridge => {
                        self.status.push_summary("child-task-stopped: signal-bridge");
                    }
                    ChildTask::HarnessBridge => {
                        self.status.push_summary("child-task-stopped: harness-bridge");
                    }
                },
                Ok(Some(Err(error))) => {
                    self.status.push_summary(format!("child-task-error: {error}"));
                }
                Ok(None) => break,
                Err(_) => {
                    self.status
                        .push_summary("shutdown-action: aborting remaining child tasks after grace timeout");
                    self.child_tasks.abort_all();
                    while let Some(result) = self.child_tasks.join_next().await {
                        if let Err(error) = result {
                            self.status.push_summary(format!("child-task-error: {error}"));
                        }
                    }
                    break;
                }
            }
        }
    }
}
