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

// Cap'n Proto generated bindings from frozen baseline schemas.
// These are the exact wire-format types the Cloudflare edge expects.
#[allow(clippy::all, clippy::unwrap_used, clippy::dbg_macro, clippy::todo, unused)]
pub mod tunnelrpc_capnp {
    include!(concat!(env!("OUT_DIR"), "/tunnelrpc_capnp.rs"));
}
#[allow(clippy::all, clippy::unwrap_used, clippy::dbg_macro, clippy::todo, unused)]
pub mod quic_metadata_protocol_capnp {
    include!(concat!(env!("OUT_DIR"), "/quic_metadata_protocol_capnp.rs"));
}

pub mod features;
pub mod protocol;
pub mod registration;
pub mod stream;
pub mod stream_codec;
pub(crate) mod stream_contract;

pub use registration::{
    ClientInfo, ConnectionDetails, ConnectionError, ConnectionOptions, ConnectionResponse,
    RegisterConnectionRequest, RegisterUdpSessionRequest, RegisterUdpSessionResponse, TunnelAuth,
    UnregisterUdpSessionRequest, UpdateConfigurationRequest, UpdateConfigurationResponse,
    UpdateLocalConfigurationRequest,
};
pub use stream::{ConnectRequest, ConnectResponse, ConnectionType, Metadata};
