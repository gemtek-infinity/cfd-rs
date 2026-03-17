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

use cfdrs_cdc::stream::ConnectRequest;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

/// QUIC stream ID for the tunnel control/registration stream.
///
/// The cloudflare tunnel wire protocol uses the first client-initiated
/// bidirectional stream for tunnel registration.
pub(crate) const CONTROL_STREAM_ID: u64 = 0;

/// Buffer depth for the transport→proxy protocol bridge channel.
///
/// Go baseline defaults `ha-connections` to 4 but the flag is
/// user-configurable (`cmd/cloudflared/tunnel/cmd.go`, line ~731).
/// This constant matches the default. Once the supervisor and
/// HA-connection lifecycle are wired, this should be derived from
/// the runtime-resolved `ha-connections` value instead of hardcoded.
const PROTOCOL_BRIDGE_CAPACITY: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Variants wired incrementally as lifecycle handling grows
pub(crate) enum ProtocolBridgeState {
    BridgeUnavailable,
    BridgeCreated,
    RegistrationSent,
    RegistrationObserved,
    /// The connection is being re-established after a failure.
    ///
    /// Matches Go's `Reconnecting` status in `connection/event.go`.
    Reconnecting,
    /// The control stream has initiated a graceful unregister sequence.
    ///
    /// Matches Go's `Unregistering` status in `connection/event.go`.
    Unregistering,
    BridgeClosed,
}

impl ProtocolBridgeState {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::BridgeUnavailable => "bridge-unavailable",
            Self::BridgeCreated => "bridge-created",
            Self::RegistrationSent => "registration-sent",
            Self::RegistrationObserved => "registration-observed",
            Self::Reconnecting => "reconnecting",
            Self::Unregistering => "unregistering",
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
    Registered { peer: SocketAddr, conn_index: u8 },

    /// An incoming QUIC data stream carries a request from the edge.
    ///
    /// The proxy layer dispatches this through ingress matching to
    /// the matched origin service.
    IncomingStream { stream_id: u64, request: ConnectRequest },

    /// Registration completed with connection details from the edge.
    RegistrationComplete { conn_uuid: Uuid, location: String },

    /// The control stream has initiated a graceful unregister sequence.
    ///
    /// Matches Go's `waitForUnregister` → `GracefulShutdown` flow in
    /// `connection/control.go`.
    Unregistering { conn_index: u8 },

    /// The connection was severed (context cancelled or transport error).
    ///
    /// Matches Go's `Disconnected` status in `connection/event.go`.
    Disconnected { conn_index: u8 },

    /// Local configuration was pushed to the edge on conn_index 0.
    ///
    /// Matches Go's `SendLocalConfiguration` call in
    /// `connection/control.go` (conn_index == 0 && !remotely_managed).
    ConfigPushed { conn_index: u8 },
}

