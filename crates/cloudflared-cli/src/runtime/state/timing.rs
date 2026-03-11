//! Phase 4.2: Transport-stage timing evidence and regression thresholds.
//!
//! Tracks wall-clock elapsed time at each transport lifecycle stage
//! transition relative to runtime start. Produces machine-readable
//! performance evidence for the admitted alpha path.
//!
//! What this measures:
//! - stage transition timing within the runtime command dispatch loop
//! - cold-start (attempt 0) vs resumed (attempt > 0) path distinction
//! - time from runtime start to each subsystem milestone
//!
//! What this does not measure:
//! - real QUIC wire latency (deferred until real transport is testable)
//! - 0-RTT session resumption handshake savings (transport is in-process)
//! - end-to-end request latency (incoming stream handling is deferred)

use std::time::{Duration, Instant};

use super::RuntimeStatus;
use crate::transport::TransportLifecycleStage;

/// Regression thresholds for the admitted alpha harness path.
///
/// These are the maximum acceptable durations for stage transitions
/// within the in-process test harness. Real transport thresholds will
/// be defined when the transport layer is testable against a real edge.
pub(in crate::runtime) struct RegressionThresholds;

impl RegressionThresholds {
    /// Maximum time from runtime start to proxy admission.
    pub(in crate::runtime) const PROXY_ADMISSION_MAX: Duration = Duration::from_millis(500);
    /// Maximum time from runtime start to readiness (full pipeline).
    pub(in crate::runtime) const READINESS_MAX: Duration = Duration::from_secs(2);
    /// Maximum restart overhead: time from restart decision to service ready.
    pub(in crate::runtime) const RESTART_OVERHEAD_MAX: Duration = Duration::from_millis(500);
    /// Maximum time from runtime start to service ready (cold path).
    pub(in crate::runtime) const SERVICE_READY_MAX: Duration = Duration::from_millis(500);
    /// Maximum total runtime duration for a harness-driven lifecycle.
    pub(in crate::runtime) const TOTAL_RUNTIME_MAX: Duration = Duration::from_secs(5);
}

/// Stage-transition timing milestones relative to runtime start.
pub(in crate::runtime) struct StageTiming {
    runtime_start: Instant,
    proxy_admitted: Option<Instant>,
    service_ready: Option<Instant>,
    transport_identity: Option<Instant>,
    transport_dialing: Option<Instant>,
    transport_established: Option<Instant>,
    control_stream_opened: Option<Instant>,
    protocol_registration: Option<Instant>,
    readiness_reached: Option<Instant>,
    last_restart: Option<Instant>,
    resumed_service_ready: Option<Instant>,
    runtime_finished: Option<Instant>,
}

impl StageTiming {
    pub(in crate::runtime) fn new() -> Self {
        Self {
            runtime_start: Instant::now(),
            proxy_admitted: None,
            service_ready: None,
            transport_identity: None,
            transport_dialing: None,
            transport_established: None,
            control_stream_opened: None,
            protocol_registration: None,
            readiness_reached: None,
            last_restart: None,
            resumed_service_ready: None,
            runtime_finished: None,
        }
    }

    pub(in crate::runtime) fn record_proxy_admitted(&mut self) {
        self.proxy_admitted.get_or_insert_with(Instant::now);
    }

    pub(in crate::runtime) fn record_service_ready(&mut self, is_resumed: bool) {
        let now = Instant::now();
        self.service_ready.get_or_insert(now);

        if is_resumed {
            self.resumed_service_ready = Some(now);
        }
    }

