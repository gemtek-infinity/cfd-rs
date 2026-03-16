use cfdrs_his::watcher::{FileWatcher, NotifyFileWatcher};

use super::super::{ApplicationRuntime, ChildTask, RuntimeCommand};

impl ApplicationRuntime {
    /// Spawn a config file watcher as a child task.
    ///
    /// Creates a `NotifyFileWatcher` on the config path, then bridges
    /// the blocking watcher loop into the async runtime via
    /// `spawn_blocking`. Write events send `ConfigFileChanged` through
    /// the runtime command channel. Errors are reported as service
    /// status events — matching Go `WatcherDidError` behavior where
    /// watching continues after errors.
    pub(in crate::runtime) fn spawn_config_watcher(&mut self) {
        let config_path = self.config.config_path().clone();

        let mut watcher = match NotifyFileWatcher::new() {
            Ok(w) => w,
            Err(error) => {
                self.status
                    .push_summary(format!("config-watcher: failed to create ({error})"));
                return;
            }
        };

        if let Err(error) = watcher.add(config_path.clone()) {
            self.status.push_summary(format!(
                "config-watcher: failed to watch {} ({error})",
                config_path.display()
            ));
            return;
        }

        let command_tx = self.command_tx.clone();
        let error_tx = self.command_tx.clone();
        let shutdown = self.shutdown.child_token();

        // Grab the shutdown flag before moving the watcher into the
        // blocking thread. This lets the async cancellation path
        // stop the blocking watcher loop.
        let watcher_shutdown = watcher.shutdown_flag();

        self.status
            .push_summary(format!("config-watcher: watching {}", config_path.display()));

        self.child_tasks.spawn(async move {
            let watcher_handle = tokio::task::spawn_blocking(move || {
                watcher.start(
                    move |path| {
                        let _ = command_tx.blocking_send(RuntimeCommand::ConfigFileChanged {
                            path: path.to_path_buf(),
                        });
                    },
                    move |error| {
                        let _ = error_tx.blocking_send(RuntimeCommand::ServiceStatus {
                            service: "config-watcher",
                            detail: format!("watcher error: {error}"),
                        });
                    },
                );
            });

            tokio::select! {
                _ = shutdown.cancelled() => {
                    watcher_shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                _ = watcher_handle => {}
            }

            ChildTask::ConfigWatcher
        });
    }
}
