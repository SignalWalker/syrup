use crate::captp::object::{DeliveryReceiver, DeliverySender};

use super::Delivery;

#[derive(Debug, Clone)]
pub struct ExportToken {
    position: u64,
    sender: DeliverySender,
}

impl ExportToken {
    fn new(position: u64) -> (Self, DeliveryReceiver) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        (Self { position, sender }, receiver)
    }
}

impl futures::Sink<Delivery> for ExportToken {
    type Error = <DeliverySender as futures::Sink<Delivery>>::Error;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.sender.poll_ready(cx)
    }

    fn start_send(mut self: std::pin::Pin<&mut Self>, item: Delivery) -> Result<(), Self::Error> {
        self.sender.start_send(item)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::pin::pin!(&mut self.sender).poll_flush(cx)
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::pin::pin!(&mut self.sender).poll_close(cx)
    }
}
