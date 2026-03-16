mod bridges;
mod drain;

use crate::proxy::PingoraProxySeam;

use super::ApplicationRuntime;

impl ApplicationRuntime {
    pub(super) fn spawn_proxy_seam(&mut self) {
        let ingress = self.config.normalized().ingress.clone();
        let seam = PingoraProxySeam::new(ingress);
        let protocol_rx = self.protocol_receiver.take();
        let stream_response_tx = self.stream_response_tx.take();
        self.status.push_summary(format!(
            "proxy-seam: origin-proxy admitted, ingress-rules={}",
            seam.ingress_count()
        ));
        seam.spawn(
            self.command_tx.clone(),
            protocol_rx,
            stream_response_tx,
            self.shutdown.child_token(),
            &mut self.child_tasks,
        );
    }

    pub(super) fn spawn_primary_service(&mut self, attempt: u32) {
        let service = self.service_source.create_service(self.config.clone(), attempt);
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