    pub(in crate::runtime) fn record_transport_stage(&mut self, stage: TransportLifecycleStage) {
        let now = Instant::now();
        match stage {
            TransportLifecycleStage::IdentityLoaded => {
                self.transport_identity.get_or_insert(now);
            }
            TransportLifecycleStage::Dialing => {
                self.transport_dialing.get_or_insert(now);
            }
            TransportLifecycleStage::Established => {
                self.transport_established.get_or_insert(now);
            }
            TransportLifecycleStage::ControlStreamOpened => {
                self.control_stream_opened.get_or_insert(now);
            }
            TransportLifecycleStage::ResolvingEdge
            | TransportLifecycleStage::Handshaking
            | TransportLifecycleStage::Teardown => {}
        }
    }

    pub(in crate::runtime) fn record_protocol_registration(&mut self) {
        self.protocol_registration.get_or_insert_with(Instant::now);
    }

    pub(in crate::runtime) fn record_readiness_reached(&mut self) {
        self.readiness_reached.get_or_insert_with(Instant::now);
    }

    pub(in crate::runtime) fn record_restart(&mut self) {
        self.last_restart = Some(Instant::now());
    }

    pub(in crate::runtime) fn record_finished(&mut self) {
        self.runtime_finished = Some(Instant::now());
    }

    fn elapsed_ms(&self, milestone: Option<Instant>) -> Option<u64> {
        milestone.map(|t| t.duration_since(self.runtime_start).as_millis() as u64)
    }

    fn restart_to_ready_ms(&self) -> Option<u64> {
        match (self.last_restart, self.resumed_service_ready) {
            (Some(restart), Some(ready)) if ready >= restart => {
                Some(ready.duration_since(restart).as_millis() as u64)
            }
            _ => None,
        }
    }

    fn total_runtime_ms(&self) -> Option<u64> {
        self.runtime_finished
            .map(|t| t.duration_since(self.runtime_start).as_millis() as u64)
    }

    /// Build the machine-readable performance evidence lines.
    ///
    /// Each line is a key=value pair suitable for structured log parsing,
    /// CI gate evaluation, and regression threshold validation.
    pub(in crate::runtime) fn evidence_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let path_label = if self.last_restart.is_some() {
            "resumed"
        } else {
            "cold"
        };

        lines.push(format!("perf-evidence-path: {path_label}"));

        if let Some(ms) = self.elapsed_ms(self.proxy_admitted) {
            lines.push(format!("perf-stage-ms[proxy-admitted]: {ms}"));
        }

        if let Some(ms) = self.elapsed_ms(self.service_ready) {
            lines.push(format!("perf-stage-ms[service-ready]: {ms}"));
        }

        if let Some(ms) = self.elapsed_ms(self.transport_identity) {
            lines.push(format!("perf-stage-ms[transport-identity]: {ms}"));
        }

        if let Some(ms) = self.elapsed_ms(self.transport_dialing) {
            lines.push(format!("perf-stage-ms[transport-dialing]: {ms}"));
        }

        if let Some(ms) = self.elapsed_ms(self.transport_established) {
            lines.push(format!("perf-stage-ms[transport-established]: {ms}"));
        }

        if let Some(ms) = self.elapsed_ms(self.control_stream_opened) {
            lines.push(format!("perf-stage-ms[control-stream-opened]: {ms}"));
        }

        if let Some(ms) = self.elapsed_ms(self.protocol_registration) {
            lines.push(format!("perf-stage-ms[protocol-registration]: {ms}"));
        }

        if let Some(ms) = self.elapsed_ms(self.readiness_reached) {
            lines.push(format!("perf-stage-ms[readiness-reached]: {ms}"));
        }

        if let Some(ms) = self.restart_to_ready_ms() {
            lines.push(format!("perf-restart-overhead-ms: {ms}"));
        }

        if let Some(ms) = self.total_runtime_ms() {
            lines.push(format!("perf-total-runtime-ms: {ms}"));
        }

