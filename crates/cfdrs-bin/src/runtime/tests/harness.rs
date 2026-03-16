pub(super) use crate::transport::{TestBehavior, TransportServiceSource};

pub(super) fn test_source(behaviors: impl IntoIterator<Item = TestBehavior>) -> TransportServiceSource {
    TransportServiceSource::test(behaviors)
}
