pub(super) const DEPLOY_CONTRACT_KEY: &str = "deploy-contract";
pub(super) const DEPLOY_HOST_VALIDATION_KEY: &str = "deploy-host-validation";
pub(super) const DEPLOY_GLIBC_MARKERS_KEY: &str = "deploy-glibc-markers";
pub(super) const DEPLOY_SYSTEMD_SUPERVISION_KEY: &str = "deploy-systemd-supervision";
pub(super) const DEPLOY_BINARY_PATH_KEY: &str = "deploy-binary-path";
pub(super) const DEPLOY_CONFIG_PATH_KEY: &str = "deploy-config-path";
pub(super) const DEPLOY_FILESYSTEM_CONTRACT_KEY: &str = "deploy-filesystem-contract";
pub(super) const DEPLOY_KNOWN_GAPS_KEY: &str = "deploy-known-gaps";
pub(super) const DEPLOY_OPERATIONAL_CAVEATS_KEY: &str = "deploy-operational-caveats";
pub(super) const DEPLOY_EVIDENCE_SCOPE_KEY: &str = "deploy-evidence-scope";

pub(super) const DEPLOY_CONTRACT_VALUE: &str = "linux-x86_64-gnu-glibc bare-metal-first systemd-expected";
pub(super) const DEPLOY_FILESYSTEM_CONTRACT_VALUE: &str =
    "operator-managed (executable, config, credentials, and logs are explicit host-path concerns)";
pub(super) const DEPLOY_EVIDENCE_SCOPE_VALUE: &str = "in-process-contract-validation (real systemd \
                                                      integration, package-manager delivery, container \
                                                      support, and log-rotation are deferred)";

pub(super) const DEPLOY_KNOWN_GAPS: &[&str] = &[
    "no-systemd-unit",
    "no-installer",
    "no-container-image",
    "no-log-rotation",
];

pub(super) const DEPLOY_OPERATIONAL_CAVEATS: &[&str] = &[
    "alpha-only",
    "limited-origin-dispatch(http_status+hello_world+http-wired-no-proxy)",
    "no-capnp-registration-rpc",
    "no-origin-cert-registration-content",
    "no-stream-roundtrip",
    "config-watcher-notify-only",
];

pub(super) fn summary_line(key: &str, value: &str) -> String {
    format!("{key}: {value}")
}

pub(super) fn host_validation_value(host_validated: bool) -> &'static str {
    if host_validated { "passed" } else { "failed" }
}

pub(super) fn glibc_markers_value(glibc_present: bool) -> &'static str {
    if glibc_present { "present" } else { "absent" }
}

pub(super) fn systemd_supervision_value(systemd_detected: bool) -> &'static str {
    if systemd_detected {
        "detected"
    } else {
        "not-detected"
    }
}

pub(super) fn known_gaps_value() -> String {
    DEPLOY_KNOWN_GAPS.join(", ")
}

pub(super) fn operational_caveats_value() -> String {
    DEPLOY_OPERATIONAL_CAVEATS.join(", ")
}
