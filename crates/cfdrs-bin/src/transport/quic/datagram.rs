//! Datagram session manager and muxer for V3 QUIC datagrams (CDC-040, CDC-041).
//!
//! Implements the [`SessionManager`] trait from `cfdrs-cdc` with an in-memory
//! `HashMap` protected by `RwLock`. This mirrors Go's `sessionManager` in
//! `quic/v3/manager.go`.
//!
//! The dispatch logic parses incoming QUIC datagrams by type discriminator
//! and routes them through the session manager, matching Go's
//! `datagramConn.Serve()` in `quic/v3/muxer.go`.

use std::collections::HashMap;
use std::sync::RwLock;

use cfdrs_cdc::datagram::{
    ConnectionId, DatagramType, RequestId, SessionError, SessionManager, SessionRegistrationResp,
    UdpSessionPayloadDatagram, UdpSessionRegistrationDatagram, UdpSessionRegistrationResponseDatagram,
};

/// In-memory session manager for V3 datagram sessions.
///
/// Tracks registered sessions as `RequestId → ConnectionId` mappings.
/// Matches Go's `sessionManager` struct in `quic/v3/manager.go`:
/// - `register_session`: checks for duplicates, inserts mapping
/// - `get_session`: read-locked lookup
/// - `unregister_session`: removes mapping
///
/// Real UDP origin dialing and flow limiting are deferred to future
/// runtime integration work.
pub(super) struct DatagramSessionManager {
    sessions: RwLock<HashMap<RequestId, ConnectionId>>,
}

impl DatagramSessionManager {
    pub(super) fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Number of active sessions.
    #[cfg(test)]
    fn session_count(&self) -> usize {
        self.sessions.read().expect("session lock poisoned").len()
    }
}

impl SessionManager for DatagramSessionManager {
    fn register_session(
        &self,
        request: &UdpSessionRegistrationDatagram,
        connection_id: ConnectionId,
    ) -> Result<RequestId, SessionError> {
        let mut sessions = self.sessions.write().expect("session lock poisoned");

        if let Some(&existing_conn) = sessions.get(&request.request_id) {
            if existing_conn == connection_id {
                return Err(SessionError::AlreadyRegistered);
            }

            return Err(SessionError::BoundToOtherConn);
        }

        sessions.insert(request.request_id, connection_id);
        Ok(request.request_id)
    }

    fn get_session(&self, request_id: &RequestId) -> Result<ConnectionId, SessionError> {
        let sessions = self.sessions.read().expect("session lock poisoned");
        sessions.get(request_id).copied().ok_or(SessionError::NotFound)
    }

    fn unregister_session(&self, request_id: &RequestId) {
        let mut sessions = self.sessions.write().expect("session lock poisoned");
        sessions.remove(request_id);
    }
}

/// Dispatch a single incoming QUIC datagram through the session manager.
///
/// Returns an optional response datagram to be sent back over the QUIC
/// connection. Matches the dispatch switch in Go's `datagramConn.Serve()`
/// (`quic/v3/muxer.go`).
pub(super) fn dispatch_datagram(
    data: &[u8],
    conn_index: u8,
    session_manager: &DatagramSessionManager,
) -> Option<Vec<u8>> {
    if data.is_empty() {
        tracing::warn!("received empty datagram, ignoring");
        return None;
    }

    let Some(datagram_type) = DatagramType::from_u8(data[0]) else {
        tracing::warn!(type_byte = data[0], "unknown datagram type, ignoring");
        return None;
    };

    match datagram_type {
        DatagramType::UdpSessionRegistration => handle_registration(data, conn_index, session_manager),
        DatagramType::UdpSessionPayload => {
            handle_payload(data, session_manager);
            None
        }
        DatagramType::Icmp => {
            // ICMP proxy is deferred (HIS-069/070).
            tracing::debug!("received ICMP datagram, ignoring (deferred)");
            None
        }
        DatagramType::UdpSessionRegistrationResponse => {
            // cloudflared should never receive registration responses.
            tracing::warn!("unexpected registration response datagram from edge");
            None
        }
    }
}

