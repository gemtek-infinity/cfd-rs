#![forbid(unsafe_code)]

//! Cloudflare-facing RPC contracts, wire and stream contracts, management
//! protocol, metrics and readiness contracts, Cloudflare API boundaries,
//! log-streaming, and CDC-owned codec logic.
//!
//! This crate owns the 44-row CDC parity surface: all interactions between
//! cloudflared and Cloudflare-managed services including registration,
//! stream framing, management routes, and REST API client behavior.
//!
//! Wire-format types match the behavioral contract from
//! `baseline-2026.2.0/tunnelrpc/pogs/` and
//! `baseline-2026.2.0/connection/`.

pub mod registration;
pub mod stream;
pub(crate) mod stream_contract;

pub use registration::{ConnectionDetails, ConnectionOptions, TunnelAuth};
pub use stream::{ConnectRequest, ConnectResponse, ConnectionType, Metadata};
