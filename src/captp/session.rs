use super::{
    msg::{DescImport, OpAbort, OpDeliver, OpDeliverOnly},
    object::{
        Answer, DeliveryReceiver, DeliverySender, LocalObject, RemoteBootstrap, RemoteObject,
    },
};
use dashmap::{DashMap, DashSet};
use ed25519_dalek::{SigningKey, VerifyingKey};
use futures::{lock::Mutex, AsyncRead, AsyncWrite, SinkExt};
use std::sync::{atomic::AtomicU64, Arc};
use syrup::{de::DeserializeError, Deserialize, RawSyrup, Serialize};

mod builder;
pub use builder::*;

mod core;
pub use core::*;

// mod message_queue;
// pub use message_queue::*;

mod manager;
pub use manager::*;

#[derive(Debug, thiserror::Error)]
pub enum RecvError {
    #[error("failed to parse syrup: {0:?}")]
    Parse(syrup::ErrorKind),
    #[error(transparent)]
    Io(#[from] futures::io::Error),
}

impl<'input> From<syrup::Error<'input>> for RecvError {
    fn from(value: syrup::Error<'input>) -> Self {
        Self::Parse(value.kind)
    }
}

#[derive(Debug)]
struct CapTpSessionInternal<Socket> {
    core: Mutex<CapTpSessionCore<Socket>>,

    signing_key: SigningKey,
    remote_vkey: VerifyingKey,

    /// Objects imported from the remote
    imports: DashSet<u64>,
    /// Objects exported to the remote
    exports: DashMap<u64, DeliverySender>,

    export_key: AtomicU64,

    recv_buf: Mutex<Vec<u8>>,
}

impl<Socket> CapTpSessionInternal<Socket> {
    pub(crate) fn new(
        core: CapTpSessionCore<Socket>,
        signing_key: SigningKey,
        remote_vkey: VerifyingKey,
    ) -> Self {
        Self {
            core: Mutex::new(core),
            signing_key,
            remote_vkey,
            imports: DashSet::new(),
            // Bootstrap object handled internally.
            exports: DashMap::new(),
            export_key: 1.into(),
            recv_buf: Mutex::new(Vec::new()),
        }
    }

    async fn send_msg<Msg: Serialize>(&self, msg: &Msg) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        self.core.lock().await.send_msg(msg).await
    }

    async fn recv(&self, max_size: usize) -> Result<usize, futures::io::Error>
    where
        Socket: AsyncRead + Unpin,
    {
        let mut recv_vec = self.recv_buf.lock().await;
        let orig_len = recv_vec.len();
        let new_len = orig_len + max_size;
        recv_vec.resize(new_len, 0);
        let amt = {
            let recv_buf = &mut recv_vec[orig_len..new_len];
            self.core.lock().await.recv(recv_buf).await?
        };
        // drop unwritten bytes from the end of the vec
        recv_vec.truncate(orig_len + amt);

        Ok(amt)
    }

    async fn pop_msg<Msg>(&self) -> Result<Msg, RecvError>
    where
        for<'de> Msg: Deserialize<'de>,
    {
        let mut buf = self.recv_buf.lock().await;

        let (rem, res) = syrup::de::nom_bytes::<Msg>(&buf)?;

        let rem_len = rem.len();
        buf.drain(..rem_len);

        Ok(res)
    }

    async fn recv_msg<Msg>(&self) -> Result<Msg, RecvError>
    where
        Socket: AsyncRead + Unpin,
        for<'de> Msg: Deserialize<'de>,
    {
        loop {
            match self.pop_msg::<Msg>().await {
                Ok(m) => {
                    return Ok(m);
                }
                Err(e) => match e {
                    RecvError::Parse(syrup::ErrorKind::Incomplete(n)) => {
                        self.recv(match n {
                            syrup::de::Needed::Unknown => 1024,
                            syrup::de::Needed::Size(a) => a.into(),
                        })
                        .await?;
                    }
                    _ => return Err(e),
                },
            }
        }
    }

