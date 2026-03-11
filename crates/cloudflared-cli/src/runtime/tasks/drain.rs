use tokio::time;

use super::super::{ApplicationRuntime, ChildTask, RuntimeServiceFactory};

impl<F> ApplicationRuntime<F>
where
    F: RuntimeServiceFactory,
{
    pub(in crate::runtime) async fn drain_child_tasks(&mut self) {
        loop {
            let joined = time::timeout(self.policy.shutdown_grace_period, self.child_tasks.join_next()).await;

            match joined {
                Ok(Some(Ok(child_task))) => self.record_stopped_child_task(child_task),
                Ok(Some(Err(error))) => {
                    self.status.push_summary(format!("child-task-error: {error}"));
                }
                Ok(None) => break,
                Err(_) => {
                    self.abort_timed_out_children().await;
                    break;
                }
            }
        }
    }

    async fn abort_timed_out_children(&mut self) {
        self.status
            .push_summary("shutdown-action: aborting remaining child tasks after grace timeout");
        self.child_tasks.abort_all();
        while let Some(result) = self.child_tasks.join_next().await {
            if let Err(error) = result {
                self.status.push_summary(format!("child-task-error: {error}"));
            }
        }
    }

    fn record_stopped_child_task(&mut self, child_task: ChildTask) {
        match child_task {
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
        }
    }
}
