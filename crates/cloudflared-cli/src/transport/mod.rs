#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransportLifecycleStage {
    IdentityLoaded,
    ResolvingEdge,
    Dialing,
    Handshaking,
    Established,
    ControlStreamOpened,
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
            Self::Teardown => "teardown",
        }
    }

    pub(crate) fn is_connected(self) -> bool {
        matches!(
            self,
            Self::Established | Self::ControlStreamOpened | Self::Teardown
        )
    }
}

mod quic;

pub(crate) use quic::QuicTunnelServiceFactory;
