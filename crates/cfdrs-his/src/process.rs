//! Graceful process restart and socket inheritance.
//!
//! Covers HIS-073, HIS-074.
//!
//! Go uses `facebookgo/grace/gracenet` for listener inheritance across
//! restarts. Deferred to Host and Runtime Foundation.

/// Trait for graceful restart with listener inheritance.
///
/// Go: `gracenet.Net` manages listener file descriptors across
/// `os.StartProcess()` restarts.
pub trait GracefulRestart: Send + Sync {
    /// Initiate a graceful restart by starting a new process and
    /// passing listener file descriptors.
    fn restart(&self) -> cfdrs_shared::Result<()>;
}

/// Stub graceful restart.
pub struct StubGracefulRestart;

impl GracefulRestart for StubGracefulRestart {
    fn restart(&self) -> cfdrs_shared::Result<()> {
        Err(cfdrs_shared::ConfigError::deferred("graceful restart"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_restart_returns_deferred() {
        let restart = StubGracefulRestart;
        assert!(restart.restart().is_err());
    }
}
