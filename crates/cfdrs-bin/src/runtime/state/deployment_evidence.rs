//! Phase 4.4: Deployment evidence emission.
//!
//! Emits structured deployment evidence lines at runtime finish.

use super::RuntimeStatus;
use super::deployment_contract::{
    DEPLOY_BINARY_PATH_KEY, DEPLOY_CONFIG_PATH_KEY, DEPLOY_CONTRACT_KEY, DEPLOY_CONTRACT_VALUE,
    DEPLOY_EVIDENCE_SCOPE_KEY, DEPLOY_EVIDENCE_SCOPE_VALUE, DEPLOY_FILESYSTEM_CONTRACT_KEY,
    DEPLOY_FILESYSTEM_CONTRACT_VALUE, DEPLOY_GLIBC_MARKERS_KEY, DEPLOY_HOST_VALIDATION_KEY,
    DEPLOY_KNOWN_GAPS_KEY, DEPLOY_OPERATIONAL_CAVEATS_KEY, DEPLOY_SYSTEMD_SUPERVISION_KEY,
    glibc_markers_value, host_validation_value, known_gaps_value, operational_caveats_value, summary_line,
    systemd_supervision_value,
};

impl RuntimeStatus {
    /// Emit structured deployment evidence at runtime finish.
    pub(in crate::runtime) fn record_deployment_evidence(&mut self) {
        self.summary_lines
            .push(summary_line(DEPLOY_CONTRACT_KEY, DEPLOY_CONTRACT_VALUE));

        self.summary_lines.push(summary_line(
            DEPLOY_HOST_VALIDATION_KEY,
            host_validation_value(self.deployment.host_validated),
        ));

        self.summary_lines.push(summary_line(
            DEPLOY_GLIBC_MARKERS_KEY,
            glibc_markers_value(self.deployment.glibc_present),
        ));

        self.summary_lines.push(summary_line(
            DEPLOY_SYSTEMD_SUPERVISION_KEY,
            systemd_supervision_value(self.deployment.systemd_detected),
        ));

        self.summary_lines.push(summary_line(
            DEPLOY_BINARY_PATH_KEY,
            &std::env::current_exe()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "unknown".to_owned()),
        ));

        self.summary_lines.push(summary_line(
            DEPLOY_CONFIG_PATH_KEY,
            self.deployment.config_path.as_deref().unwrap_or("not-recorded"),
        ));

        self.summary_lines.push(summary_line(
            DEPLOY_FILESYSTEM_CONTRACT_KEY,
            DEPLOY_FILESYSTEM_CONTRACT_VALUE,
        ));

        self.summary_lines
            .push(summary_line(DEPLOY_KNOWN_GAPS_KEY, &known_gaps_value()));

        self.summary_lines.push(summary_line(
            DEPLOY_OPERATIONAL_CAVEATS_KEY,
            &operational_caveats_value(),
        ));

        self.summary_lines.push(summary_line(
            DEPLOY_EVIDENCE_SCOPE_KEY,
            DEPLOY_EVIDENCE_SCOPE_VALUE,
        ));
    }
}
