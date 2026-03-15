//! Cap'n Proto RPC dispatch bridge.
//!
//! Connects the generated capnp-rpc interface types to Rust domain types
//! in `registration.rs`. Provides:
//!
//! - **Server dispatch** (`CloudflaredServerDispatch`): implements the
//!   generated `cloudflared_server::Server` by delegating to
//!   `SessionManagerHandler` and `ConfigurationManagerHandler` traits. This
//!   handles edge→cloudflared RPCs on accepted QUIC streams.
//!
//! - **Client wrapper** (`RegistrationClient`): wraps the generated
//!   `registration_server::Client` with domain-type conversions. This handles
//!   cloudflared→edge RPCs on the control stream (stream 0).
//!
//! Transport setup is the caller's responsibility. This module handles
//! only the domain↔capnp translation layer.
//!
//! Go baseline:
//! - `baseline-2026.2.0/tunnelrpc/pogs/registration_server.go`
//! - `baseline-2026.2.0/tunnelrpc/registration_client.go`

use crate::registration::{
    ConnectionResponse, RegisterConnectionRequest, RegisterUdpSessionRequest, RegisterUdpSessionResponse,
    UnregisterUdpSessionRequest, UpdateConfigurationRequest, UpdateConfigurationResponse,
    UpdateLocalConfigurationRequest,
};
use crate::registration_codec::read_capnp_text;
use crate::tunnelrpc_capnp;

// ---------------------------------------------------------------------------
// Handler traits — application-level callbacks for edge→cloudflared RPCs
// ---------------------------------------------------------------------------

/// Handles `SessionManager` RPCs dispatched from the edge.
///
/// The edge calls these methods on accepted RPC streams (identified by
/// the RPC signature preamble). Matches Go `pogs.SessionManager`.
pub trait SessionManagerHandler: 'static {
    /// Handle `registerUdpSession` — edge requests a new UDP session.
    fn register_udp_session(&self, request: RegisterUdpSessionRequest) -> RegisterUdpSessionResponse;

    /// Handle `unregisterUdpSession` — edge tears down a UDP session.
    fn unregister_udp_session(&self, request: UnregisterUdpSessionRequest);
}

/// Handles `ConfigurationManager` RPCs dispatched from the edge.
///
/// Matches Go `pogs.ConfigurationManager`.
pub trait ConfigurationManagerHandler: 'static {
    /// Handle `updateConfiguration` — edge pushes a remote config update.
    fn update_configuration(&self, request: UpdateConfigurationRequest) -> UpdateConfigurationResponse;
}

// ---------------------------------------------------------------------------
// Server dispatch — capnp → domain → handler → marshal response
// ---------------------------------------------------------------------------

/// Serves `CloudflaredServer` = `SessionManager` + `ConfigurationManager`.
///
/// Translates incoming capnp-rpc calls into domain types, invokes the
/// handler, and marshals the response back through Cap'n Proto.
///
/// Go baseline: `CloudflaredServer_PogsImpl` which embeds both handler impls.
pub struct CloudflaredServerDispatch<S, C> {
    session_handler: S,
    config_handler: C,
}

impl<S, C> CloudflaredServerDispatch<S, C>
where
    S: SessionManagerHandler,
    C: ConfigurationManagerHandler,
{
    pub fn new(session_handler: S, config_handler: C) -> Self {
        Self {
            session_handler,
            config_handler,
        }
    }
}

impl<S, C> tunnelrpc_capnp::session_manager::Server for CloudflaredServerDispatch<S, C>
where
    S: SessionManagerHandler,
    C: ConfigurationManagerHandler,
{
    async fn register_udp_session(
        self: ::capnp::capability::Rc<Self>,
        params: tunnelrpc_capnp::session_manager::RegisterUdpSessionParams,
        mut results: tunnelrpc_capnp::session_manager::RegisterUdpSessionResults,
    ) -> Result<(), ::capnp::Error> {
        let reader = params.get()?;

        let session_id = reader.get_session_id()?;
        let dst_ip = reader.get_dst_ip()?;
        let dst_port = reader.get_dst_port();
        let close_after_idle_hint = reader.get_close_after_idle_hint();
        let trace_context = read_capnp_text(reader.get_trace_context()?)?;

        let request = RegisterUdpSessionRequest::from_rpc_params(
            session_id,
            dst_ip,
            dst_port,
            close_after_idle_hint,
            &trace_context,
        )
        .ok_or_else(|| ::capnp::Error::failed("invalid session ID length".into()))?;

        let response = self.session_handler.register_udp_session(request);
        response.marshal_capnp(results.get().init_result());

        Ok(())
    }

    async fn unregister_udp_session(
        self: ::capnp::capability::Rc<Self>,
        params: tunnelrpc_capnp::session_manager::UnregisterUdpSessionParams,
        _results: tunnelrpc_capnp::session_manager::UnregisterUdpSessionResults,
    ) -> Result<(), ::capnp::Error> {
        let reader = params.get()?;

        let session_id = reader.get_session_id()?;
        let message = read_capnp_text(reader.get_message()?)?;

        let uuid = uuid::Uuid::from_slice(session_id)
            .map_err(|e| ::capnp::Error::failed(format!("invalid session ID: {e}")))?;

        let request = UnregisterUdpSessionRequest {
            session_id: uuid,
            message,
        };

        self.session_handler.unregister_udp_session(request);

        Ok(())
    }
}

