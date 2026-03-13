use tracing::info;

use super::RuntimeStatus;

impl RuntimeStatus {
    pub(in super::super) fn record_operability_summary(&mut self) {
        let snapshot = OperabilitySnapshot::from_status(self);
        info!("{}", snapshot.log_line());
        self.summary_lines.push(snapshot.status_line());
        self.summary_lines.push(snapshot.metrics_line());
    }
}

struct OperabilitySnapshot {
    lifecycle: &'static str,
    readiness: &'static str,
    transport: &'static str,
    protocol: &'static str,
    proxy: &'static str,
    restart_attempts: u32,
    proxy_admissions: u32,
    protocol_registrations: u32,
    transport_failures: u32,
    failure_events: u32,
}

impl OperabilitySnapshot {
    fn from_status(status: &RuntimeStatus) -> Self {
        Self {
            lifecycle: status.lifecycle_state.as_str(),
            readiness: status.readiness_state.as_str(),
            transport: status.transport_stage.map_or("not-reported", |s| s.as_str()),
            protocol: status.protocol_state.as_str(),
            proxy: status.proxy_state.map_or("not-reported", |s| s.as_str()),
            restart_attempts: status.restart_attempts,
            proxy_admissions: status.proxy_admissions,
            protocol_registrations: status.protocol_registrations,
            transport_failures: status.transport_failures,
            failure_events: status.failure_events,
        }
    }

    fn status_line(&self) -> String {
        format!(
            "operability-status: lifecycle={} readiness={} transport-stage={} protocol-state={} \
             proxy-state={}",
            self.lifecycle, self.readiness, self.transport, self.protocol, self.proxy
        )
    }

    fn metrics_line(&self) -> String {
        format!(
            "operability-metrics: restart-attempts={} proxy-admissions={} protocol-registrations={} \
             transport-failures={} failure-events={}",
            self.restart_attempts,
            self.proxy_admissions,
            self.protocol_registrations,
            self.transport_failures,
            self.failure_events
        )
    }

    fn log_line(&self) -> String {
        format!(
            "operability-summary lifecycle={} readiness={} transport-stage={} protocol-state={} \
             proxy-state={} restart-attempts={} proxy-admissions={} protocol-registrations={} \
             transport-failures={} failure-events={}",
            self.lifecycle,
            self.readiness,
            self.transport,
            self.protocol,
            self.proxy,
            self.restart_attempts,
            self.proxy_admissions,
            self.protocol_registrations,
            self.transport_failures,
            self.failure_events
        )
    }
}
