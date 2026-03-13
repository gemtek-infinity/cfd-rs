#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransportLifecycleStage {
    IdentityLoaded,
    ResolvingEdge,
    Dialing,
    Handshaking,
    Established,
    ControlStreamOpened,
    ServingStreams,
    Teardown,
}

impl TransportLifecycleStage {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::IdentityLoaded => "identity-loaded",
            Self::ResolvingEdge => "resolving-edge",
            Self::Dialing => "dialing",
            Self::Handshaking => "handshaking",
            Self::Established => "established",
            Self::ControlStreamOpened => "control-stream-opened",
            Self::ServingStreams => "serving-streams",
            Self::Teardown => "teardown",
        }
    }

    pub(crate) fn is_connected(self) -> bool {
        matches!(
            self,
            Self::Established | Self::ControlStreamOpened | Self::ServingStreams | Self::Teardown
        )
    }
}

impl std::fmt::Display for TransportLifecycleStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

mod quic;

pub(crate) use quic::QuicTunnelServiceFactory;
