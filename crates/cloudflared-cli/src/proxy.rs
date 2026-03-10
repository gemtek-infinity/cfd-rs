//! Phase 3.4a: Pingora proxy-layer seam admission.
//!
//! This module is the owned entry point for Pingora in the production-alpha
//! path. All direct Pingora types and API usage are confined here. The rest
//! of the binary does not depend on Pingora crates directly.
//!
//! ADR-0003 governs Pingora scope: application-layer proxy above the quiche
//! transport lane, not a transport replacement.
//!
//! Admitted: dependency path and seam location.
//! Deferred to 3.4b: runtime lifecycle integration, transport → proxy handoff.
//! Deferred to 3.4c: origin-facing proxy behavior via `pingora-proxy`.

use std::marker::PhantomData;

/// Owned boundary for Pingora proxy-layer admission.
///
/// Confines the Pingora dependency surface to this module. The type witness
/// binds the admitted Pingora HTTP request type without allocating, proving
/// the dependency path compiles and the seam location is intentional.
///
/// Construction is the only admitted operation at this phase. Proxy behavior
/// and runtime wiring are deferred to later slices.
pub(crate) struct PingoraProxySeam {
    _marker: PhantomData<pingora_http::RequestHeader>,
}

impl PingoraProxySeam {
    /// Construct the proxy seam boundary for the production-alpha path.
    pub(crate) fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
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
}
