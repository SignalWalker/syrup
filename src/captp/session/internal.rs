use super::{CapTpSessionCore, ExportToken, KeyMap, RecvError, SendError, SwissRegistry};
use crate::async_compat::{AsyncIoError, AsyncRead, AsyncWrite};
use dashmap::{DashMap, DashSet};
use ed25519_dalek::{SigningKey, VerifyingKey};
use futures::lock::Mutex;
use std::sync::{atomic::AtomicBool, Arc, RwLock};
use syrup::{Deserialize, Serialize};

pub(crate) struct CapTpSessionInternal<Reader, Writer> {
    core: CapTpSessionCore<Reader, Writer>,
    // msg_queue: flume::Receiver<Event<Socket>>,
    // msg_queue_sender: flume::Sender<Event<Socket>>,
    pub(super) signing_key: SigningKey,
    pub(super) remote_vkey: VerifyingKey,

    // pub(super) registry: Arc<SwissRegistry<Socket>>,
    /// Objects imported from the remote
    pub(super) imports: DashSet<u64>,
    /// Objects exported to the remote
    pub(super) exports: KeyMap<Arc<dyn crate::captp::object::Object + Send + Sync>>,
    /// Answers exported to the remote
    pub(super) answers: DashMap<u64, ()>,

    pub(super) recv_buf: Mutex<Vec<u8>>,

    pub(super) aborted_by_remote: RwLock<Option<String>>,
    pub(super) aborted_locally: AtomicBool,
}

#[cfg(feature = "extra_diagnostics")]
impl<Reader, Writer> Drop for CapTpSessionInternal<Reader, Writer> {
    fn drop(&mut self) {
        if !self.is_aborted() {
            tracing::warn!(session = ?self, "dropping non-aborted session");
        }
    }
}

impl<Reader, Writer> std::fmt::Debug for CapTpSessionInternal<Reader, Writer> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapTpSessionInternal")
            .field("remote_vkey", &crate::hash(&self.remote_vkey))
            .finish_non_exhaustive()
    }
}

impl<Reader, Writer> CapTpSessionInternal<Reader, Writer> {
    pub(crate) fn new(
        core: CapTpSessionCore<Reader, Writer>,
        signing_key: SigningKey,
        remote_vkey: VerifyingKey,
        // registry: Arc<SwissRegistry<Socket>>,
    ) -> Self {
        // let (sender, receiver) = flume::unbounded();
        Self {
            core,
            // msg_queue: receiver,
            // msg_queue_sender: sender,
            // registry,
            signing_key,
            remote_vkey,
            imports: DashSet::new(),
            // Bootstrap object handled internally.
            exports: KeyMap::with_initial(1),
            answers: DashMap::new(),
            recv_buf: Mutex::new(Vec::new()),
            aborted_by_remote: RwLock::default(),
            aborted_locally: false.into(),
        }
    }

    #[tracing::instrument(fields(msg = %syrup::ser::to_pretty(msg).unwrap()))]
    pub(super) async fn send_msg<Msg: Serialize>(&self, msg: &Msg) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        if self
            .aborted_locally
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            return Err(SendError::SessionAbortedLocally);
        }
        if let Some(reason) = self.aborted_by_remote.read().unwrap().as_ref() {
            return Err(SendError::SessionAborted(reason.clone()));
        }
        tracing::trace!("sending message");
        self.core.send_msg(msg).await.map_err(SendError::from)
    }

    #[tracing::instrument()]
    async fn recv(&self, max_size: usize) -> Result<usize, AsyncIoError>
    where
        Reader: AsyncRead + Unpin,
    {
        let mut recv_vec = self.recv_buf.lock().await;
        let orig_len = recv_vec.len();
        let new_len = orig_len + max_size;
        recv_vec.resize(new_len, 0);
        let amt = {
            let recv_buf = &mut recv_vec[orig_len..new_len];
            self.core.recv(recv_buf).await?
        };
        // drop unwritten bytes from the end of the vec
        recv_vec.truncate(orig_len + amt);

        Ok(amt)
    }

    #[tracing::instrument]
    async fn pop_msg<Msg>(&self) -> Result<Msg, RecvError>
    where
        for<'de> Msg: Deserialize<'de>,
    {
        let mut buf = self.recv_buf.lock().await;

        let (rem, res) = syrup::de::nom_bytes::<Msg>(&buf)?;

        let amt_consumed = buf.len() - rem.len();
        tracing::trace!(bytes = amt_consumed, "popped message");
        buf.drain(..amt_consumed);

        Ok(res)
    }

    pub(super) async fn recv_msg<Msg>(&self) -> Result<Msg, RecvError>
    where
        Reader: AsyncRead + Unpin,
        for<'de> Msg: Deserialize<'de>,
    {
        loop {
            if self
                .aborted_locally
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                return Err(RecvError::SessionAbortedLocally);
            }
            if let Some(reason) = self.aborted_by_remote.read().unwrap().as_ref() {
                return Err(RecvError::SessionAborted(reason.clone()));
            }
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

    pub(super) fn export(&self, val: Arc<dyn crate::captp::object::Object + Send + Sync>) -> u64 {
        let pos = self.exports.push(val);
        tracing::trace!(pos, "exporting object");
        pos
    }

    pub(super) fn local_abort(&self) {
        self.aborted_locally
            .store(true, std::sync::atomic::Ordering::Relaxed)
    }

    pub(super) fn set_remote_abort(&self, reason: String) {
        *self.aborted_by_remote.write().unwrap() = Some(reason);
    }

    pub(super) fn is_aborted(&self) -> bool {
        self.aborted_locally
            .load(std::sync::atomic::Ordering::Acquire)
            || self.aborted_by_remote.read().unwrap().is_some()
    }

    // fn gen_export(self: Arc<Self>) -> ObjectInbox<Socket> {
    //     let (sender, receiver) = futures::channel::mpsc::unbounded();
    //     let pos = self.exports.push(sender);
    //     ObjectInbox::new(pos, receiver)
    // }
}
