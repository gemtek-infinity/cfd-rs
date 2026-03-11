#![forbid(unsafe_code)]

//! Wire-format and RPC boundary for the cloudflare tunnel protocol.
//!
//! This crate owns the types that cross the QUIC stream boundary between
//! the edge and the tunnel client. The Go baseline uses Cap'n Proto for
//! wire encoding; the Rust rewrite defines the same logical types here
//! and will add wire codec support as needed.
//!
//! All types match the behavioral contract from
//! `baseline-2026.2.0/old-impl/tunnelrpc/pogs/` and
//! `baseline-2026.2.0/old-impl/connection/`.

pub mod registration;
pub mod stream;

pub use registration::{ConnectionDetails, ConnectionOptions, TunnelAuth};
pub use stream::{ConnectRequest, ConnectResponse, ConnectionType, Metadata};
