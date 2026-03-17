use cfdrs_his::environment::current_executable;
use cfdrs_his::updater::{ManualUpdateOutcome, WorkersUpdateRequest, WorkersUpdater, run_manual_update};
use tokio::time::{self, Instant};

use super::super::types::RuntimeAutoUpdate;
use super::super::{ApplicationRuntime, ChildTask, RuntimeCommand};

impl ApplicationRuntime {
    pub(in crate::runtime) fn spawn_auto_updater(&mut self) {
        let Some(auto_update) = self.config.auto_update().cloned() else {
            return;
        };

        if let Some(reason) = auto_update.settings().disabled_reason() {
            self.status
                .push_summary(format!("auto-update: disabled ({reason})"));
            return;
        }

        let frequency = auto_update.settings().frequency();
        self.status
            .push_summary(format!("auto-update: enabled freq={frequency:?}"));

        let command_tx = self.command_tx.clone();
        let shutdown = self.shutdown.child_token();
        self.child_tasks.spawn(async move {
            let mut next_tick = Instant::now() + frequency;

            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = time::sleep_until(next_tick) => {}
                }
                next_tick += frequency;

                let cycle = match tokio::task::spawn_blocking({
                    let auto_update = auto_update.clone();
                    move || run_auto_update_cycle(&auto_update)
                })
                .await
                {
                    Ok(cycle) => cycle,
                    Err(error) => AutoUpdateCycle::Status {
                        detail: format!("auto-updater task failed: {error}"),
                    },
                };

                match cycle {
                    AutoUpdateCycle::Idle => {}
                    AutoUpdateCycle::Status { detail } => {
                        let _ = command_tx
                            .send(RuntimeCommand::ServiceStatus {
                                service: "auto-updater",
                                detail,
                            })
                            .await;
                    }
                    AutoUpdateCycle::Applied {
                        version,
                        user_message,
                    } => {
                        if let Some(detail) = user_message {
                            let _ = command_tx
                                .send(RuntimeCommand::ServiceStatus {
                                    service: "auto-updater",
                                    detail,
                                })
                                .await;
                        }
                        let _ = command_tx
                            .send(RuntimeCommand::AutoUpdateApplied { version })
                            .await;
                        break;
                    }
                }
            }

            ChildTask::AutoUpdater
        });
    }
}

enum AutoUpdateCycle {
    Idle,
    Status {
        detail: String,
    },
    Applied {
        version: String,
        user_message: Option<String>,
    },
}

fn run_auto_update_cycle(auto_update: &RuntimeAutoUpdate) -> AutoUpdateCycle {
    let target_path = match auto_update.target_path_override().cloned() {
        Some(path) => path,
        None => match current_executable() {
            Ok(path) => path,
            Err(error) => {
                return AutoUpdateCycle::Status {
                    detail: format!("auto-update target unavailable: {error}"),
                };
            }
        },
    };

    let mut request =
        WorkersUpdateRequest::new(env!("CARGO_PKG_VERSION"), target_path, false, false, false, None);
    if let Some(base_url) = auto_update.base_url_override() {
        request.base_url = base_url.to_owned();
    }

    let updater = match WorkersUpdater::new(request) {
        Ok(updater) => updater,
        Err(error) => {
            return AutoUpdateCycle::Status {
                detail: format!("auto-update client setup failed: {error}"),
            };
        }
    };

    match run_manual_update(&updater, false) {
        Ok(ManualUpdateOutcome::NoUpdate { user_message }) => user_message
            .map(|detail| AutoUpdateCycle::Status { detail })
            .unwrap_or(AutoUpdateCycle::Idle),
        Ok(ManualUpdateOutcome::Updated {
            version,
            user_message,
        }) => AutoUpdateCycle::Applied {
            version,
            user_message,
        },
        Ok(ManualUpdateOutcome::PackageManaged { message }) => AutoUpdateCycle::Status { detail: message },
        Err(error) => AutoUpdateCycle::Status {
            detail: format!("auto-update check failed: {error}"),
        },
    }
}