impl<S, C> tunnelrpc_capnp::configuration_manager::Server for CloudflaredServerDispatch<S, C>
where
    S: SessionManagerHandler,
    C: ConfigurationManagerHandler,
{
    async fn update_configuration(
        self: ::capnp::capability::Rc<Self>,
        params: tunnelrpc_capnp::configuration_manager::UpdateConfigurationParams,
        mut results: tunnelrpc_capnp::configuration_manager::UpdateConfigurationResults,
    ) -> Result<(), ::capnp::Error> {
        let reader = params.get()?;

        let version = reader.get_version();
        let config = reader.get_config()?;

        let request = UpdateConfigurationRequest::from_rpc_params(version, config);
        let response = self.config_handler.update_configuration(request);

        response.marshal_capnp(results.get().init_result());

        Ok(())
    }
}

// The `cloudflared_server::Server` trait is a marker that extends
// `session_manager::Server + configuration_manager::Server`. It has no
// methods of its own — dispatch routing is handled by the generated
// `ServerDispatch::dispatch_call` which routes by interface ID.
impl<S, C> tunnelrpc_capnp::cloudflared_server::Server for CloudflaredServerDispatch<S, C>
where
    S: SessionManagerHandler,
    C: ConfigurationManagerHandler,
{
}

/// Create a `cloudflared_server::Client` from handler implementations.
///
/// The returned client can be provided as the main interface for an
/// `RpcSystem` serving edge→cloudflared RPCs on accepted QUIC streams.
pub fn new_cloudflared_server<S, C>(
    session_handler: S,
    config_handler: C,
) -> tunnelrpc_capnp::cloudflared_server::Client
where
    S: SessionManagerHandler,
    C: ConfigurationManagerHandler,
{
    capnp_rpc::new_client(CloudflaredServerDispatch::new(session_handler, config_handler))
}

// ---------------------------------------------------------------------------
// Client wrapper — domain → capnp → send → unmarshal
// ---------------------------------------------------------------------------

/// Client for calling `RegistrationServer` RPCs on the edge.
///
/// Wraps the generated capnp-rpc client to convert between domain types
/// and Cap'n Proto parameters/results. Used on the control stream
/// (QUIC stream 0) where cloudflared is the RPC client.
///
/// Go baseline: `tunnelrpc/registration_client.go`
pub struct RegistrationClient {
    client: tunnelrpc_capnp::registration_server::Client,
}

impl RegistrationClient {
    /// Wrap a generated capnp-rpc client obtained from the control stream
    /// RPC bootstrap.
    pub fn new(client: tunnelrpc_capnp::registration_server::Client) -> Self {
        Self { client }
    }

    /// Call `registerConnection` on the edge's `RegistrationServer`.
    ///
    /// CDC-001 through CDC-006: sends auth, tunnel ID, connection index,
    /// and connection options; receives connection details or a
    /// retry-aware error.
    pub async fn register_connection(
        &self,
        request: &RegisterConnectionRequest,
    ) -> Result<ConnectionResponse, ::capnp::Error> {
        let mut rpc_request = self.client.register_connection_request();

        {
            let mut params = rpc_request.get();
            request.auth.marshal_capnp(params.reborrow().init_auth());
            params.reborrow().set_tunnel_id(request.tunnel_id.as_bytes());
            params.reborrow().set_conn_index(request.conn_index);
            request.options.marshal_capnp(params.init_options());
        }

        let response = rpc_request.send().promise.await?;
        let result_reader = response.get()?.get_result()?;
        ConnectionResponse::unmarshal_capnp(result_reader)
    }

