use std::sync::Arc;

use ed25519_dalek::VerifyingKey;
use futures::future::BoxFuture;
use syrup::{RawSyrup, Serialize};

use super::{
    msg::{DescExport, DescImport},
    AbstractCapTpSession, CapTpDeliver, Delivery, GenericResolver, RemoteKey, SendError,
};
use crate::async_compat::{mpsc, oneshot, OneshotRecvError};

mod bootstrap;
pub use bootstrap::*;

/// Sending half of an object pipe.
pub type DeliverySender = mpsc::UnboundedSender<Delivery>;
/// Receiving half of an object pipe.
pub type DeliveryReceiver = mpsc::UnboundedReceiver<Delivery>;

/// Returned by [`Object::deliver_only`]
pub enum ObjectOnlyError {}

/// Returned by [`Object`] functions.
#[derive(Debug, thiserror::Error)]
pub enum ObjectError {
    #[error(transparent)]
    Deliver(#[from] DeliverError),
    #[error(transparent)]
    DeliverOnly(#[from] DeliverOnlyError),
    #[error("missing argument at position {position}: {expected}")]
    MissingArgument {
        position: usize,
        expected: &'static str,
    },
    #[error("expected {expected} at position {position}, received: {received:?}")]
    UnexpectedArgument {
        expected: &'static str,
        position: usize,
        received: syrup::Item,
    },
}

impl ObjectError {
    pub fn missing(position: usize, expected: &'static str) -> Self {
        Self::MissingArgument { position, expected }
    }

    pub fn unexpected(expected: &'static str, position: usize, received: syrup::Item) -> Self {
        Self::UnexpectedArgument {
            expected,
            position,
            received,
        }
    }
}

pub trait Object {
    fn deliver_only(
        &self,
        session: Arc<dyn AbstractCapTpSession + Send + Sync>,
        args: Vec<syrup::Item>,
    ) -> Result<(), ObjectError>;

    fn deliver(
        &self,
        session: Arc<dyn AbstractCapTpSession + Send + Sync>,
        args: Vec<syrup::Item>,
        resolver: GenericResolver,
    ) -> BoxFuture<'_, Result<(), ObjectError>>;

    /// Called when this object is exported. By default, does nothing.
    #[allow(unused_variables)]
    fn exported(&self, remote_key: &VerifyingKey, position: DescExport) {}
}

// /// An object to which the answer to a Promise may be sent.
// pub struct RemoteResolver {
//     base: RemoteObject,
// }
//
// impl RemoteResolver {
//     pub async fn fulfill<'arg, Arg: Serialize + 'arg>(
//         &self,
//         args: impl IntoIterator<Item = &'arg Arg>,
//         answer_pos: Option<u64>,
//         resolve_me_desc: DescImport,
//     ) -> Result<(), SendError> {
//         self.base
//             .call("fulfill", args, answer_pos, resolve_me_desc)
//             .await
//     }
//
//     pub async fn break_promise(&self, error: impl Serialize) -> Result<(), SendError> {
//         self.base.call_only("break", &[error]).await
//     }
// }

pub type PromiseResult = Result<Vec<syrup::Item>, syrup::Item>;
pub type PromiseSender = oneshot::Sender<PromiseResult>;
pub type PromiseReceiver = oneshot::Receiver<PromiseResult>;

pub struct Resolver {
    sender: parking_lot::Mutex<Option<PromiseSender>>,
}

impl Resolver {
    fn resolve(&self, res: PromiseResult) -> Result<(), PromiseResult> {
        let Some(sender) = self.sender.lock().take() else {
            return Err(res);
        };
        sender.send(res)
    }
}

#[crate::impl_object(rexa = crate, tracing = ::tracing)]
impl Resolver {
    #[deliver()]
    async fn fulfill(
        &self,
        #[arg(args)] args: Vec<syrup::Item>,
        #[arg(resolver)] resolver: GenericResolver,
    ) -> Result<(), ObjectError> {
        match self.resolve(Ok(args)) {
            Ok(_) => Ok(()),
            Err(_) => resolver
                .break_promise("promise already resolved")
                .await
                .map_err(From::from),
        }
    }

    #[deliver(symbol = "break")]
    async fn break_promise(
        &self,
        #[arg(syrup = arg)] reason: syrup::Item,
        #[arg(resolver)] resolver: GenericResolver,
    ) -> Result<(), ObjectError> {
        match self.resolve(Err(reason)) {
            Ok(_) => Ok(()),
            Err(_) => resolver
                .break_promise("promise already resolved")
                .await
                .map_err(From::from),
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

/// An object representing the response to an [`OpDeliver`].
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

#[derive(Debug, thiserror::Error)]
pub enum DeliverOnlyError {
    #[error(transparent)]
    Serialize(#[from] syrup::Error<'static>),
    #[error(transparent)]
    Send(#[from] SendError),
}

#[derive(Debug, thiserror::Error)]
pub enum DeliverError {
    #[error(transparent)]
    Serialize(#[from] syrup::Error<'static>),
    #[error(transparent)]
    Send(#[from] SendError),
    #[error(transparent)]
    Recv(#[from] OneshotRecvError),
    #[error("promise broken, reason: {0:?}")]
    Broken(syrup::Item),
}

#[derive(Debug, thiserror::Error)]
pub enum RemoteError {
    #[error(transparent)]
    Deliver(#[from] DeliverError),
    #[error("missing argument at position {position}: {expected}")]
    MissingArgument {
        position: usize,
        expected: &'static str,
    },
    #[error("expected {expected} at position {position}, received: {received:?}")]
    UnexpectedArgument {
        expected: &'static str,
        position: usize,
        received: syrup::Item,
    },
}

impl RemoteError {
    pub fn missing(position: usize, expected: &'static str) -> Self {
        Self::MissingArgument { position, expected }
    }

    pub fn unexpected(expected: &'static str, position: usize, received: syrup::Item) -> Self {
        Self::UnexpectedArgument {
            expected,
            position,
            received,
        }
    }
}

#[derive(Clone)]
pub struct RemoteObject {
    position: DescExport,
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
        position: DescExport,
    ) -> Self {
        Self { session, position }
    }

    pub fn session(&self) -> &Arc<dyn CapTpDeliver + Send + Sync + 'static> {
        &self.session
    }

    pub fn remote_vkey(&self) -> RemoteKey {
        self.session.remote_vkey()
    }

    pub async fn deliver_only_serialized(&self, args: &[syrup::RawSyrup]) -> Result<(), SendError> {
        self.session.deliver_only(self.position, args).await
    }

    pub async fn deliver_only<'arg, Arg: Serialize + ?Sized + 'arg>(
        &self,
        args: impl IntoIterator<Item = &'arg Arg>,
    ) -> Result<(), DeliverOnlyError> {
        self.deliver_only_serialized(&RawSyrup::vec_from_iter(args.into_iter())?)
            .await
            .map_err(From::from)
    }

    pub async fn deliver_serialized(
        &self,
        args: &[syrup::RawSyrup],
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), SendError> {
        self.session
            .deliver(self.position, args, answer_pos, resolve_me_desc)
            .await
    }

    pub async fn deliver<'arg, Arg: Serialize + ?Sized + 'arg>(
        &self,
        args: impl IntoIterator<Item = &'arg Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), DeliverError> {
        self.deliver_serialized(
            &RawSyrup::vec_from_iter(args.into_iter())?,
            answer_pos,
            resolve_me_desc,
        )
        .await
        .map_err(From::from)
    }

    pub async fn deliver_and_serialized(
        &self,
        args: &[RawSyrup],
    ) -> Result<Vec<syrup::Item>, DeliverError> {
        self.session.deliver_and(self.position, args).await
    }

    pub async fn deliver_and<'arg, Arg: Serialize + ?Sized + 'arg>(
        &self,
        args: impl IntoIterator<Item = &'arg Arg>,
    ) -> Result<Vec<syrup::Item>, DeliverError> {
        self.deliver_and_serialized(
            &args
                .into_iter()
                .map(syrup::RawSyrup::from_serialize)
                .collect::<Vec<_>>(),
        )
        .await
    }

    pub async fn call_only<'arg, Arg: Serialize + ?Sized + 'arg>(
        &self,
        ident: impl AsRef<str>,
        args: impl IntoIterator<Item = &'arg Arg>,
    ) -> Result<(), DeliverOnlyError> {
        self.deliver_only_serialized(&RawSyrup::vec_from_ident_iter(ident, args.into_iter())?)
            .await
            .map_err(From::from)
    }

    pub async fn call<'arg, Arg: Serialize + ?Sized + 'arg>(
        &self,
        ident: impl AsRef<str>,
        args: impl IntoIterator<Item = &'arg Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), DeliverError> {
        self.deliver_serialized(
            &RawSyrup::vec_from_ident_iter(ident, args.into_iter())?,
            answer_pos,
            resolve_me_desc,
        )
        .await
        .map_err(From::from)
    }

    pub async fn call_and<'arg, Arg: Serialize + ?Sized + 'arg>(
        &self,
        ident: impl AsRef<str>,
        args: impl IntoIterator<Item = &'arg Arg>,
    ) -> Result<Vec<syrup::Item>, DeliverError> {
        self.deliver_and_serialized(&RawSyrup::vec_from_ident_iter(ident, args.into_iter())?)
            .await
    }

    // pub fn get_remote_object(&self, position: DescExport) -> Option<RemoteObject> {
    //     self.session.clone().into_remote_object(position)
    // }
    //
    // /// # Safety
    // /// - There must be an object exported at `position`.
    // #[allow(unsafe_code)]
    // pub unsafe fn get_remote_object_unchecked(&self, position: DescExport) -> RemoteObject {
    //     unsafe { self.session.clone().into_remote_object_unchecked(position) }
    // }
}
