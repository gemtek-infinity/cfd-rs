use tokio::sync::mpsc;

use super::edge::EDGE_DEFAULT_REGION;
use super::identity::TransportIdentity;
use super::session::QuicSessionState;
use super::{QuicEdgeTarget, QuicTunnelService, TransportLifecycleStage};
use crate::runtime::{RuntimeCommand, RuntimeService};

impl QuicTunnelService {
    pub(super) async fn report_identity_status(
        &self,
        command_tx: &mpsc::Sender<RuntimeCommand>,
        service_name: &'static str,
        identity: &TransportIdentity,
    ) {
        let endpoint_hint = identity.endpoint_hint.as_deref().unwrap_or(EDGE_DEFAULT_REGION);

        super::send_transport_stage(
            command_tx,
            service_name,
            TransportLifecycleStage::IdentityLoaded,
            format!("identity-source={}", identity.identity_source),
        )
        .await;

        super::send_status(
            command_tx,
            service_name,
            format!("transport-phase: quiche attempt={}", self.attempt + 1),
        )
        .await;
        super::send_status(
            command_tx,
            service_name,
            format!("transport-tunnel-id: {}", identity.tunnel_id),
        )
        .await;
        super::send_status(
            command_tx,
            service_name,
            format!("transport-identity-source: {}", identity.identity_source),
        )
        .await;
        super::send_status(
            command_tx,
            service_name,
            format!("quic-0rtt-policy: {}", identity.resumption.policy_label()),
        )
        .await;
        super::send_status(
            command_tx,
            service_name,
            "quic-pqc-compatibility: preserved through quiche + boringssl lane".to_owned(),
        )
        .await;
        super::send_transport_stage(
            command_tx,
            service_name,
            TransportLifecycleStage::ResolvingEdge,
            format!("endpoint-hint={endpoint_hint}"),
        )
        .await;
    }

    pub(super) async fn report_established(
        &self,
        session: &QuicSessionState,
        identity: &TransportIdentity,
        target: &QuicEdgeTarget,
        command_tx: &mpsc::Sender<RuntimeCommand>,
    ) {
        super::send_status(
            command_tx,
            self.name(),
            format!(
                "transport-session-state: established peer={} early-data={} resumed-shape={}",
                target.connect_addr,
                session.connection.is_in_early_data(),
                identity.resumption.shape_label(),
            ),
        )
        .await;
        super::send_transport_stage(
            command_tx,
            self.name(),
            TransportLifecycleStage::Established,
            format!(
                "peer={} resumed-shape={}",
                target.connect_addr,
                identity.resumption.shape_label()
            ),
        )
        .await;
        let _ = command_tx
            .send(RuntimeCommand::ServiceReady { service: self.name() })
            .await;
    }
}
