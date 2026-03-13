mod bridges;
mod drain;

use crate::proxy::PingoraProxySeam;

use super::{ApplicationRuntime, RuntimeServiceFactory};

impl<F> ApplicationRuntime<F>
where
    F: RuntimeServiceFactory,
{
    pub(super) fn spawn_proxy_seam(&mut self) {
        let ingress = self.config.normalized().ingress.clone();
        let seam = PingoraProxySeam::new(ingress);
        let protocol_rx = self.protocol_receiver.take();
        self.status.push_summary(format!(
            "proxy-seam: origin-proxy admitted, ingress-rules={}",
            seam.ingress_count()
        ));
        seam.spawn(
            self.command_tx.clone(),
            protocol_rx,
            self.shutdown.child_token(),
            &mut self.child_tasks,
        );
    }

    pub(super) fn spawn_primary_service(&mut self, attempt: u32) {
        let service = self.factory.create_primary(self.config.clone(), attempt);
        self.status.push_summary(format!(
            "primary-service-attempt: {} service={}",
            attempt + 1,
            service.name()
        ));
        service.spawn(
            self.command_tx.clone(),
            self.shutdown.child_token(),
            &mut self.child_tasks,
        );
    }
}
