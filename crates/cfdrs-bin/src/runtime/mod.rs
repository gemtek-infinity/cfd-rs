use std::sync::Arc;

use cfdrs_his::signal::remove_pidfile;

use crate::protocol::{self, ProtocolReceiver};
use crate::transport::QuicTunnelServiceFactory;

use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

mod command_dispatch;
mod deployment;
mod logging;
mod metrics;
mod state;
mod tasks;
mod types;

#[cfg(test)]
mod tests;

pub(crate) use self::logging::install_runtime_logging;
use self::state::{LifecycleState, ReadinessState, RuntimeStatus};
use self::types::RuntimePolicy;
pub(crate) use self::types::{
    ChildTask, HarnessBuilder, RuntimeCommand, RuntimeConfig, RuntimeExecution, RuntimeExit, RuntimeHarness,
    RuntimeService, RuntimeServiceFactory, ServiceExit, ShutdownReason,
};

const PRIMARY_SERVICE_NAME: &str = "quic-tunnel-core";
const FROZEN_TARGET_TRIPLE: &str = "x86_64-unknown-linux-gnu";
const TRANSPORT_CRYPTO_LANE: &str = "quiche+boringssl";
const READINESS_SCOPE: &str = "narrow-alpha-control-plane-only";
const GLIBC_RUNTIME_MARKERS: &[&str] = &[
    "/lib64/ld-linux-x86-64.so.2",
    "/lib/x86_64-linux-gnu/libc.so.6",
    "/usr/lib64/libc.so.6",
];

static RUNTIME_LOGGING: std::sync::OnceLock<()> = std::sync::OnceLock::new();

struct ApplicationRuntime<F> {
    config: Arc<RuntimeConfig>,
    factory: F,
    policy: RuntimePolicy,
    harness: RuntimeHarness,
    command_tx: mpsc::Sender<RuntimeCommand>,
    command_rx: mpsc::Receiver<RuntimeCommand>,
    child_tasks: JoinSet<ChildTask>,
    shutdown: CancellationToken,
    status: RuntimeStatus,
    protocol_receiver: Option<ProtocolReceiver>,
    metrics: Option<metrics::RuntimeMetricsHandle>,
}

