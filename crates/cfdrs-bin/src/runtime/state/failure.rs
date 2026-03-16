//! Phase 4.3: Failure-mode evidence emission.
//!
//! Emits structured `failure-*` evidence lines at runtime finish,
//! analogous to `perf-*` performance evidence from Phase 4.2.
//!
//! What this proves:
//! - reconnect/retry behavior is bounded and visible
//! - shutdown behavior is observable through lifecycle transitions
//! - dependency-boundary failures are reported at the correct owner
//! - config-file-watcher is wired via `NotifyFileWatcher`
//!
//! What this does not prove:
//! - real QUIC reconnect against a live edge (transport is in-process)
//! - broad proxy failure recovery beyond the admitted origin path
//! - full config-reload integration (watcher exists, re-apply path pending)
//! - deployment-level recovery (packaging/process manager integration)

use super::RuntimeStatus;

impl RuntimeStatus {
    /// Emit structured failure-mode evidence at runtime finish.
    ///
    /// Produces machine-readable `failure-*` lines that summarize the
    /// failure/recovery behavior observed during this runtime execution.
    /// Called from `finish()` alongside performance evidence.
    pub(in crate::runtime) fn record_failure_evidence(&mut self) {
        let restart_budget_used = self.restart_attempts;
        let restart_budget_max = self.restart_budget_max;
        let exhausted = restart_budget_used >= restart_budget_max && restart_budget_used > 0;

        self.summary_lines.push(format!(
            "failure-restart-budget: used={restart_budget_used} max={restart_budget_max} \
             exhausted={exhausted}"
        ));

        self.summary_lines
            .push(format!("failure-events-total: {}", self.failure_events));

        self.summary_lines
            .push(format!("failure-transport-failures: {}", self.transport_failures));

        // Config reload: file watcher is wired via NotifyFileWatcher,
        // but the re-apply path through ReloadActionLoop is not yet
        // connected. The watcher detects changes; the runtime logs them.
        self.summary_lines.push(
            "failure-config-reload: watcher-only (file changes detected, re-apply path pending)".to_owned(),
        );

        // Dependency-boundary summary: report which boundaries reported
        // failures during this runtime execution.
        let dep_boundaries = self.collect_dependency_boundary_summary();
        self.summary_lines
            .push(format!("failure-dependency-boundaries: {dep_boundaries}"));

        // Evidence scope honesty.
        self.summary_lines.push(
            "failure-evidence-scope: in-process-harness-failure-proof (real transport reconnect and \
             deployment-level recovery are deferred; config-watcher is wired, re-apply pending)"
                .to_owned(),
        );
    }

    fn collect_dependency_boundary_summary(&self) -> String {
        let mut parts = Vec::new();

        if self.transport_failures > 0 {
            parts.push(format!("transport(failures={})", self.transport_failures));
        }

        let protocol_label = self.protocol_state.as_str();
        parts.push(format!("protocol(state={protocol_label})"));

        let proxy_label = self.proxy_state.map_or("not-reported", |s| s.as_str());
        parts.push(format!("proxy(state={proxy_label})"));

        parts.join(", ")
    }
}
