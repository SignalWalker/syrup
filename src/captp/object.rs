use super::{
    msg::DescImport, AbstractCapTpSession, CapTpDeliver, Delivery, GenericResolver, SendError,
};
use crate::async_compat::oneshot;
use futures::future::BoxFuture;
use std::sync::Arc;
use syrup::{Serialize, Symbol};

mod bootstrap;
pub use bootstrap::*;

/// Sending half of an object pipe.
pub type DeliverySender = futures::channel::mpsc::UnboundedSender<Delivery>;
/// Receiving half of an object pipe.
pub type DeliveryReceiver = futures::channel::mpsc::UnboundedReceiver<Delivery>;

pub trait Object {
    // TODO :: Better error type
    fn deliver_only(
        &self,
        session: &(dyn AbstractCapTpSession + Sync),
        args: Vec<syrup::Item>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
    // TODO :: Better error type
    fn deliver<'result>(
        &'result self,
        session: &'result (dyn AbstractCapTpSession + Sync),
        args: Vec<syrup::Item>,
        resolver: GenericResolver,
    ) -> BoxFuture<'result, Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>>;
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
pub struct RemoteResolver {
    base: RemoteObject,
}

impl RemoteResolver {
    pub async fn fulfill<'arg, Arg: Serialize + 'arg>(
        &self,
        args: impl IntoIterator<Item = &'arg Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), SendError> {
        self.base
            .call("fulfill", args, answer_pos, resolve_me_desc)
            .await
    }

    pub async fn break_promise(&self, error: impl Serialize) -> Result<(), SendError> {
        self.base.call_only("break", &[error]).await
    }
}

pub type PromiseResult = Result<Vec<syrup::Item>, syrup::Item>;
pub type PromiseSender = oneshot::Sender<PromiseResult>;
pub type PromiseReceiver = oneshot::Receiver<PromiseResult>;

pub struct Resolver {
    sender: std::sync::Mutex<Option<PromiseSender>>,
}

// TODO :: convert this to #[impl_object]
impl Object for Resolver {
    fn deliver_only(
        &self,
        _session: &(dyn AbstractCapTpSession + Sync),
        _args: Vec<syrup::Item>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        todo!()
    }

    fn deliver<'s>(
        &'s self,
        _: &(dyn AbstractCapTpSession + Sync),
        args: Vec<syrup::Item>,
        _: GenericResolver,
    ) -> BoxFuture<'s, Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>> {
        use futures::FutureExt;
        async move {
            let sender = match self.sender.lock().unwrap().take() {
                Some(s) => s,
                None => return Err("broken promise pipe".into()),
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
            Ok(())
        }
        .boxed()
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
pub struct RemoteObject {
    position: u64,
    session: Arc<dyn CapTpDeliver + Send + Sync + 'static>,
}

impl std::fmt::Debug for RemoteObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteObject")
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

impl RemoteObject {
    pub(crate) fn new(
        session: Arc<dyn CapTpDeliver + Send + Sync + 'static>,
        position: u64,
    ) -> Self {
        Self { session, position }
    }

    pub async fn deliver_only<Arg: Serialize>(&self, args: Vec<Arg>) -> Result<(), SendError> {
        self.session
            .deliver_only(
                self.position,
                args.iter()
                    .map(syrup::RawSyrup::from_serialize)
                    .collect::<Vec<_>>(),
            )
            .await
    }

    pub async fn deliver<Arg: Serialize>(
        &self,
        args: Vec<Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), SendError> {
        self.session
            .deliver(
                self.position,
                args.iter()
                    .map(syrup::RawSyrup::from_serialize)
                    .collect::<Vec<_>>(),
                answer_pos,
                resolve_me_desc,
            )
            .await
    }

    pub async fn deliver_and<Arg: Serialize>(&self, args: Vec<Arg>) -> Result<Answer, SendError> {
        self.session
            .deliver_and(
                self.position,
                args.iter()
                    .map(syrup::RawSyrup::from_serialize)
                    .collect::<Vec<_>>(),
            )
            .await
    }

    pub async fn call_only<'arg, Arg: Serialize + 'arg>(
        &self,
        ident: impl AsRef<str>,
        args: impl IntoIterator<Item = &'arg Arg>,
    ) -> Result<(), SendError> {
        self.deliver_only(syrup::raw_syrup_unwrap![&Symbol(ident.as_ref()); args])
            .await
    }

    pub async fn call<'arg, Arg: Serialize + 'arg>(
        &self,
        ident: impl AsRef<str>,
        args: impl IntoIterator<Item = &'arg Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), SendError> {
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
    ) -> Result<Answer, SendError> {
        self.deliver_and(syrup::raw_syrup_unwrap![&Symbol(ident.as_ref()); args])
            .await
    }

    pub fn get_remote_object(&self, position: u64) -> Option<RemoteObject> {
        self.session.clone().into_remote_object(position)
    }

    #[allow(unsafe_code)]
    pub unsafe fn get_remote_object_unchecked(&self, position: u64) -> RemoteObject {
        unsafe { self.session.clone().into_remote_object_unchecked(position) }
    }
}