    /// Call `unregisterConnection` on the edge's `RegistrationServer`.
    ///
    /// CDC-007: graceful disconnect over the control stream. The schema
    /// defines this as `() -> ()` — no parameters, void return.
    pub async fn unregister_connection(&self) -> Result<(), ::capnp::Error> {
        let rpc_request = self.client.unregister_connection_request();
        rpc_request.send().promise.await?;
        Ok(())
    }

    /// Call `updateLocalConfiguration` on the edge's `RegistrationServer`.
    ///
    /// CDC-008: pushes tunnel config to edge. Only sent on connIndex==0
    /// when not remotely managed.
    pub async fn update_local_configuration(
        &self,
        request: &UpdateLocalConfigurationRequest,
    ) -> Result<(), ::capnp::Error> {
        let mut rpc_request = self.client.update_local_configuration_request();
        rpc_request.get().set_config(&request.config);
        rpc_request.send().promise.await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use uuid::Uuid;

    // -- Mock handlers ----------------------------------------------------

    struct MockSessionManager {
        register_called: Cell<bool>,
        unregister_called: Cell<bool>,
    }

    impl MockSessionManager {
        fn new() -> Self {
            Self {
                register_called: Cell::new(false),
                unregister_called: Cell::new(false),
            }
        }
    }

    impl SessionManagerHandler for MockSessionManager {
        fn register_udp_session(&self, _request: RegisterUdpSessionRequest) -> RegisterUdpSessionResponse {
            self.register_called.set(true);
            RegisterUdpSessionResponse {
                err: String::new(),
                spans: vec![1, 2, 3],
            }
        }

        fn unregister_udp_session(&self, _request: UnregisterUdpSessionRequest) {
            self.unregister_called.set(true);
        }
    }

    struct MockConfigManager;

    impl ConfigurationManagerHandler for MockConfigManager {
        fn update_configuration(&self, request: UpdateConfigurationRequest) -> UpdateConfigurationResponse {
            UpdateConfigurationResponse {
                latest_applied_version: request.version,
                err: String::new(),
            }
        }
    }

    // -- Mock RegistrationServer (edge side, for testing RegistrationClient) --

    struct MockRegistrationServer;

    impl tunnelrpc_capnp::registration_server::Server for MockRegistrationServer {
        async fn register_connection(
            self: ::capnp::capability::Rc<Self>,
            params: tunnelrpc_capnp::registration_server::RegisterConnectionParams,
            mut results: tunnelrpc_capnp::registration_server::RegisterConnectionResults,
        ) -> Result<(), ::capnp::Error> {
            let reader = params.get()?;
            let tunnel_id = reader.get_tunnel_id()?;
            let conn_index = reader.get_conn_index();

            // Echo back connection details derived from the request.
            let uuid = Uuid::from_slice(tunnel_id)
                .map_err(|e| ::capnp::Error::failed(format!("bad tunnel_id: {e}")))?;

            let response = ConnectionResponse::success(crate::registration::ConnectionDetails {
                uuid,
                location: format!("SFO-{conn_index}"),
                is_remotely_managed: false,
            });

            response.marshal_capnp(results.get().init_result());
            Ok(())
        }

        async fn unregister_connection(
            self: ::capnp::capability::Rc<Self>,
            _params: tunnelrpc_capnp::registration_server::UnregisterConnectionParams,
            _results: tunnelrpc_capnp::registration_server::UnregisterConnectionResults,
        ) -> Result<(), ::capnp::Error> {
            Ok(())
        }

        async fn update_local_configuration(
            self: ::capnp::capability::Rc<Self>,
            _params: tunnelrpc_capnp::registration_server::UpdateLocalConfigurationParams,
            _results: tunnelrpc_capnp::registration_server::UpdateLocalConfigurationResults,
        ) -> Result<(), ::capnp::Error> {
            Ok(())
        }
    }

    // -- Helper: cast cloudflared_server::Client to sub-interface clients --

    fn session_manager_client(
        cs: &tunnelrpc_capnp::cloudflared_server::Client,
    ) -> tunnelrpc_capnp::session_manager::Client {
        tunnelrpc_capnp::session_manager::Client {
            client: cs.client.clone(),
        }
    }

    fn configuration_manager_client(
        cs: &tunnelrpc_capnp::cloudflared_server::Client,
    ) -> tunnelrpc_capnp::configuration_manager::Client {
        tunnelrpc_capnp::configuration_manager::Client {
            client: cs.client.clone(),
        }
    }

    // -- Server dispatch tests (edge→cloudflared direction) ---------------

    #[tokio::test]
    async fn server_dispatches_register_udp_session() {
        let cs_client = new_cloudflared_server(MockSessionManager::new(), MockConfigManager);
        let sm_client = session_manager_client(&cs_client);

        let session_id = Uuid::new_v4();
        let mut request = sm_client.register_udp_session_request();

        {
            let mut params = request.get();
            params.set_session_id(session_id.as_bytes());
            params.set_dst_ip(&[127, 0, 0, 1]);
            params.set_dst_port(8080);
            params.set_close_after_idle_hint(5_000_000_000);
            params.set_trace_context("test-trace");
        }

        let response = request.send().promise.await.expect("rpc should succeed");
        let result = response
            .get()
            .expect("result should be readable")
            .get_result()
            .expect("result field");

        let domain_resp =
            RegisterUdpSessionResponse::unmarshal_capnp(result).expect("unmarshal should succeed");

        assert!(domain_resp.is_ok(), "empty err means success");
        assert_eq!(domain_resp.spans, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn server_dispatches_unregister_udp_session() {
        let cs_client = new_cloudflared_server(MockSessionManager::new(), MockConfigManager);
        let sm_client = session_manager_client(&cs_client);

        let session_id = Uuid::new_v4();
        let mut request = sm_client.unregister_udp_session_request();

        {
            let mut params = request.get();
            params.set_session_id(session_id.as_bytes());
            params.set_message("test teardown");
        }

        request
            .send()
            .promise
            .await
            .expect("unregister rpc should succeed");
    }

    #[tokio::test]
    async fn server_dispatches_update_configuration() {
        let cs_client = new_cloudflared_server(MockSessionManager::new(), MockConfigManager);
        let cm_client = configuration_manager_client(&cs_client);

        let mut request = cm_client.update_configuration_request();

        {
            let mut params = request.get();
            params.set_version(42);
            params.set_config(b"config-payload");
        }

        let response = request.send().promise.await.expect("rpc should succeed");
        let result = response
            .get()
            .expect("result should be readable")
            .get_result()
            .expect("result field");

        let domain_resp =
            UpdateConfigurationResponse::unmarshal_capnp(result).expect("unmarshal should succeed");

        assert!(domain_resp.is_ok());
        assert_eq!(domain_resp.latest_applied_version, 42);
    }

    // -- Client wrapper tests (cloudflared→edge direction) ----------------

    #[tokio::test]
    async fn registration_client_register_connection() {
        let edge: tunnelrpc_capnp::registration_server::Client =
            capnp_rpc::new_client(MockRegistrationServer);
        let client = RegistrationClient::new(edge);

        let tunnel_id = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").expect("uuid should parse");

        let request = RegisterConnectionRequest {
            auth: crate::registration::TunnelAuth {
                account_tag: "acct".into(),
                tunnel_secret: vec![0xab; 32],
            },
            tunnel_id,
            conn_index: 2,
            options: crate::registration::ConnectionOptions::for_current_platform(tunnel_id, 0),
        };

        let response = client
            .register_connection(&request)
            .await
            .expect("register should succeed");

        assert!(response.is_ok());

        let details = response.details().expect("should have details");
        assert_eq!(details.uuid, tunnel_id);
        assert_eq!(details.location, "SFO-2");
        assert!(!details.is_remotely_managed);
    }

    #[tokio::test]
    async fn registration_client_unregister_connection() {
        let edge: tunnelrpc_capnp::registration_server::Client =
            capnp_rpc::new_client(MockRegistrationServer);
        let client = RegistrationClient::new(edge);

        client
            .unregister_connection()
            .await
            .expect("unregister should succeed");
    }

    #[tokio::test]
    async fn registration_client_update_local_configuration() {
        let edge: tunnelrpc_capnp::registration_server::Client =
            capnp_rpc::new_client(MockRegistrationServer);
        let client = RegistrationClient::new(edge);

        let request = UpdateLocalConfigurationRequest {
            config: b"tunnel-config-json".to_vec(),
        };

        client
            .update_local_configuration(&request)
            .await
            .expect("update_local_configuration should succeed");
    }
}
