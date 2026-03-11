mod service;

use std::collections::VecDeque;
use std::sync::Mutex;

use self::service::TestService;

use super::super::{RuntimeConfig, RuntimeService, RuntimeServiceFactory};

#[derive(Clone)]
pub(super) enum TestBehavior {
    WaitForShutdown,
    RetryableFailure,
    FatalFailure,
}

#[derive(Clone)]
pub(super) struct TestFactory {
    behaviors: std::sync::Arc<Mutex<VecDeque<TestBehavior>>>,
}

impl TestFactory {
    pub(super) fn new(behaviors: impl IntoIterator<Item = TestBehavior>) -> Self {
        Self {
            behaviors: std::sync::Arc::new(Mutex::new(behaviors.into_iter().collect())),
        }
    }
}

impl RuntimeServiceFactory for TestFactory {
    fn create_primary(
        &self,
        _config: std::sync::Arc<RuntimeConfig>,
        _attempt: u32,
    ) -> Box<dyn RuntimeService> {
        let behavior = self
            .behaviors
            .lock()
            .expect("test factory lock should not be poisoned")
            .pop_front()
            .unwrap_or(TestBehavior::WaitForShutdown);

        Box::new(TestService { behavior })
    }
}