        lines
    }

    /// Validate timing against regression thresholds.
    ///
    /// Returns a list of threshold violations found. An empty list means
    /// the timing meets all regression thresholds.
    pub(in crate::runtime) fn threshold_violations(&self) -> Vec<String> {
        let mut violations = Vec::new();

        if let Some(proxy_admitted) = self.proxy_admitted {
            let elapsed = proxy_admitted.duration_since(self.runtime_start);
            if elapsed > RegressionThresholds::PROXY_ADMISSION_MAX {
                violations.push(format!(
                    "proxy-admission exceeded {}ms threshold: {}ms",
                    RegressionThresholds::PROXY_ADMISSION_MAX.as_millis(),
                    elapsed.as_millis()
                ));
            }
        }

        if let Some(service_ready) = self.service_ready {
            let elapsed = service_ready.duration_since(self.runtime_start);
            if elapsed > RegressionThresholds::SERVICE_READY_MAX {
                violations.push(format!(
                    "service-ready exceeded {}ms threshold: {}ms",
                    RegressionThresholds::SERVICE_READY_MAX.as_millis(),
                    elapsed.as_millis()
                ));
            }
        }

        if let Some(readiness_reached) = self.readiness_reached {
            let elapsed = readiness_reached.duration_since(self.runtime_start);
            if elapsed > RegressionThresholds::READINESS_MAX {
                violations.push(format!(
                    "readiness exceeded {}ms threshold: {}ms",
                    RegressionThresholds::READINESS_MAX.as_millis(),
                    elapsed.as_millis()
                ));
            }
        }

        if let (Some(restart), Some(ready)) = (self.last_restart, self.resumed_service_ready)
            && ready >= restart
        {
            let elapsed = ready.duration_since(restart);
            if elapsed > RegressionThresholds::RESTART_OVERHEAD_MAX {
                violations.push(format!(
                    "restart-overhead exceeded {}ms threshold: {}ms",
                    RegressionThresholds::RESTART_OVERHEAD_MAX.as_millis(),
                    elapsed.as_millis()
                ));
            }
        }

        if let Some(finished) = self.runtime_finished {
            let elapsed = finished.duration_since(self.runtime_start);
            if elapsed > RegressionThresholds::TOTAL_RUNTIME_MAX {
                violations.push(format!(
                    "total-runtime exceeded {}ms threshold: {}ms",
                    RegressionThresholds::TOTAL_RUNTIME_MAX.as_millis(),
                    elapsed.as_millis()
                ));
            }
        }

        violations
    }
}

impl RuntimeStatus {
    pub(in crate::runtime) fn record_timing_proxy_admitted(&mut self) {
        self.timing.record_proxy_admitted();
    }

    pub(in crate::runtime) fn record_timing_service_ready(&mut self, is_resumed: bool) {
        self.timing.record_service_ready(is_resumed);
    }

    pub(in crate::runtime) fn record_timing_transport_stage(&mut self, stage: TransportLifecycleStage) {
        self.timing.record_transport_stage(stage);
    }

    pub(in crate::runtime) fn record_timing_protocol_registration(&mut self) {
        self.timing.record_protocol_registration();
    }

    pub(in crate::runtime) fn record_timing_readiness_reached(&mut self) {
        self.timing.record_readiness_reached();
    }

    pub(in crate::runtime) fn record_timing_restart(&mut self) {
        self.timing.record_restart();
    }

    pub(in crate::runtime) fn record_timing_finished(&mut self) {
        self.timing.record_finished();
    }

    /// Emit machine-readable performance evidence and threshold validation.
    pub(in crate::runtime) fn record_performance_evidence(&mut self) {
        let evidence_lines = self.timing.evidence_lines();
        for line in &evidence_lines {
            self.summary_lines.push(line.clone());
        }

        let violations = self.timing.threshold_violations();
        if violations.is_empty() {
            self.summary_lines.push("perf-threshold-gate: pass".to_owned());
        } else {
            self.summary_lines
                .push(format!("perf-threshold-gate: fail ({})", violations.len()));
            for v in &violations {
                self.summary_lines.push(format!("perf-threshold-violation: {v}"));
            }
        }
    }
}