/// Handle a session registration datagram.
///
/// Registers the session and returns a response datagram.
/// Matches `datagramConn.handleSessionRegistrationDatagram` in
/// `quic/v3/muxer.go`.
fn handle_registration(
    data: &[u8],
    conn_index: u8,
    session_manager: &DatagramSessionManager,
) -> Option<Vec<u8>> {
    let Some(reg) = UdpSessionRegistrationDatagram::unmarshal(data) else {
        tracing::warn!("failed to unmarshal session registration datagram");
        return None;
    };

    let request_id = reg.request_id;
    let dest = reg.dest;

    match session_manager.register_session(&reg, conn_index) {
        Ok(_) => {
            tracing::debug!(
                request_id = %request_id,
                dest = %dest,
                "session registered"
            );

            let response = UdpSessionRegistrationResponseDatagram {
                request_id,
                response_type: SessionRegistrationResp::Ok,
                error_msg: String::new(),
            };

            Some(response.marshal())
        }
        Err(SessionError::AlreadyRegistered) => {
            // Re-send Ok — the original response may have been lost.
            tracing::debug!(
                request_id = %request_id,
                "session already registered, retrying response"
            );

            let response = UdpSessionRegistrationResponseDatagram {
                request_id,
                response_type: SessionRegistrationResp::Ok,
                error_msg: String::new(),
            };

            Some(response.marshal())
        }
        Err(SessionError::BoundToOtherConn) => {
            tracing::debug!(
                request_id = %request_id,
                "session bound to another connection"
            );

            // Go migrates the session to the new connection. We send Ok
            // for now; full migration is deferred to runtime integration.
            let response = UdpSessionRegistrationResponseDatagram {
                request_id,
                response_type: SessionRegistrationResp::Ok,
                error_msg: String::new(),
            };

            Some(response.marshal())
        }
        Err(SessionError::RegistrationRateLimited) => {
            tracing::warn!(
                request_id = %request_id,
                "session registration rate limited"
            );

            let response = UdpSessionRegistrationResponseDatagram {
                request_id,
                response_type: SessionRegistrationResp::TooManyActiveFlows,
                error_msg: String::new(),
            };

            Some(response.marshal())
        }
        Err(error) => {
            tracing::warn!(
                request_id = %request_id,
                error = %error,
                "session registration failed"
            );

            let response = UdpSessionRegistrationResponseDatagram {
                request_id,
                response_type: SessionRegistrationResp::UnableToBindSocket,
                error_msg: error.to_string(),
            };

            Some(response.marshal())
        }
    }
}

