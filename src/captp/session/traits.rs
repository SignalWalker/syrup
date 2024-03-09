use super::{CapTpSession, CapTpSessionInternal, Event, RecvError, SendError};
use crate::async_compat::{AsyncRead, AsyncWrite};
use crate::captp::msg::{OpAbort, OpDeliver, OpDeliverOnly};
use crate::captp::object::{RemoteBootstrap, RemoteObject, Resolver};
use crate::captp::{msg::DescImport, object::Answer};
use ed25519_dalek::{SigningKey, VerifyingKey};
use futures::future::BoxFuture;
use futures::FutureExt;
use std::future::Future;
use std::sync::Arc;

pub(crate) trait CapTpDeliver {
    fn deliver_only<'f>(
        &'f self,
        position: u64,
        args: Vec<syrup::RawSyrup>,
    ) -> futures::future::BoxFuture<'f, Result<(), SendError>>;
    fn deliver<'f>(
        &'f self,
        position: u64,
        args: Vec<syrup::RawSyrup>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> futures::future::BoxFuture<'f, Result<(), SendError>>;
    fn deliver_and<'f>(
        &'f self,
        position: u64,
        args: Vec<syrup::RawSyrup>,
    ) -> futures::future::BoxFuture<'f, Result<Answer, SendError>>;
    fn into_remote_object(self: Arc<Self>, position: u64) -> Option<RemoteObject>;
    unsafe fn into_remote_object_unchecked(self: Arc<Self>, position: u64) -> RemoteObject;
}

/// Allows dynamic dispatch for CapTpSessions.
pub trait AbstractCapTpSession {
    fn signing_key(&self) -> &SigningKey;
    fn remote_vkey(&self) -> &VerifyingKey;
    fn export(&self, obj: Arc<dyn crate::captp::object::Object + Send + Sync>) -> u64;
    fn is_aborted(&self) -> bool;
    fn abort<'s>(&'s self, reason: String) -> BoxFuture<'s, Result<(), SendError>>;
    fn recv_event<'s>(self: Arc<Self>) -> BoxFuture<'s, Result<Event, RecvError>>;
    fn into_remote_bootstrap(self: Arc<Self>) -> RemoteBootstrap;
}

impl<Reader, Writer> CapTpDeliver for CapTpSessionInternal<Reader, Writer>
where
    Reader: Send + 'static,
    Writer: AsyncWrite + Send + Unpin + 'static,
{
    fn deliver_only<'f>(
        &'f self,
        position: u64,
        args: Vec<syrup::RawSyrup>,
    ) -> futures::future::BoxFuture<'f, Result<(), SendError>> {
        let del = OpDeliverOnly::new(position, args);
        async move { self.send_msg(&del).await }.boxed()
    }

    fn deliver<'f>(
        &'f self,
        position: u64,
        args: Vec<syrup::RawSyrup>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> futures::future::BoxFuture<'f, Result<(), SendError>> {
        let del = OpDeliver::new(position, args, answer_pos, resolve_me_desc);
        async move { self.send_msg(&del).await }.boxed()
    }

    fn deliver_and<'f>(
        &'f self,
        position: u64,
        args: Vec<syrup::RawSyrup>,
    ) -> futures::future::BoxFuture<'f, Result<Answer, SendError>> {
        let (resolver, answer) = Resolver::new();
        let pos = self.export(resolver);
        async move {
            self.deliver(position, args, None, DescImport::Object(pos.into()))
                .await?;
            Ok(answer)
        }
        .boxed()
    }

    fn into_remote_object(self: Arc<Self>, position: u64) -> Option<RemoteObject> {
        if position != 0 && !self.imports.contains(&position) {
            None
        } else {
            Some(RemoteObject::new(self.clone(), position))
        }
    }

    unsafe fn into_remote_object_unchecked(self: Arc<Self>, position: u64) -> RemoteObject {
        RemoteObject::new(self.clone(), position)
    }
}

impl<Reader, Writer> AbstractCapTpSession for CapTpSessionInternal<Reader, Writer>
where
    Reader: AsyncRead + Send + Unpin + 'static,
    Writer: AsyncWrite + Send + Unpin + 'static,
{
    fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    fn remote_vkey(&self) -> &VerifyingKey {
        &self.remote_vkey
    }

    fn export(&self, obj: Arc<dyn crate::captp::object::Object + Send + Sync>) -> u64 {
        self.export(obj)
    }

    fn is_aborted(&self) -> bool {
        self.is_aborted()
    }

    fn abort<'s>(&'s self, reason: String) -> BoxFuture<'s, Result<(), SendError>> {
        async move {
            let res = self.send_msg(&OpAbort::from(reason)).await;
            self.local_abort();
            res
        }
        .boxed()
    }

    fn recv_event<'s>(self: Arc<Self>) -> BoxFuture<'s, Result<Event, RecvError>> {
        CapTpSessionInternal::recv_event(self).boxed()
    }

    fn into_remote_bootstrap(self: Arc<Self>) -> RemoteBootstrap {
        RemoteBootstrap::new(self)
    }
}
