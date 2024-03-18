use super::{CapTpSessionCore, KeyMap, RecvError, SendError};
use crate::{
    async_compat::{AsyncRead, AsyncWrite},
    captp::msg::Operation,
    locator::NodeLocator,
};
use dashmap::{DashMap, DashSet};
use ed25519_dalek::{SigningKey, VerifyingKey};
use futures::lock::Mutex;
use std::sync::{atomic::AtomicBool, Arc, RwLock};
use syrup::{Deserialize, Serialize};

pub(crate) struct CapTpSessionInternal<Reader, Writer> {
    core: CapTpSessionCore<Reader, Writer>,
    pub(super) signing_key: SigningKey,
    pub(super) remote_vkey: VerifyingKey,

    /// Objects imported from the remote
    pub(super) imports: DashSet<u64>,
    /// Objects exported to the remote
    pub(super) exports: KeyMap<Arc<dyn crate::captp::object::Object + Send + Sync>>,
    /// Answers exported to the remote
    pub(super) answers: DashMap<u64, ()>,

    pub(super) recv_buf: Mutex<Vec<u8>>,

    pub(super) aborted_by_remote: RwLock<Option<String>>,
    pub(super) aborted_locally: AtomicBool,

    pub(super) locator_serialized: Vec<u8>,
}

#[cfg(feature = "extra-diagnostics")]
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
    pub(super) fn new<HKey, HVal>(
        core: CapTpSessionCore<Reader, Writer>,
        signing_key: SigningKey,
        remote_vkey: VerifyingKey,
        locator: &NodeLocator<HKey, HVal>,
    ) -> Self
    where
        NodeLocator<HKey, HVal>: Serialize,
    {
        Self {
            core,
            signing_key,
            remote_vkey,
            imports: DashSet::new(),
            // Bootstrap object handled internally.
            exports: KeyMap::with_initial(1),
            answers: DashMap::new(),
            recv_buf: Mutex::new(Vec::new()),
            aborted_by_remote: RwLock::default(),
            aborted_locally: false.into(),

            locator_serialized: syrup::ser::to_bytes(locator).unwrap(),
        }
    }

    #[tracing::instrument(skip(msg))]
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
        tracing::trace!(msg = %syrup::ser::to_pretty(msg).unwrap(), "sending message");
        self.core.send_msg(msg).await.map_err(SendError::from)
    }

    #[tracing::instrument()]
    async fn recv(&self, max_size: usize) -> Result<usize, std::io::Error>
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
        let pos = self.exports.push(val.clone());
        val.exported(&self.remote_vkey, pos.into());
        pos
    }

    pub(super) fn local_abort(&self) {
        self.aborted_locally
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub(super) fn set_remote_abort(&self, reason: String) {
        *self.aborted_by_remote.write().unwrap() = Some(reason);
    }

    pub(super) fn is_aborted(&self) -> bool {
        self.aborted_locally
            .load(std::sync::atomic::Ordering::Acquire)
            || self.aborted_by_remote.read().unwrap().is_some()
    }

    // TODO :: propagate delivery errors
    pub(super) async fn recv_event(self: Arc<Self>) -> Result<super::Event, RecvError>
    where
        Reader: AsyncRead + Send + Unpin + 'static,
        Writer: AsyncWrite + Send + Unpin + 'static,
    {
        fn bootstrap_deliver_only(args: Vec<syrup::Item>) -> super::Event {
            use syrup::Item;
            let mut args = args.into_iter();
            match args.next() {
                Some(Item::Symbol(ident)) => match ident.as_str() {
                    "deposit-gift" => todo!("bootstrap: deposit-gift"),
                    id => todo!("unrecognized bootstrap function: {id}"),
                },
                _ => todo!(),
            }
        }
        fn bootstrap_deliver<Reader, Writer>(
            session: Arc<CapTpSessionInternal<Reader, Writer>>,
            args: Vec<syrup::Item>,
            answer_pos: Option<u64>,
            resolve_me_desc: crate::captp::msg::DescImport,
        ) -> super::Event
        where
            Writer: AsyncWrite + Send + Unpin + 'static,
            Reader: Send + 'static,
        {
            use syrup::Item;
            let mut args = args.into_iter();
            match args.next() {
                Some(Item::Symbol(ident)) => match ident.as_str() {
                    "fetch" => {
                        let swiss = match args.next() {
                            Some(Item::Bytes(swiss)) => swiss,
                            Some(s) => todo!("malformed swiss num: {s:?}"),
                            None => todo!("missing swiss num"),
                        };
                        super::Event::Bootstrap(crate::captp::BootstrapEvent::Fetch {
                            resolver: crate::captp::GenericResolver::new(
                                session,
                                answer_pos,
                                resolve_me_desc,
                            )
                            .into(),
                            swiss,
                        })
                    }
                    "withdraw-gift" => todo!("bootstrap: withdraw-gift"),
                    id => todo!("unrecognized bootstrap function: {id}"),
                },
                _ => todo!(),
            }
        }
        loop {
            tracing::trace!("awaiting message");
            let msg = self
                .recv_msg::<crate::captp::msg::Operation<syrup::Item>>()
                .await?;
            tracing::debug!(?msg, "received message");
            match msg {
                Operation::DeliverOnly(del) => match del.to_desc.position {
                    0 => break Ok(bootstrap_deliver_only(del.args)),
                    pos => {
                        // let del = Delivery::DeliverOnly {
                        //     to_desc: del.to_desc,
                        //     args: del.args,
                        // };
                        // break Ok(Event::Delivery(del));
                        match self.exports.get(&pos) {
                            Some(obj) => obj.deliver_only(self.clone(), del.args).unwrap(),
                            None => break Err(RecvError::UnknownTarget(pos, del.args)),
                        }
                    }
                },
                Operation::Deliver(del) => match del.to_desc.position {
                    0 => {
                        break Ok(bootstrap_deliver(
                            self.clone(),
                            del.args,
                            del.answer_pos,
                            del.resolve_me_desc,
                        ))
                    }
                    pos => {
                        // let del = Delivery::Deliver {
                        //     to_desc: del.to_desc,
                        //     args: del.args,
                        //     resolver: GenericResolver {
                        //         session: self.clone(),
                        //         answer_pos: del.answer_pos,
                        //         resolve_me_desc: del.resolve_me_desc,
                        //     },
                        // };
                        // break Ok(Event::Delivery(del));
                        match self.exports.get(&pos) {
                            Some(obj) => obj
                                .deliver(
                                    self.clone(),
                                    del.args,
                                    crate::captp::GenericResolver::new(
                                        self.clone(),
                                        del.answer_pos,
                                        del.resolve_me_desc,
                                    ),
                                )
                                .await
                                .unwrap(),
                            None => break Err(RecvError::UnknownTarget(pos, del.args)),
                        }
                    }
                },
                Operation::Abort(crate::captp::msg::OpAbort { reason }) => {
                    self.set_remote_abort(reason.clone());
                    break Ok(super::Event::Abort(reason));
                }
            }
        }
    }

    // fn gen_export(self: Arc<Self>) -> ObjectInbox<Socket> {
    //     let (sender, receiver) = futures::channel::mpsc::unbounded();
    //     let pos = self.exports.push(sender);
    //     ObjectInbox::new(pos, receiver)
    // }
}
