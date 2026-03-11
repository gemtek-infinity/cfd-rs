//! Phase 3.5 + 4.1 + 5.1: Wire/protocol boundary between transport and proxy.
//!
//! Owns the protocol-level boundary that bridges the QUIC transport
//! session to the Pingora proxy layer. Transport owns QUIC establishment.
//! Proxy owns Pingora request dispatch. This module owns the explicit
//! handoff between them.
//!
//! The admitted alpha path uses the cloudflare tunnel wire protocol:
//! - client-initiated bidi stream 0 is the control/registration stream
//! - edge-initiated QUIC data streams carry incoming requests as ConnectRequest
//!   messages dispatched through the proxy layer
//! - registration RPC sends credentials and receives connection details over
//!   the control stream

use cloudflared_proto::stream::ConnectRequest;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use uuid::Uuid;

/// QUIC stream ID for the tunnel control/registration stream.
///
/// The cloudflare tunnel wire protocol uses the first client-initiated
/// bidirectional stream for tunnel registration.
pub(crate) const CONTROL_STREAM_ID: u64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProtocolBridgeState {
    BridgeUnavailable,
    BridgeCreated,
    RegistrationSent,
    RegistrationObserved,
    BridgeClosed,
}

impl ProtocolBridgeState {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::BridgeUnavailable => "bridge-unavailable",
            Self::BridgeCreated => "bridge-created",
            Self::RegistrationSent => "registration-sent",
            Self::RegistrationObserved => "registration-observed",
            Self::BridgeClosed => "bridge-closed",
        }
    }
}

impl std::fmt::Display for ProtocolBridgeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Events that cross the wire/protocol boundary from transport to proxy.
///
/// This is the explicit handoff surface. Transport sends these events
/// after crossing protocol boundaries. The proxy layer receives them
/// for its owned lifecycle reporting and request dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // Phase 5.1: variants wired incrementally
pub(crate) enum ProtocolEvent {
    /// The transport has opened the control stream and reached the
    /// protocol registration boundary.
    Registered { peer: SocketAddr },

    /// An incoming QUIC data stream carries a request from the edge.
    ///
    /// The proxy layer dispatches this through ingress matching to
    /// the matched origin service.
    IncomingStream { stream_id: u64, request: ConnectRequest },

    /// Registration completed with connection details from the edge.
    RegistrationComplete { conn_uuid: Uuid, location: String },
}

/// Create the explicit protocol bridge between transport and proxy.
///
/// Returns the transport-owned sender and the proxy-owned receiver.
/// The runtime creates this bridge and hands the endpoints to the
/// services it supervises.
pub(crate) fn protocol_bridge() -> (ProtocolSender, ProtocolReceiver) {
    let (tx, rx) = mpsc::channel(4);
    (ProtocolSender(tx), ProtocolReceiver(rx))
}

/// Transport-owned end of the protocol bridge.
///
/// Cloneable so the runtime service factory can provide a sender
/// to each transport service instance across restarts.
#[derive(Debug, Clone)]
pub(crate) struct ProtocolSender(mpsc::Sender<ProtocolEvent>);

impl ProtocolSender {
    /// Send a protocol event across the wire/protocol boundary.
    ///
    /// Returns an error when the proxy-owned receiver is no longer
    /// available so the transport can report that failure boundary
    /// explicitly instead of losing the signal silently.
    pub(crate) async fn send(&self, event: ProtocolEvent) -> Result<(), String> {
        self.0
            .send(event)
            .await
            .map_err(|_| String::from("proxy-side protocol bridge receiver is no longer available"))
    }
}

/// Proxy-owned end of the protocol bridge.
#[derive(Debug)]
pub(crate) struct ProtocolReceiver(mpsc::Receiver<ProtocolEvent>);

impl ProtocolReceiver {
    /// Receive the next protocol event from the transport layer.
    ///
    /// Returns `None` when all senders have been dropped.
    pub(crate) async fn recv(&mut self) -> Option<ProtocolEvent> {
        self.0.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bridge_delivers_registered_event() {
        let (sender, mut receiver) = protocol_bridge();

        sender
            .send(ProtocolEvent::Registered {
                peer: "127.0.0.1:7844".parse().expect("socket addr should parse"),
            })
            .await
            .expect("bridge sender should deliver event while receiver is alive");

        assert_eq!(
            receiver.recv().await,
            Some(ProtocolEvent::Registered {
                peer: "127.0.0.1:7844".parse().expect("socket addr should parse"),
            })
        );
    }

    #[tokio::test]
    async fn bridge_returns_none_after_all_senders_dropped() {
        let (sender, mut receiver) = protocol_bridge();
        drop(sender);

        assert_eq!(receiver.recv().await, None);
    }

    #[tokio::test]
    async fn sender_clone_keeps_bridge_alive() {
        let (sender, mut receiver) = protocol_bridge();
        let sender_clone = sender.clone();
        drop(sender);

        sender_clone
            .send(ProtocolEvent::Registered {
                peer: "10.0.0.1:7844".parse().expect("socket addr should parse"),
            })
            .await
            .expect("bridge sender clone should deliver event while receiver is alive");

        assert_eq!(
            receiver.recv().await,
            Some(ProtocolEvent::Registered {
                peer: "10.0.0.1:7844".parse().expect("socket addr should parse"),
            })
        );
    }

    #[test]
    fn control_stream_id_is_first_client_bidi() {
        // QUIC stream ID 0 is client-initiated bidirectional,
        // matching the cloudflare tunnel protocol expectation.
        assert_eq!(CONTROL_STREAM_ID % 4, 0);
    }
}