    async fn try_recv_msg<Msg>(&self) -> Result<Option<Msg>, futures::io::Error>
    where
        Socket: AsyncRead + Unpin,
        for<'de> Msg: Deserialize<'de>,
    {
        match self.recv_msg::<Msg>().await {
            Ok(o) => Ok(Some(o)),
            Err(RecvError::Parse(_)) => Ok(None),
            Err(RecvError::Io(e)) => Err(e),
        }
    }

    async fn process_next_operation(&self) -> Result<(), RecvError>
    where
        Socket: AsyncRead + Unpin,
    {
        if let Some(del_only) = self.try_recv_msg::<OpDeliverOnly<syrup::Item>>().await? {
            match self
                .exports
                .get_mut(&del_only.to_desc.position)
                .unwrap()
                .send((del_only.args, None))
                .await
            {
                Ok(_) => Ok(()),
                Err(e) => todo!(),
            }
        } else if let Some(del) = self.try_recv_msg::<OpDeliver<syrup::Item>>().await? {
            match self
                .exports
                .get_mut(&del.to_desc.position)
                .unwrap()
                .send((del.args, Some(del.resolve_me_desc)))
                .await
            {
                Ok(_) => Ok(()),
                Err(e) => todo!(),
            }
        } else if let Some(abort) = self.try_recv_msg::<OpAbort>().await? {
            todo!()
        } else {
            Ok(())
        }
    }

    /// Get the next export key and advance it by 1.
    fn advance_export_key(&self) -> u64 {
        self.export_key
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
    }

    fn gen_export(self: Arc<Self>) -> LocalObject<Socket> {
        let position = self.advance_export_key();
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        self.exports.insert(position, sender);
        LocalObject::new(CapTpSession { base: self }, position, receiver)
    }
}

#[derive(Debug, Clone)]
pub struct CapTpSession<Socket> {
    base: Arc<CapTpSessionInternal<Socket>>,
}

impl<Socket> CapTpSession<Socket> {
    pub fn signing_key(&self) -> &SigningKey {
        &self.base.signing_key
    }

    pub fn remote_vkey(&self) -> &VerifyingKey {
        &self.base.remote_vkey
    }

    pub async fn recv_msg<Msg>(&self) -> Result<Msg, RecvError>
    where
        Socket: AsyncRead + Unpin,
        for<'de> Msg: Deserialize<'de>,
    {
        self.base.recv_msg::<Msg>().await
    }

    pub async fn send_msg<Msg: Serialize>(&self, msg: &Msg) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        self.base.send_msg(msg).await
    }

    pub async fn abort(self, reason: impl Into<OpAbort>) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        self.send_msg(&reason.into()).await
    }

    pub fn get_remote_object(self, position: u64) -> Option<RemoteObject<Socket>> {
        if position != 0 && !self.base.imports.contains(&position) {
            None
        } else {
            Some(RemoteObject::new(self, position))
        }
    }

    pub fn get_remote_bootstrap(self) -> RemoteBootstrap<Socket> {
        RemoteBootstrap::new(self)
    }

    pub(crate) async fn deliver_only<Arg: Serialize>(
        &self,
        position: u64,
        args: Vec<Arg>,
    ) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        self.send_msg(&OpDeliverOnly::new(position, args)).await
    }

    pub(crate) async fn deliver<Arg: Serialize>(
        &self,
        position: u64,
        args: Vec<Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        self.send_msg(&OpDeliver::new(position, args, answer_pos, resolve_me_desc))
            .await
    }

    pub fn gen_export(&self) -> LocalObject<Socket> {
        self.base.clone().gen_export()
    }
}

impl<Socket> From<Arc<CapTpSessionInternal<Socket>>> for CapTpSession<Socket> {
    fn from(base: Arc<CapTpSessionInternal<Socket>>) -> Self {
        Self { base }
    }
}

impl<Socket> From<&'_ Arc<CapTpSessionInternal<Socket>>> for CapTpSession<Socket> {
    fn from(base: &'_ Arc<CapTpSessionInternal<Socket>>) -> Self {
        Self { base: base.clone() }
    }
}