impl<F> ApplicationRuntime<F>
where
    F: RuntimeServiceFactory,
{
    fn start_metrics_server(&mut self) -> Result<(), String> {
        let handle = metrics::RuntimeMetricsHandle::start(self.config.as_ref())?;
        let actual_address = handle.actual_address();

        self.status
            .push_summary(format!("metrics-listener: http://{actual_address}"));
        self.metrics = Some(handle);
        self.sync_metrics_snapshot();

        Ok(())
    }

    fn sync_metrics_snapshot(&self) {
        if let Some(metrics) = self.metrics.as_ref() {
            metrics.sync_from_status(&self.status);
        }
    }

    fn new(
        config: RuntimeConfig,
        factory: F,
        harness: RuntimeHarness,
        protocol_receiver: Option<ProtocolReceiver>,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::channel(16);
        let mut policy = RuntimePolicy::default();

        if let Some(shutdown_grace_period) = config.shutdown_grace_period() {
            policy.shutdown_grace_period = shutdown_grace_period;
        }

        Self {
            config: Arc::new(config),
            factory,
            policy,
            harness,
            command_tx,
            command_rx,
            child_tasks: JoinSet::new(),
            shutdown: CancellationToken::new(),
            status: RuntimeStatus::new(protocol_receiver.is_some()),
            protocol_receiver,
            metrics: None,
        }
    }

    async fn run(mut self) -> RuntimeExecution {
        if let Err(detail) = self.start_metrics_server() {
            return self.finish(RuntimeExit::Failed { detail }).await;
        }

        self.status.record_runtime_owner();
        self.status.record_runtime_config(self.config.as_ref());
        self.status.record_supervision_policy(&self.policy);
        self.status.restart_budget_max = self.policy.max_restart_attempts;
        self.status.record_readiness_scope();
        self.sync_metrics_snapshot();

        if let Err(detail) = self.record_security_compliance_boundary() {
            return self.finish(RuntimeExit::Failed { detail }).await;
        }

        self.status
            .record_state(LifecycleState::Starting, "startup sequencing entered");
        self.status
            .record_readiness(ReadinessState::Starting, "runtime startup sequencing entered");
        self.status.record_protocol_state(
            self.status.protocol_state(),
            if self.status.protocol_bridge_is_present() {
                "runtime created protocol bridge endpoints"
            } else {
                "protocol bridge omitted by runtime harness"
            },
        );

        self.spawn_signal_bridge();
        self.spawn_harness_shutdown();
        self.spawn_proxy_seam();
        self.spawn_primary_service(0);

        loop {
            let Some(command) = self.command_rx.recv().await else {
                return self
                    .finish(RuntimeExit::Failed {
                        detail: "runtime command channel closed unexpectedly".to_owned(),
                    })
                    .await;
            };

            if let Some(exit) = self.handle_command(command).await {
                return self.finish(exit).await;
            }
        }
    }

    async fn finish(mut self, exit: RuntimeExit) -> RuntimeExecution {
        let stopping_reason = match &exit {
            RuntimeExit::Clean => "graceful shutdown requested".to_owned(),
            RuntimeExit::Deferred { detail, .. } => {
                format!("deferred service boundary reached: {detail}")
            }
            RuntimeExit::Failed { detail } => format!("runtime failure: {detail}"),
        };
        self.status
            .record_state(LifecycleState::Stopping, stopping_reason);
        self.status
            .record_readiness(ReadinessState::Stopping, "runtime shutdown sequencing entered");

        if matches!(exit, RuntimeExit::Deferred { .. }) {
            self.status
                .record_shutdown_reason(&ShutdownReason::ServiceFailure(PRIMARY_SERVICE_NAME));
        }

        self.shutdown.cancel();
        self.drain_child_tasks().await;

        if let Some(metrics) = self.metrics.take() {
            metrics.stop();
        }

        if let Some(pidfile_path) = self.config.pidfile_path() {
            remove_pidfile(pidfile_path);
        }

        match exit {
            RuntimeExit::Clean => {
                self.status
                    .record_state(LifecycleState::Stopped, "runtime stopped cleanly");
                self.status
                    .record_readiness(ReadinessState::Stopping, "runtime stopped after clean shutdown");
            }
            RuntimeExit::Deferred { .. } | RuntimeExit::Failed { .. } => {
                self.status.record_state(
                    LifecycleState::Failed,
                    "runtime stopped with a deferred or failed service boundary",
                );
                self.status.record_readiness(
                    ReadinessState::Failed,
                    "runtime stopped after deferred or failed service boundary",
                );
            }
        }

        self.status.record_operability_summary();
        self.status.record_timing_finished();
        self.status.record_performance_evidence();
        self.status.record_failure_evidence();
        self.status.record_deployment_evidence();

        RuntimeExecution {
            summary_lines: self.status.into_summary_lines(),
            exit,
        }
    }
}

pub(crate) fn run(config: RuntimeConfig) -> RuntimeExecution {
    let (protocol_sender, protocol_receiver) = protocol::protocol_bridge();
    run_with_factory(
        config,
        QuicTunnelServiceFactory::production(protocol_sender),
        HarnessBuilder::production().build(),
        Some(protocol_receiver),
    )
}

pub(crate) fn run_with_factory<F>(
    config: RuntimeConfig,
    factory: F,
    harness: RuntimeHarness,
    protocol_receiver: Option<ProtocolReceiver>,
) -> RuntimeExecution
where
    F: RuntimeServiceFactory,
{
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build for the admitted production-alpha shell");

    runtime.block_on(ApplicationRuntime::new(config, factory, harness, protocol_receiver).run())
}
