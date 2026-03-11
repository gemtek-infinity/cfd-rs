use std::sync::Arc;

use crate::protocol::{self, ProtocolReceiver};
use crate::transport::QuicTunnelServiceFactory;

use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::fmt;

mod command_dispatch;
mod deployment;
mod state;
mod tasks;
mod types;

#[cfg(test)]
mod tests;

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

pub(crate) fn install_runtime_logging() {
    RUNTIME_LOGGING.get_or_init(|| {
        let subscriber = fmt()
            .with_writer(std::io::stderr)
            .without_time()
            .with_target(false)
            .compact()
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    });
}

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
}

impl<F> ApplicationRuntime<F>
where
    F: RuntimeServiceFactory,
{
    fn new(
        config: RuntimeConfig,
        factory: F,
        harness: RuntimeHarness,
        protocol_receiver: Option<ProtocolReceiver>,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::channel(16);

        Self {
            config: Arc::new(config),
            factory,
            policy: RuntimePolicy::default(),
            harness,
            command_tx,
            command_rx,
            child_tasks: JoinSet::new(),
            shutdown: CancellationToken::new(),
            status: RuntimeStatus::new(protocol_receiver.is_some()),
            protocol_receiver,
        }
    }

    async fn run(mut self) -> RuntimeExecution {
        self.status.record_runtime_owner();
        self.status.record_runtime_config(self.config.as_ref());
        self.status.record_supervision_policy(&self.policy);
        self.status.record_readiness_scope();

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
