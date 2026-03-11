//! Phase 4.4: Deployment evidence emission.
//!
//! Emits structured `deploy-*` evidence lines at runtime finish,
//! analogous to `perf-*` (Phase 4.2) and `failure-*` (Phase 4.3).
//!
//! What this proves:
//! - the deployment contract was validated at runtime startup
//! - host assumptions (Linux, GNU/glibc, x86_64) were checked
//! - systemd supervision was probed
//! - config and binary origins are explicit
//! - known deployment gaps are honestly declared
//! - operational caveats are explicit and reviewable
//!
//! What this does not prove:
//! - real systemd unit integration (no unit file shipped)
//! - real package manager integration (no installer exists)
//! - container deployment (not part of the alpha contract)
//! - log rotation or journal integration
//! - automatic updates or rollback

use super::RuntimeStatus;

impl RuntimeStatus {
    /// Emit structured deployment evidence at runtime finish.
    ///
    /// Produces machine-readable `deploy-*` lines that summarize the
    /// deployment contract validation observed during this runtime
    /// execution. Called from `finish()` alongside performance and
    /// failure evidence.
    pub(in crate::runtime) fn record_deployment_evidence(&mut self) {
        self.summary_lines
            .push("deploy-contract: linux-x86_64-gnu-glibc bare-metal-first systemd-expected".to_owned());

        let host_passed = self.deployment.host_validated;
        self.summary_lines.push(format!(
            "deploy-host-validation: {}",
            if host_passed { "passed" } else { "failed" }
        ));

        self.summary_lines.push(format!(
            "deploy-glibc-markers: {}",
            if self.deployment.glibc_present {
                "present"
            } else {
                "absent"
            }
        ));

        self.summary_lines.push(format!(
            "deploy-systemd-supervision: {}",
            if self.deployment.systemd_detected {
                "detected"
            } else {
                "not-detected"
            }
        ));

        self.summary_lines.push(format!(
            "deploy-binary-path: {}",
            std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".to_owned())
        ));

        self.summary_lines.push(format!(
            "deploy-config-path: {}",
            self.deployment.config_path.as_deref().unwrap_or("not-recorded")
        ));

        self.summary_lines.push(
            "deploy-filesystem-contract: operator-managed (executable, config, credentials, and logs are \
             explicit host-path concerns)"
                .to_owned(),
        );

        self.summary_lines.push(
            "deploy-known-gaps: no-systemd-unit, no-installer, no-container-image, no-updater, \
             no-log-rotation"
                .to_owned(),
        );

        self.summary_lines.push(
            "deploy-operational-caveats: alpha-only, narrow-origin-path(http_status), no-rpc-registration, \
             no-incoming-streams, no-config-reload"
                .to_owned(),
        );

        self.summary_lines.push(
            "deploy-evidence-scope: in-process-contract-validation (real systemd integration, \
             package-manager delivery, container support, and log-rotation are deferred)"
                .to_owned(),
        );
    }
}
