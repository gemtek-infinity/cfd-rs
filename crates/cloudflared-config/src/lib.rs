#![forbid(unsafe_code)]

//! Config and credential parsing boundary for the rewrite.
//!
//! The accepted first subsystem slice will land here:
//!
//! - config discovery and parsing
//! - credentials handling
//! - ingress normalization
//!
//! Intentionally minimal for now. No subsystem behavior is implemented yet.
