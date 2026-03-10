//! Phase 3.5: Wire/protocol boundary between transport and proxy.
//!
//! Owns the protocol-level boundary that bridges the QUIC transport
//! session to the Pingora proxy layer. Transport owns QUIC establishment.
//! Proxy owns Pingora request dispatch. This module owns the explicit
//! handoff between them.
//!
//! The admitted alpha path uses the cloudflare tunnel wire protocol:
//! - client-initiated bidi stream 0 is the control/registration stream
//! - later slices will carry HTTP requests as edge-initiated QUIC streams
//! - registration RPC content (capnp) remains deferred to later slices

use tokio::sync::mpsc;

/// QUIC stream ID for the tunnel control/registration stream.
///
/// The cloudflare tunnel wire protocol uses the first client-initiated
/// bidirectional stream for tunnel registration.
pub(crate) const CONTROL_STREAM_ID: u64 = 0;

/// Events that cross the wire/protocol boundary from transport to proxy.
///
/// This is the explicit handoff surface. Transport sends these events
/// after crossing protocol boundaries. The proxy layer receives them
/// to coordinate its readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProtocolEvent {
    /// The transport has opened the control stream and reached the
    /// protocol registration boundary.
    ///
    /// Later slices will carry real registration RPC and incoming
    /// request streams through this boundary.
    Registered { peer: String },
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
    pub(crate) async fn send(&self, event: ProtocolEvent) {
        // Best-effort: if the receiver has been dropped, the event
        // is silently lost. The runtime owns shutdown ordering.
        let _ = self.0.send(event).await;
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
                peer: "127.0.0.1:7844".to_owned(),
            })
            .await;

        assert_eq!(
            receiver.recv().await,
            Some(ProtocolEvent::Registered {
                peer: "127.0.0.1:7844".to_owned(),
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
                peer: "10.0.0.1:7844".to_owned(),
            })
            .await;

        assert_eq!(
            receiver.recv().await,
            Some(ProtocolEvent::Registered {
                peer: "10.0.0.1:7844".to_owned(),
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
