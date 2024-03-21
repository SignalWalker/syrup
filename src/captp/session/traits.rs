use std::sync::Arc;

use ed25519_dalek::{SigningKey, VerifyingKey};
use futures::future::BoxFuture;
use futures::FutureExt;
use syrup::RawSyrup;

use super::{CapTpSessionInternal, Event, RecvError, RemoteKey, SendError};
use crate::captp::msg::{DescExport, OpAbort, OpDeliverOnlySlice, OpDeliverSlice};
use crate::captp::object::{DeliverError, RemoteBootstrap, RemoteObject, Resolver};
use crate::captp::{msg::DescImport, ExportManager};
use crate::{
    async_compat::{AsyncRead, AsyncWrite},
    captp::object::Object,
};

pub trait IntoExport {
    fn into_export(self) -> Arc<dyn Object + Send + Sync + 'static>;
}

impl<Obj: Object + Send + Sync + 'static> IntoExport for Arc<Obj> {
    #[inline(always)]
    fn into_export(self) -> Arc<dyn Object + Send + Sync + 'static> {
        self
    }
}

impl IntoExport for Arc<dyn Object + Send + Sync + 'static> {
    #[inline(always)]
    fn into_export(self) -> Arc<dyn Object + Send + Sync + 'static> {
        self
    }
}

pub(crate) trait CapTpDeliver {
    fn deliver_only<'f>(
        &'f self,
        position: DescExport,
        args: &'f [RawSyrup],
    ) -> futures::future::BoxFuture<'f, Result<(), SendError>>;
    fn deliver<'f>(
        &'f self,
        position: DescExport,
        args: &'f [RawSyrup],
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> futures::future::BoxFuture<'f, Result<(), SendError>>;
    fn deliver_and<'f>(
        &'f self,
        position: DescExport,
        args: &'f [RawSyrup],
    ) -> futures::future::BoxFuture<'f, Result<Vec<syrup::Item>, DeliverError>>;
    fn into_remote_object(self: Arc<Self>, position: DescExport) -> Option<RemoteObject>;
    #[allow(unsafe_code)]
    unsafe fn into_remote_object_unchecked(self: Arc<Self>, position: DescExport) -> RemoteObject;

    fn remote_vkey(&self) -> RemoteKey;
}

/// Allows dynamic dispatch for `CapTpSession`s.
pub trait AbstractCapTpSession {
    fn signing_key(&self) -> &SigningKey;
    fn remote_vkey(&self) -> &VerifyingKey;
    fn exports(&self) -> &ExportManager;
    fn into_remote_object(self: Arc<Self>, position: DescExport) -> Option<RemoteObject>;
    /// # Safety
    /// - An object must already be exported at `position`.
    #[allow(unsafe_code)]
    unsafe fn into_remote_object_unchecked(self: Arc<Self>, position: DescExport) -> RemoteObject;
    fn is_aborted(&self) -> bool;
    fn abort<'result>(
        &'result self,
        reason: &'result OpAbort,
    ) -> BoxFuture<'result, Result<(), SendError>>;
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
        position: DescExport,
        args: &'f [syrup::RawSyrup],
    ) -> futures::future::BoxFuture<'f, Result<(), SendError>> {
        let del = OpDeliverOnlySlice::new(position, args);
        async move { self.send_msg(&del).await }.boxed()
    }

    fn deliver<'f>(
        &'f self,
        position: DescExport,
        args: &'f [syrup::RawSyrup],
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> futures::future::BoxFuture<'f, Result<(), SendError>> {
        let del = OpDeliverSlice::new(position, args, answer_pos, resolve_me_desc);
        async move { self.send_msg(&del).await }.boxed()
    }

    fn deliver_and<'f>(
        &'f self,
        position: DescExport,
        args: &'f [syrup::RawSyrup],
    ) -> futures::future::BoxFuture<'f, Result<Vec<syrup::Item>, DeliverError>> {
        let (resolver, answer) = Resolver::new();
        let pos = self.exports.export(resolver);
        async move {
            self.deliver(position, args, None, DescImport::Object(pos.into()))
                .await?;
            answer.await?.map_err(DeliverError::Broken)
        }
        .boxed()
    }

    fn into_remote_object(self: Arc<Self>, position: DescExport) -> Option<RemoteObject> {
        if position.position != 0 && !self.imports.contains(&position.position) {
            None
        } else {
            Some(RemoteObject::new(self.clone(), position))
        }
    }

    #[allow(unsafe_code)]
    unsafe fn into_remote_object_unchecked(self: Arc<Self>, position: DescExport) -> RemoteObject {
        RemoteObject::new(self.clone(), position)
    }

    fn remote_vkey(&self) -> RemoteKey {
        self.remote_vkey
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

    #[inline]
    fn exports(&self) -> &ExportManager {
        &self.exports
    }

    fn into_remote_object(self: Arc<Self>, position: DescExport) -> Option<RemoteObject> {
        <Self as CapTpDeliver>::into_remote_object(self, position)
    }

    #[allow(unsafe_code)]
    unsafe fn into_remote_object_unchecked(self: Arc<Self>, position: DescExport) -> RemoteObject {
        unsafe { <Self as CapTpDeliver>::into_remote_object_unchecked(self, position) }
    }

    fn is_aborted(&self) -> bool {
        self.is_aborted()
    }

    fn abort<'f>(&'f self, reason: &'f OpAbort) -> BoxFuture<'f, Result<(), SendError>> {
        async move {
            let res = self.send_msg(reason).await;
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
