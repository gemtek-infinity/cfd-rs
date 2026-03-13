#![forbid(unsafe_code)]
//! Cross-domain config types, error taxonomy, in-memory parsing,
//! normalization, ingress rule representation, and credentials surface.
//!
//! Config modules live under `config/`. Parity artifact reporting lives
//! under `artifact/`. Filesystem discovery IO lives in `cfdrs-his`.
//! CDC-facing contracts live in `cfdrs-cdc`.

// cfdrs-his is used by integration tests and examples, not by the library
// itself.
#[cfg(test)]
extern crate cfdrs_his as _;

pub mod artifact;
pub mod config;

// Re-export the full config surface at the crate root for ergonomic access.
pub use self::config::*;