/// Handle a session payload datagram.
///
/// Looks up the session and, in a future runtime integration,
/// forwards the payload to the origin UDP socket. For now, logs
/// and drops the payload. Matches `datagramConn.handleSessionPayloadDatagram`
/// in `quic/v3/muxer.go`.
fn handle_payload(data: &[u8], session_manager: &DatagramSessionManager) {
    let Some(payload) = UdpSessionPayloadDatagram::unmarshal(data) else {
        tracing::warn!("failed to unmarshal session payload datagram");
        return;
    };

    match session_manager.get_session(&payload.request_id) {
        Ok(_conn_id) => {
            // Real UDP forwarding is deferred to runtime integration. For
            // now, acknowledge the lookup succeeded but drop the payload.
            tracing::trace!(
                request_id = %payload.request_id,
                payload_len = payload.payload.len(),
                "session payload received (forwarding deferred)"
            );
        }
        Err(_) => {
            tracing::debug!(
                request_id = %payload.request_id,
                "payload for unknown session, dropping"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};
    use std::time::Duration;

    use cfdrs_cdc::datagram::{
        RequestId, SessionError, SessionManager, SessionRegistrationResp, UdpSessionRegistrationDatagram,
        UdpSessionRegistrationResponseDatagram,
    };

    use super::{DatagramSessionManager, dispatch_datagram};

    fn test_registration(request_id: RequestId, dest: SocketAddr) -> UdpSessionRegistrationDatagram {
        UdpSessionRegistrationDatagram {
            request_id,
            dest,
            traced: false,
            idle_duration_hint: Duration::from_secs(30),
            payload: Vec::new(),
        }
    }

    #[test]
    fn register_and_lookup() {
        let mgr = DatagramSessionManager::new();
        let rid = RequestId::new(1, 2);
        let dest = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8080);
        let reg = test_registration(rid, dest);

        let result = mgr.register_session(&reg, 0);
        assert_eq!(result, Ok(rid));
        assert_eq!(mgr.get_session(&rid), Ok(0));
    }

    #[test]
    fn duplicate_registration_same_conn() {
        let mgr = DatagramSessionManager::new();
        let rid = RequestId::new(10, 20);
        let dest = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 9090);
        let reg = test_registration(rid, dest);

        let _ = mgr.register_session(&reg, 0);
        assert_eq!(
            mgr.register_session(&reg, 0),
            Err(SessionError::AlreadyRegistered)
        );
    }

    #[test]
    fn duplicate_registration_different_conn() {
        let mgr = DatagramSessionManager::new();
        let rid = RequestId::new(30, 40);
        let dest = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 7070);
        let reg = test_registration(rid, dest);

        let _ = mgr.register_session(&reg, 0);
        assert_eq!(mgr.register_session(&reg, 1), Err(SessionError::BoundToOtherConn));
    }

    #[test]
    fn unregister() {
        let mgr = DatagramSessionManager::new();
        let rid = RequestId::new(50, 60);
        let dest = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 6060);
        let reg = test_registration(rid, dest);

        let _ = mgr.register_session(&reg, 0);
        assert_eq!(mgr.session_count(), 1);

        mgr.unregister_session(&rid);
        assert_eq!(mgr.session_count(), 0);
        assert_eq!(mgr.get_session(&rid), Err(SessionError::NotFound));
    }

    #[test]
    fn lookup_missing_session() {
        let mgr = DatagramSessionManager::new();
        let rid = RequestId::new(99, 99);
        assert_eq!(mgr.get_session(&rid), Err(SessionError::NotFound));
    }

    #[test]
    fn dispatch_registration_returns_ok_response() {
        let mgr = DatagramSessionManager::new();
        let rid = RequestId::new(100, 200);
        let dest = SocketAddr::new(Ipv4Addr::new(10, 0, 0, 1).into(), 53);
        let reg = test_registration(rid, dest);
        let wire = reg.marshal();

        let response_bytes =
            dispatch_datagram(&wire, 0, &mgr).expect("registration should produce a response");
        let response = UdpSessionRegistrationResponseDatagram::unmarshal(&response_bytes)
            .expect("response should unmarshal");

        assert_eq!(response.request_id, rid);
        assert_eq!(response.response_type, SessionRegistrationResp::Ok);
        assert_eq!(mgr.session_count(), 1);
    }

    #[test]
    fn dispatch_duplicate_registration_still_ok() {
        let mgr = DatagramSessionManager::new();
        let rid = RequestId::new(101, 201);
        let dest = SocketAddr::new(Ipv4Addr::new(10, 0, 0, 2).into(), 53);
        let reg = test_registration(rid, dest);
        let wire = reg.marshal();

        let _ = dispatch_datagram(&wire, 0, &mgr);

        let response_bytes =
            dispatch_datagram(&wire, 0, &mgr).expect("duplicate registration should still produce Ok");
        let response = UdpSessionRegistrationResponseDatagram::unmarshal(&response_bytes)
            .expect("response should unmarshal");

        assert_eq!(response.response_type, SessionRegistrationResp::Ok);
    }

    #[test]
    fn dispatch_payload_for_known_session() {
        let mgr = DatagramSessionManager::new();
        let rid = RequestId::new(102, 202);
        let dest = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 1234);
        let reg = test_registration(rid, dest);
        let _ = mgr.register_session(&reg, 0);

        let payload = cfdrs_cdc::datagram::UdpSessionPayloadDatagram {
            request_id: rid,
            payload: b"hello".to_vec(),
        };
        let wire = payload.marshal();

        // Payload dispatch returns None (no response datagram).
        assert!(dispatch_datagram(&wire, 0, &mgr).is_none());
    }

    #[test]
    fn dispatch_payload_for_unknown_session() {
        let mgr = DatagramSessionManager::new();
        let rid = RequestId::new(103, 203);
        let payload = cfdrs_cdc::datagram::UdpSessionPayloadDatagram {
            request_id: rid,
            payload: b"orphan".to_vec(),
        };
        let wire = payload.marshal();

        assert!(dispatch_datagram(&wire, 0, &mgr).is_none());
    }

    #[test]
    fn dispatch_icmp_returns_none() {
        let mgr = DatagramSessionManager::new();
        let icmp = cfdrs_cdc::datagram::IcmpDatagram {
            payload: vec![0x08, 0x00, 0x00, 0x00],
        };
        let wire = icmp.marshal();

        assert!(dispatch_datagram(&wire, 0, &mgr).is_none());
    }

    #[test]
    fn dispatch_unknown_type_returns_none() {
        let mgr = DatagramSessionManager::new();
        let wire = vec![0xfe, 0x01, 0x02];

        assert!(dispatch_datagram(&wire, 0, &mgr).is_none());
    }

    #[test]
    fn dispatch_empty_datagram_returns_none() {
        let mgr = DatagramSessionManager::new();
        assert!(dispatch_datagram(&[], 0, &mgr).is_none());
    }
}
