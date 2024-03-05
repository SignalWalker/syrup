use super::{
    msg::{DescImport, OpAbort, OpDeliver, OpDeliverOnly, Operation},
    object::{RemoteBootstrap, RemoteObject},
};
use crate::async_compat::{AsyncRead, AsyncWrite};
use ed25519_dalek::{SigningKey, VerifyingKey};
use futures::FutureExt;
use std::sync::Arc;
use syrup::{Deserialize, Serialize};

mod builder;
pub use builder::*;

mod core;
pub use core::*;

// mod message_queue;
// pub use message_queue::*;

mod manager;
pub use manager::*;

mod error;
pub use error::*;

mod keymap;
pub use keymap::*;

mod registry;
pub use registry::*;

mod export_token;
pub use export_token::*;

mod internal;
pub use internal::*;

mod resolver;
pub use resolver::*;

mod event;
pub use event::*;

mod traits;
pub use traits::*;

pub struct CapTpSession<Reader, Writer> {
    base: Arc<CapTpSessionInternal<Reader, Writer>>,
}

impl<Reader, Writer> std::fmt::Debug for CapTpSession<Reader, Writer> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key_hash = crate::hash(self.remote_vkey());
        f.debug_struct("CapTpSession")
            .field("remote_vkey", &key_hash)
            .finish_non_exhaustive()
    }
}

impl<Reader, Writer> std::clone::Clone for CapTpSession<Reader, Writer> {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
        }
    }
}

impl<Reader, Writer> PartialEq for CapTpSession<Reader, Writer> {
    fn eq(&self, other: &Self) -> bool {
        self.remote_vkey() == other.remote_vkey() && self.signing_key() == other.signing_key()
    }
}

impl<Reader, Writer> From<Arc<CapTpSessionInternal<Reader, Writer>>>
    for CapTpSession<Reader, Writer>
{
    fn from(base: Arc<CapTpSessionInternal<Reader, Writer>>) -> Self {
        Self { base }
    }
}

impl<Reader, Writer> From<&'_ Arc<CapTpSessionInternal<Reader, Writer>>>
    for CapTpSession<Reader, Writer>
{
    fn from(base: &'_ Arc<CapTpSessionInternal<Reader, Writer>>) -> Self {
        Self { base: base.clone() }
    }
}

impl<Reader, Writer> CapTpSession<Reader, Writer> {
    pub fn signing_key(&self) -> &SigningKey {
        &self.base.signing_key
    }

    pub fn remote_vkey(&self) -> &VerifyingKey {
        &self.base.remote_vkey
    }

    pub fn export(&self, obj: Arc<dyn super::object::Object + Send + Sync>) -> u64 {
        self.base.export(obj)
    }

    pub fn is_aborted(&self) -> bool {
        self.base.is_aborted()
    }

    pub async fn abort(self, reason: impl Into<OpAbort>) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        let res = self.send_msg(&reason.into()).await;
        self.base.local_abort();
        res
    }

    pub fn get_remote_object(self, position: u64) -> Option<RemoteObject<Reader, Writer>> {
        if position != 0 && !self.base.imports.contains(&position) {
            None
        } else {
            Some(RemoteObject::new(self, position))
        }
    }

    pub fn get_remote_bootstrap(self) -> RemoteBootstrap<Reader, Writer> {
        RemoteBootstrap::new(self)
    }

    // pub fn gen_export(&self) -> ObjectInbox<Socket> {
    //     self.base.clone().gen_export()
    // }

    pub fn event_stream<'s>(
        &'s self,
    ) -> impl futures::stream::Stream<Item = Result<Event, RecvError>> + 's
    where
        Reader: AsyncRead + Send + Unpin + 'static,
        Writer: AsyncWrite + Send + Unpin + 'static,
    {
        futures::stream::unfold(self, |session| async move {
            Some((session.recv_event().await, session))
        })
    }

    pub fn into_event_stream(
        self,
    ) -> impl futures::stream::Stream<Item = Result<Event, RecvError>> + Unpin
    where
        Reader: AsyncRead + Unpin + Send + 'static,
        Writer: AsyncWrite + Unpin + Send + 'static,
    {
        use futures::StreamExt;
        async fn recv<Reader, Writer>(
            session: CapTpSession<Reader, Writer>,
        ) -> Option<(Result<Event, RecvError>, CapTpSession<Reader, Writer>)>
        where
            Reader: AsyncRead + Send + Unpin + 'static,
            Writer: AsyncWrite + Send + Unpin + 'static,
        {
            Some((session.recv_event().await, session))
        }
        futures::stream::unfold(self, recv).boxed()
    }

    // #[tracing::instrument()]
    pub async fn recv_event(&self) -> Result<Event, RecvError>
    where
        Reader: AsyncRead + Send + Unpin + 'static,
        Writer: AsyncWrite + Send + Unpin + 'static,
    {
        self.base.recv_event().await
    }
}

impl<Reader, Writer> CapTpSession<Reader, Writer> {
    pub async fn deliver_only<Arg: Serialize>(
        &self,
        position: u64,
        args: Vec<Arg>,
    ) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.send_msg(&OpDeliverOnly::new(position, args)).await
    }

    pub async fn deliver<Arg: Serialize>(
        &self,
        position: u64,
        args: Vec<Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.send_msg(&OpDeliver::new(position, args, answer_pos, resolve_me_desc))
            .await
    }

    pub async fn deliver_and<Arg: Serialize>(
        &self,
        position: u64,
        args: Vec<Arg>,
    ) -> Result<super::object::Answer, SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        let (resolver, answer) = super::object::Resolver::new();
        let pos = self.export(resolver);
        self.deliver(position, args, None, DescImport::Object(pos.into()))
            .await?;
        Ok(answer)
    }

    pub async fn recv_msg<Msg>(&self) -> Result<Msg, RecvError>
    where
        Reader: AsyncRead + Unpin,
        for<'de> Msg: Deserialize<'de>,
    {
        self.base.recv_msg::<Msg>().await
    }

    pub async fn send_msg<Msg: Serialize>(&self, msg: &Msg) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.base.send_msg(msg).await
    }

    // pub(crate) async fn recv_delivery_for(
    //     &self,
    //     position: u64,
    // ) -> Result<Delivery<Socket>, RecvError>
    // where
    //     Socket: AsyncRead + Unpin,
    // {
    //     loop {
    //         match self.recv_event().await? {
    //             Event::Delivery(del) if del.position() == position => break Ok(del),
    //             ev => self.base.msg_queue_sender.send(ev).unwrap(),
    //         }
    //     }
    // }
}