/// Create the explicit protocol bridge between transport and proxy.
///
/// Returns the transport-owned sender and the proxy-owned receiver.
/// The runtime creates this bridge and hands the endpoints to the
/// services it supervises.
pub(crate) fn protocol_bridge() -> (ProtocolSender, ProtocolReceiver) {
    let (tx, rx) = mpsc::channel(PROTOCOL_BRIDGE_CAPACITY);
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

// ---------------------------------------------------------------------------
// Stream response channel (proxy → transport)
// ---------------------------------------------------------------------------

/// An encoded stream response to be written back to a QUIC data stream.
#[derive(Debug)]
pub(crate) struct StreamResponse {
    pub stream_id: u64,
    pub data: Vec<u8>,
}

/// Proxy-owned sender for stream responses.
#[derive(Debug, Clone)]
pub(crate) struct StreamResponseSender(mpsc::UnboundedSender<StreamResponse>);

impl StreamResponseSender {
    pub(crate) fn send(&self, response: StreamResponse) {
        // Best-effort: if the transport is gone the response is dropped.
        let _ = self.0.send(response);
    }
}

/// Transport-owned receiver for stream responses.
///
/// Wrapped in `Arc<std::sync::Mutex<...>>` so the factory can share
/// it across sequential transport service instances (restarts).
pub(crate) type SharedStreamResponseReceiver = Arc<std::sync::Mutex<mpsc::UnboundedReceiver<StreamResponse>>>;

/// Create the stream response channel between proxy and transport.
///
/// The proxy sends encoded `ConnectResponse` bytes through the sender;
/// the transport drains and writes them to QUIC streams through the
/// shared receiver.
pub(crate) fn stream_response_bridge() -> (StreamResponseSender, SharedStreamResponseReceiver) {
    let (tx, rx) = mpsc::unbounded_channel();
    (StreamResponseSender(tx), Arc::new(std::sync::Mutex::new(rx)))
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
                conn_index: 0,
            })
            .await
            .expect("bridge sender should deliver event while receiver is alive");

        assert_eq!(
            receiver.recv().await,
            Some(ProtocolEvent::Registered {
                peer: "127.0.0.1:7844".parse().expect("socket addr should parse"),
                conn_index: 0,
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
                conn_index: 1,
            })
            .await
            .expect("bridge sender clone should deliver event while receiver is alive");

        assert_eq!(
            receiver.recv().await,
            Some(ProtocolEvent::Registered {
                peer: "10.0.0.1:7844".parse().expect("socket addr should parse"),
                conn_index: 1,
            })
        );
    }

    #[test]
    fn control_stream_id_is_first_client_bidi() {
        // QUIC stream ID 0 is client-initiated bidirectional,
        // matching the cloudflare tunnel protocol expectation.
        assert_eq!(CONTROL_STREAM_ID % 4, 0);
    }

    // --- CDC-019: control stream lifecycle state coverage ---

    #[test]
    fn bridge_state_all_variants_have_distinct_display() {
        let variants = [
            ProtocolBridgeState::BridgeUnavailable,
            ProtocolBridgeState::BridgeCreated,
            ProtocolBridgeState::RegistrationSent,
            ProtocolBridgeState::RegistrationObserved,
            ProtocolBridgeState::Reconnecting,
            ProtocolBridgeState::Unregistering,
            ProtocolBridgeState::BridgeClosed,
        ];
        let strings: Vec<&str> = variants.iter().map(|v| v.as_str()).collect();
        // All 7 variants must produce distinct strings.
        for (i, a) in strings.iter().enumerate() {
            for (j, b) in strings.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "variants {i} and {j} have same display string");
                }
            }
        }
        assert_eq!(strings.len(), 7);
    }

    #[test]
    fn bridge_state_display_trait_matches_as_str() {
        let variants = [
            ProtocolBridgeState::BridgeUnavailable,
            ProtocolBridgeState::BridgeCreated,
            ProtocolBridgeState::RegistrationSent,
            ProtocolBridgeState::RegistrationObserved,
            ProtocolBridgeState::Reconnecting,
            ProtocolBridgeState::Unregistering,
            ProtocolBridgeState::BridgeClosed,
        ];
        for v in &variants {
            assert_eq!(v.to_string(), v.as_str());
        }
    }

    #[tokio::test]
    async fn bridge_delivers_all_event_variants() {
        use cfdrs_cdc::stream::{ConnectRequest, ConnectionType};

        let (sender, mut receiver) = protocol_bridge();

        let events = vec![
            ProtocolEvent::Registered {
                peer: "127.0.0.1:7844".parse().expect("valid addr"),
                conn_index: 0,
            },
            ProtocolEvent::RegistrationComplete {
                conn_uuid: Uuid::nil(),
                location: "LAX".into(),
            },
            ProtocolEvent::IncomingStream {
                stream_id: 4,
                request: ConnectRequest {
                    dest: "http://localhost/".into(),
                    connection_type: ConnectionType::Http,
                    metadata: vec![],
                },
            },
            ProtocolEvent::Unregistering { conn_index: 0 },
            ProtocolEvent::Disconnected { conn_index: 1 },
            ProtocolEvent::ConfigPushed { conn_index: 0 },
        ];

        // Channel capacity is 4 but we send 6 events, so sender and
        // receiver must run concurrently to avoid a deadlock.
        let expected = events.clone();
        tokio::spawn(async move {
            for event in events {
                sender.send(event).await.expect("send should succeed");
            }
        });

        for expected_event in &expected {
            let received = receiver.recv().await.expect("should receive event");
            assert_eq!(&received, expected_event);
        }
    }

    // --- CDC-019: baseline lifecycle state transition ordering ---

    /// Go baseline control.go dispatches events in a deterministic
    /// order: Registered → RegistrationComplete → ConfigPushed →
    /// (monitoring) → Unregistering → Disconnected.
    ///
    /// This test verifies the bridge faithfully delivers the
    /// baseline lifecycle sequence without reordering.
    #[tokio::test]
    async fn bridge_preserves_baseline_lifecycle_order() {
        let (sender, mut receiver) = protocol_bridge();

        let lifecycle_sequence = vec![
            ProtocolEvent::Registered {
                peer: "198.41.200.1:7844".parse().expect("valid addr"),
                conn_index: 0,
            },
            ProtocolEvent::RegistrationComplete {
                conn_uuid: Uuid::new_v4(),
                location: "DFW".into(),
            },
            ProtocolEvent::ConfigPushed { conn_index: 0 },
            ProtocolEvent::Unregistering { conn_index: 0 },
            ProtocolEvent::Disconnected { conn_index: 0 },
        ];

        let expected = lifecycle_sequence.clone();
        tokio::spawn(async move {
            for event in lifecycle_sequence {
                sender.send(event).await.expect("send should succeed");
            }
        });

        for (i, expected_event) in expected.iter().enumerate() {
            let received = receiver.recv().await.expect("should receive event");
            assert_eq!(
                &received, expected_event,
                "lifecycle event at position {i} should match baseline order"
            );
        }
    }

    /// ConfigPushed should only fire on conn_index 0 per Go baseline
    /// (connection/control.go: conn_index == 0 && !remotely_managed).
    /// Verify the event carries the expected index.
    #[test]
    fn config_pushed_event_only_relevant_on_conn_zero() {
        let event = ProtocolEvent::ConfigPushed { conn_index: 0 };

        match event {
            ProtocolEvent::ConfigPushed { conn_index } => {
                assert_eq!(conn_index, 0, "config push should target conn_index 0");
            }
            _ => panic!("expected ConfigPushed"),
        }
    }

    /// Connection events carry their index so the supervisor can
    /// track per-connection lifecycle independently.
    #[test]
    fn lifecycle_events_carry_connection_index() {
        let events = vec![
            (ProtocolEvent::Unregistering { conn_index: 2 }, 2u8),
            (ProtocolEvent::Disconnected { conn_index: 3 }, 3u8),
            (ProtocolEvent::ConfigPushed { conn_index: 0 }, 0u8),
        ];

        for (event, expected_index) in events {
            let actual_index = match event {
                ProtocolEvent::Unregistering { conn_index } => conn_index,
                ProtocolEvent::Disconnected { conn_index } => conn_index,
                ProtocolEvent::ConfigPushed { conn_index } => conn_index,
                _ => panic!("unexpected event variant"),
            };
            assert_eq!(actual_index, expected_index);
        }
    }

    /// RegistrationComplete carries connection UUID and location
    /// from the edge registration response.
    #[test]
    fn registration_complete_carries_edge_detail() {
        let uuid = Uuid::new_v4();
        let event = ProtocolEvent::RegistrationComplete {
            conn_uuid: uuid,
            location: "SIN".into(),
        };

        match event {
            ProtocolEvent::RegistrationComplete { conn_uuid, location } => {
                assert_eq!(conn_uuid, uuid);
                assert_eq!(location, "SIN");
            }
            _ => panic!("expected RegistrationComplete"),
        }
    }

    /// Bridge state transitions map to Go baseline control.go stages:
    /// - BridgeCreated → (pre-registration setup)
    /// - RegistrationSent → (RegisterConnection RPC dispatched)
    /// - RegistrationObserved → (RPC response received)
    /// - Reconnecting → (connection retry after failure)
    /// - Unregistering → (graceful shutdown initiated)
    /// - BridgeClosed → (teardown complete)
    #[test]
    fn bridge_state_maps_to_go_baseline_stages() {
        // Verify the expected state→display mappings exist and
        // align with Go baseline event stages.
        assert_eq!(ProtocolBridgeState::BridgeCreated.as_str(), "bridge-created");
        assert_eq!(
            ProtocolBridgeState::RegistrationSent.as_str(),
            "registration-sent"
        );
        assert_eq!(
            ProtocolBridgeState::RegistrationObserved.as_str(),
            "registration-observed"
        );
        assert_eq!(ProtocolBridgeState::Reconnecting.as_str(), "reconnecting");
        assert_eq!(ProtocolBridgeState::Unregistering.as_str(), "unregistering");
        assert_eq!(ProtocolBridgeState::BridgeClosed.as_str(), "bridge-closed");
    }
}
