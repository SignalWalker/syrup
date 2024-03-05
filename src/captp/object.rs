use super::{msg::DescImport, CapTpSession, Delivery, GenericResolver, SendError};
use std::{any::Any, sync::Arc};
use syrup::{raw_syrup, Serialize, Symbol};

use crate::async_compat::{oneshot, AsyncWrite};

mod promise;
pub use promise::*;

mod bootstrap;
pub use bootstrap::*;

/// Sending half of an object pipe.
pub type DeliverySender = futures::channel::mpsc::UnboundedSender<Delivery>;
/// Receiving half of an object pipe.
pub type DeliveryReceiver = futures::channel::mpsc::UnboundedReceiver<Delivery>;

pub trait Object {
    fn deliver_only(&self, args: Vec<syrup::Item>);
    fn deliver(&self, args: Vec<syrup::Item>, resolver: GenericResolver);
}

// pub trait Object {
//     fn deliver_only(&self, args: Vec<syrup::Item>);
//     // fn deliver(&self, args: Vec<syrup::Item>);
// }

// pub trait RemoteObject {
//     async fn deliver_only<Arg: Serialize>(self: Arc<Self>, ident: &str, args: Vec<Arg>);
//     async fn deliver<Arg: Serialize>(self: Arc<Self>, ident: &str, args: Vec<Arg>);
// }

/// An object to which the answer to a Promise may be sent.
pub struct RemoteResolver<Reader, Writer> {
    base: RemoteObject<Reader, Writer>,
}

impl<Reader, Writer> RemoteResolver<Reader, Writer> {
    pub async fn fulfill<'arg, Arg: Serialize + 'arg>(
        &self,
        args: impl IntoIterator<Item = &'arg Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.base
            .call("fulfill", args, answer_pos, resolve_me_desc)
            .await
    }

    pub async fn break_promise(&self, error: impl Serialize) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.base.call_only("break", &[error]).await
    }
}

pub type PromiseResult = Result<Vec<syrup::Item>, syrup::Item>;
pub type PromiseSender = oneshot::Sender<PromiseResult>;
pub type PromiseReceiver = oneshot::Receiver<PromiseResult>;

pub struct Resolver {
    sender: std::sync::Mutex<Option<PromiseSender>>,
}

impl Object for Resolver {
    fn deliver_only(&self, args: Vec<syrup::Item>) {
        todo!()
    }

    fn deliver(&self, args: Vec<syrup::Item>, resolver: GenericResolver) {
        let sender = match self.sender.lock().unwrap().take() {
            Some(s) => s,
            None => todo!("broken promise pipe"),
        };
        let mut args = args.into_iter();
        match args.next() {
            Some(syrup::Item::Symbol(id)) => match id.as_str() {
                "fulfill" => sender.send(Ok(args.collect())).unwrap(),
                "break" => match args.next() {
                    Some(reason) => sender.send(Err(reason)).unwrap(),
                    _ => todo!(),
                },
                _ => todo!(),
            },
            _ => todo!(),
        }
    }
}

impl Resolver {
    pub(crate) fn new() -> (Arc<Self>, Answer) {
        let (sender, receiver) = oneshot::channel();
        (
            Arc::new(Self {
                sender: Some(sender).into(),
            }),
            Answer { receiver },
        )
    }
}

/// An object representing the response to an [OpDeliver].
pub struct Answer {
    receiver: PromiseReceiver,
}

impl std::future::Future for Answer {
    type Output = <PromiseReceiver as std::future::Future>::Output;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::future::Future::poll(std::pin::pin!(&mut self.receiver), cx)
    }
}

#[derive(Clone)]
pub struct RemoteObject<Reader, Writer> {
    session: CapTpSession<Reader, Writer>,
    position: u64,
}

impl<Reader, Writer> RemoteObject<Reader, Writer> {
    pub(crate) fn new(session: CapTpSession<Reader, Writer>, position: u64) -> Self {
        Self { session, position }
    }

    pub async fn deliver_only<Arg: Serialize>(&self, args: Vec<Arg>) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.session.deliver_only(self.position, args).await
    }

    pub async fn deliver<Arg: Serialize>(
        &self,
        args: Vec<Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.session
            .deliver(self.position, args, answer_pos, resolve_me_desc)
            .await
    }

    pub async fn deliver_and<Arg: Serialize>(&self, args: Vec<Arg>) -> Result<Answer, SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.session.deliver_and(self.position, args).await
    }

    pub async fn call_only<'arg, Arg: Serialize + 'arg>(
        &self,
        ident: impl AsRef<str>,
        args: impl IntoIterator<Item = &'arg Arg>,
    ) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
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
    ) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.deliver(
            syrup::raw_syrup_unwrap![&Symbol(ident.as_ref()); args],
            answer_pos,
            resolve_me_desc,
        )
        .await
    }

    pub async fn call_and<'arg, Arg: Serialize + 'arg>(
        &self,
        ident: impl AsRef<str>,
        args: impl IntoIterator<Item = &'arg Arg>,
    ) -> Result<Answer, SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.deliver_and(syrup::raw_syrup_unwrap![&Symbol(ident.as_ref()); args])
            .await
    }
}

pub struct ObjectInbox {
    position: u64,
    receiver: DeliveryReceiver,
}

impl ObjectInbox {
    pub(crate) fn new(position: u64, receiver: DeliveryReceiver) -> Self {
        Self { position, receiver }
    }

    #[inline]
    pub fn position(&self) -> u64 {
        self.position
    }
}

impl futures::stream::Stream for ObjectInbox {
    type Item = Delivery;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::pin!(&mut self.receiver).poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        futures::stream::Stream::size_hint(&self.receiver)
    }
}

impl futures::stream::FusedStream for ObjectInbox {
    fn is_terminated(&self) -> bool {
        futures::stream::FusedStream::is_terminated(&self.receiver)
    }
}
