//! Phase 3.4a+b: Pingora proxy-layer seam with runtime lifecycle participation.
//!
//! This module is the owned entry point for Pingora in the production-alpha
//! path. All direct Pingora types and API usage are confined here. The rest
//! of the binary does not depend on Pingora crates directly.
//!
//! ADR-0003 governs Pingora scope: application-layer proxy above the quiche
//! transport lane, not a transport replacement.
//!
//! 3.4a admitted: dependency path and seam location.
//! 3.4b admitted: runtime lifecycle participation (startup/shutdown
//! coordination). Deferred to 3.4c: origin-facing proxy behavior via
//! `pingora-proxy`. Deferred to 3.4d: transport → proxy handoff.

use std::marker::PhantomData;

use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::runtime::{ChildTask, RuntimeCommand};

pub(crate) const PROXY_SEAM_NAME: &str = "pingora-proxy-seam";

/// Owned boundary for Pingora proxy-layer admission and lifecycle
/// participation.
///
/// Confines the Pingora dependency surface to this module. The type witness
/// binds the admitted Pingora HTTP request type without allocating, proving
/// the dependency path compiles and the seam location is intentional.
///
/// Construction and lifecycle participation are admitted at this phase.
/// Actual proxy behavior is deferred to 3.4c.
pub(crate) struct PingoraProxySeam {
    _marker: PhantomData<pingora_http::RequestHeader>,
}

impl PingoraProxySeam {
    pub(crate) fn new() -> Self {
        Self { _marker: PhantomData }
    }

    /// Spawn the proxy seam as a runtime-owned lifecycle participant.
    ///
    /// The seam holds a lifecycle position in the runtime's child task set,
    /// participates in startup/shutdown coordination, and exits when the
    /// shutdown token is cancelled. Actual proxy behavior is deferred to 3.4c.
    pub(crate) fn spawn(
        self,
        command_tx: mpsc::Sender<RuntimeCommand>,
        shutdown: CancellationToken,
        child_tasks: &mut JoinSet<ChildTask>,
    ) {
        child_tasks.spawn(async move {
            let _ = command_tx
                .send(RuntimeCommand::ServiceStatus {
                    service: PROXY_SEAM_NAME,
                    detail: "lifecycle-admitted: startup position held, proxy behavior deferred to 3.4c"
                        .to_owned(),
                })
                .await;

            // Hold the lifecycle position until shutdown. In 3.4c this becomes
            // the actual proxy service loop.
            shutdown.cancelled().await;

            let _ = command_tx
                .send(RuntimeCommand::ServiceStatus {
                    service: PROXY_SEAM_NAME,
                    detail: "lifecycle-exit: shutdown acknowledged".to_owned(),
                })
                .await;

            ChildTask::ProxySeam
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seam_constructs_at_proxy_boundary() {
        let _seam = PingoraProxySeam::new();
    }

    #[test]
    fn pingora_http_request_type_admitted() {
        // Dependency admission proof: Pingora HTTP types can be
        // constructed and contained at this seam boundary.
        let header = pingora_http::RequestHeader::build("GET", b"/", None);
        assert!(
            header.is_ok(),
            "Pingora HTTP request type should build at the admitted seam"
        );
    }

    #[tokio::test]
    async fn proxy_seam_participates_in_startup_lifecycle() {
        let (command_tx, mut command_rx) = mpsc::channel(16);
        let shutdown = CancellationToken::new();
        let mut child_tasks = JoinSet::new();

        let seam = PingoraProxySeam::new();
        seam.spawn(command_tx, shutdown.clone(), &mut child_tasks);

        // Seam should report lifecycle admission status on startup.
        let msg = command_rx.recv().await.expect("should receive lifecycle status");
        match msg {
            RuntimeCommand::ServiceStatus { service, detail } => {
                assert_eq!(service, PROXY_SEAM_NAME);
                assert!(detail.contains("lifecycle-admitted"));
            }
            other => panic!("expected ServiceStatus for lifecycle admission, got: {other:?}"),
        }

        // Trigger shutdown and verify the seam acknowledges it.
        shutdown.cancel();

        let msg = command_rx.recv().await.expect("should receive shutdown status");
        match msg {
            RuntimeCommand::ServiceStatus { service, detail } => {
                assert_eq!(service, PROXY_SEAM_NAME);
                assert!(detail.contains("shutdown acknowledged"));
            }
            other => panic!("expected ServiceStatus for shutdown exit, got: {other:?}"),
        }

        // Child task should complete as ProxySeam.
        let result = child_tasks.join_next().await;
        assert!(result.is_some(), "proxy seam child task should complete");
        match result
            .expect("join should succeed")
            .expect("task should not panic")
        {
            ChildTask::ProxySeam => {}
            other => panic!("expected ChildTask::ProxySeam, got: {other:?}"),
        }
    }
}
