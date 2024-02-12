use super::{msg::DescImport, CapTpSession};
use futures::{
    channel::oneshot::{Receiver, Sender},
    AsyncWrite, Future,
};
use std::{any::Any, sync::Arc};
use syrup::{raw_syrup, Serialize, Symbol};

mod promise;
pub use promise::*;

mod bootstrap;
pub use bootstrap::*;

/// Data received in an op:deliver or op:deliver-only
pub type Delivery = (Vec<syrup::Item>, Option<DescImport>);
/// Sending half of an object pipe.
pub type DeliverySender = futures::channel::mpsc::UnboundedSender<Delivery>;
/// Receiving half of an object pipe.
pub type DeliveryReceiver = futures::channel::mpsc::UnboundedReceiver<Delivery>;

// pub trait Object {
//     fn deliver(&self, args: Vec<syrup::Item>);
// }

// pub trait Object {
//     fn deliver_only(&self, args: Vec<syrup::Item>);
//     fn deliver(&self, args: Vec<syrup::Item>);
// }

// pub trait RemoteObject {
//     async fn deliver_only<Arg: Serialize>(self: Arc<Self>, ident: &str, args: Vec<Arg>);
//     async fn deliver<Arg: Serialize>(self: Arc<Self>, ident: &str, args: Vec<Arg>);
// }

/// An object to which the answer to a Promise may be sent.
pub struct RemoteResolver<Socket> {
    base: RemoteObject<Socket>,
}

impl<Socket> RemoteResolver<Socket> {
    pub async fn fulfill<'arg, Arg: Serialize + 'arg>(
        &self,
        args: impl IntoIterator<Item = &'arg Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        self.base
            .call("fulfill", args, answer_pos, resolve_me_desc)
            .await
    }

    pub async fn break_promise(&self, error: impl Serialize) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        self.base.call_only("break", &[error]).await
    }
}

pub type PromiseResult = Result<Vec<syrup::Item>, syrup::Item>;
pub type PromiseSender = futures::channel::oneshot::Sender<PromiseResult>;
pub type PromiseReceiver = futures::channel::oneshot::Receiver<PromiseResult>;

pub struct Resolver {
    sender: PromiseSender,
}

/// An object representing the response to an [OpDeliver].
pub struct Answer {
    receiver: PromiseReceiver,
}

impl From<PromiseReceiver> for Answer {
    fn from(receiver: PromiseReceiver) -> Self {
        Answer { receiver }
    }
}

impl Answer {
    pub async fn resolve(self) -> Result<PromiseResult, futures::channel::oneshot::Canceled> {
        self.receiver.await
    }
}

#[derive(Clone)]
pub struct RemoteObject<Socket> {
    session: CapTpSession<Socket>,
    position: u64,
}

impl<Socket> RemoteObject<Socket> {
    pub(crate) fn new(session: CapTpSession<Socket>, position: u64) -> Self {
        Self { session, position }
    }

    pub async fn deliver_only<Arg: Serialize>(
        &self,
        args: Vec<Arg>,
    ) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        self.session.deliver_only(self.position, args).await
    }

    pub async fn deliver<Arg: Serialize>(
        &self,
        args: Vec<Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        self.session
            .deliver(self.position, args, answer_pos, resolve_me_desc)
            .await
    }

    pub async fn call_only<'arg, Arg: Serialize + 'arg>(
        &self,
        ident: impl AsRef<str>,
        args: impl IntoIterator<Item = &'arg Arg>,
    ) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        self.deliver_only(syrup::raw_syrup_unwrap![&Symbol(ident.as_ref()); args])
            .await
    }

    pub async fn call<'arg, Arg: Serialize + 'arg>(
        &self,
        ident: impl AsRef<str>,
        args: impl IntoIterator<Item = &'arg Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        self.deliver(
            syrup::raw_syrup_unwrap![&Symbol(ident.as_ref()); args],
            answer_pos,
            resolve_me_desc,
        )
        .await
    }
}

pub struct LocalObject<Socket> {
    session: CapTpSession<Socket>,
    position: u64,
    receiver: DeliveryReceiver,
}

impl<Socket> LocalObject<Socket> {
    pub(crate) fn new(
        session: CapTpSession<Socket>,
        position: u64,
        receiver: DeliveryReceiver,
    ) -> Self {
        Self {
            session,
            position,
            receiver,
        }
    }

    pub fn try_next(&mut self) -> Result<Option<Delivery>, futures::channel::mpsc::TryRecvError> {
        self.receiver.try_next()
    }
}

impl<Socket> futures::stream::Stream for LocalObject<Socket> {
    type Item = Delivery;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use futures::stream::Stream;
        use std::task::{self, Poll};

        match Stream::poll_next(std::pin::Pin::new(&mut self.receiver), cx) {
            Poll::Ready(r) => Poll::Ready(r),
            Poll::Pending => todo!(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        futures::stream::Stream::size_hint(&self.receiver)
    }
}

impl<Socket> futures::stream::FusedStream for LocalObject<Socket> {
    fn is_terminated(&self) -> bool {
        futures::stream::FusedStream::is_terminated(&self.receiver)
    }
}
