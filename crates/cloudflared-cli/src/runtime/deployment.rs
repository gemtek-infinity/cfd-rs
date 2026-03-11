use std::{env, fs};

use super::{
    ApplicationRuntime, FROZEN_TARGET_TRIPLE, GLIBC_RUNTIME_MARKERS, RuntimeServiceFactory,
    TRANSPORT_CRYPTO_LANE,
};

impl<F> ApplicationRuntime<F>
where
    F: RuntimeServiceFactory,
{
    pub(super) fn record_security_compliance_boundary(&mut self) -> Result<(), String> {
        self.status.push_summary(format!(
            "security-boundary: runtime-crypto-surface=transport-tls-only lane={TRANSPORT_CRYPTO_LANE}"
        ));
        self.status.push_summary(
            "security-boundary-claims: bounded-surface-only, not-whole-program, not-certification",
        );
        self.status.push_summary(format!(
            "security-build-contract: target={FROZEN_TARGET_TRIPLE} \
             pingora-role=application-layer-above-transport"
        ));
        self.status.push_summary(
            "security-deployment-contract: linux-gnu-glibc supervised-host-service systemd-expected \
             bare-metal-first",
        );

        self.validate_deployment_contract()?;

        let systemd = if is_systemd_supervision_detected() {
            "detected"
        } else {
            "not-detected"
        };
        self.status.push_summary(format!(
            "security-supervision-signal: {systemd} (systemd expected by deployment contract)"
        ));

        Ok(())
    }

    fn validate_deployment_contract(&mut self) -> Result<(), String> {
        if !cfg!(target_os = "linux") {
            return Err(format!(
                "security/compliance operational boundary requires Linux host runtime, current target_os={} ",
                env::consts::OS
            ));
        }

        if !cfg!(target_arch = "x86_64") {
            return Err(format!(
                "security/compliance operational boundary requires x86_64 host runtime, current \
                 target_arch={} ",
                env::consts::ARCH
            ));
        }

        if !cfg!(target_env = "gnu") {
            return Err(
                "security/compliance operational boundary requires GNU/glibc build contract for the \
                 admitted lane"
                    .to_owned(),
            );
        }

        if !glibc_runtime_marker_present(GLIBC_RUNTIME_MARKERS) {
            return Err(format!(
                "security/compliance operational boundary requires GNU/glibc host runtime markers; none \
                 found in {}",
                GLIBC_RUNTIME_MARKERS.join(", ")
            ));
        }

        self.status
            .push_summary("security-host-contract: linux-x86_64-gnu-glibc markers present");

        Ok(())
    }
}

pub(super) fn glibc_runtime_marker_present(candidates: &[&str]) -> bool {
    candidates.iter().any(|path| fs::metadata(path).is_ok())
}

fn is_systemd_supervision_detected() -> bool {
    env::var_os("INVOCATION_ID").is_some()
        || env::var_os("NOTIFY_SOCKET").is_some()
        || env::var_os("JOURNAL_STREAM").is_some()
}
