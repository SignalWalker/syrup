//! - [Draft Specification](<https://github.com/ocapn/ocapn/blob/main/draft-specifications/CapTP Specification.md>)

use self::msg::{OpAbort, OpStartSession};
use crate::{locator::NodeLocator, CAPTP_VERSION};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Future};
use rand::rngs::OsRng;
use smol::lock::Mutex;
use std::{
    marker::PhantomData,
    pin::Pin,
    task::{ready, Poll},
};
use syrup::{de::DeserializeError, Deserialize, Serialize};

pub mod msg;

// pub struct RecvMsg<'de, 'recv, Reader: Unpin + ?Sized, Msg: Deserialize<'de>> {
//     reader: &'recv mut Reader,
//     buf: &'de mut [u8],
//     msg: PhantomData<&'de Msg>,
// }
//
// impl<'de, 'recv, R: Unpin + ?Sized, Msg: Deserialize<'de>> RecvMsg<'de, 'recv, R, Msg> {
//     fn new(reader: &'recv mut R, buf: &'de mut [u8]) -> Self {
//         Self {
//             reader,
//             buf,
//             msg: PhantomData,
//         }
//     }
// }
//
// impl<'de, 'recv, R: Unpin + ?Sized, Msg: Deserialize<'de>> Unpin for RecvMsg<'de, 'recv, R, Msg> {}
//
// impl<'de, 'recv: 'de, R: AsyncRead + Unpin + ?Sized, Msg: Deserialize<'de>> Future
//     for RecvMsg<'de, 'recv, R, Msg>
// {
//     type Output = Result<Msg, syrup::Error<'static>>;
//
//     fn poll(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Self::Output> {
//         // let buf = self.get_mut().buf;
//         let recv = self.get_mut();
//         let amt = ready!(Pin::new(&mut *recv.reader).poll_read(cx, &mut *recv.buf)).unwrap();
//         match syrup::de::from_bytes::<Msg>(recv.buf) {
//             Ok(res) => Poll::Ready(Ok(res)),
//             Err(_) => todo!(),
//         }
//     }
// }

pub struct CapTpSessionCore<Socket> {
    pub(crate) socket: Mutex<Socket>,
    signing_key: SigningKey,

    des_queue: Mutex<Vec<u8>>,
    read_buf: Mutex<[u8; 1024]>,
}

impl<Socket: AsyncRead + AsyncWrite + Unpin> CapTpSessionCore<Socket> {
    pub async fn and_accept<HKey, HVal>(
        mut self,
        local_locator: NodeLocator<HKey, HVal>,
    ) -> Result<CapTpSession<Socket>, futures::io::Error>
    where
        NodeLocator<HKey, HVal>: Serialize,
        OpStartSession<HKey, HVal>: Serialize,
    {
        tracing::debug!(local = %local_locator.designator, "accepting OpStartSession");

        let remote_vkey = self.recv_start_session::<String, String>().await?;

        self.send_msg(&self.generate_start_msg(local_locator))
            .await?;

        Ok(CapTpSession::<Socket> {
            core: self,
            remote_vkey,
        })
    }

    pub async fn and_connect<HKey, HVal>(
        mut self,
        local_locator: NodeLocator<HKey, HVal>,
    ) -> Result<CapTpSession<Socket>, futures::io::Error>
    where
        NodeLocator<HKey, HVal>: Serialize,
        OpStartSession<HKey, HVal>: Serialize,
    {
        let local_designator = local_locator.designator.clone();
        tracing::debug!(local = %local_designator, "connecting with OpStartSession");
        self.send_msg(&self.generate_start_msg(local_locator))
            .await?;
        // self.socket.flush().await?;

        tracing::debug!(local = %local_designator, "sent OpStartSession, receiving response");

        let remote_vkey = self.recv_start_session::<String, String>().await?;

        Ok(CapTpSession::<Socket> {
            core: self,
            remote_vkey,
        })
    }

    fn generate_start_msg<HKey, HVal>(
        &self,
        local_locator: NodeLocator<HKey, HVal>,
    ) -> OpStartSession<HKey, HVal>
    where
        NodeLocator<HKey, HVal>: Serialize,
    {
        let location_sig = self
            .signing_key
            .sign(&syrup::ser::to_bytes(&local_locator).unwrap());
        OpStartSession::new(
            self.signing_key.verifying_key().into(),
            local_locator,
            location_sig.into(),
        )
    }

    async fn recv_start_session<HKey, HVal>(&self) -> Result<VerifyingKey, futures::io::Error>
    where
        HKey: Serialize,
        HVal: Serialize,
        for<'de> OpStartSession<HKey, HVal>: Deserialize<'de>,
    {
        let response = self.recv_msg::<OpStartSession<HKey, HVal>>().await?;

        if response.captp_version != CAPTP_VERSION {
            todo!()
        }

        if let Err(_) = response.verify_location() {
            todo!()
        }

        Ok(response.session_pubkey.ecc)
    }

    #[inline]
    pub(crate) async fn recv_msg<Msg>(&self) -> Result<Msg, futures::io::Error>
    where
        for<'de> Msg: Deserialize<'de>,
    {
        let mut socket = self.socket.lock().await;
        let mut des_queue = self.des_queue.lock().await;
        let mut read_buf = self.read_buf.lock().await;
        let (rem, res) = loop {
            match syrup::de::nom_bytes::<Msg>(&des_queue) {
                Ok(m) => break m,
                Err(e) => match e.needed() {
                    Some(_) => {
                        let amt = socket.read(&mut *read_buf).await?;
                        tracing::trace!(recv_amt = amt);
                        if amt == 0 {
                            todo!("handle recv 0 bytes")
                        }
                        des_queue.extend_from_slice(&read_buf[..amt]);
                    }
                    None => todo!(),
                },
            }
        };
        let queue_len = des_queue.len();
        let rem_len = rem.len();
        des_queue.drain(..(queue_len - rem_len));
        Ok(res)
    }

    #[inline]
    pub(crate) async fn send_msg<Msg: Serialize>(
        &mut self,
        msg: &Msg,
    ) -> Result<(), futures::io::Error> {
        let bytes = syrup::ser::to_bytes(msg).unwrap();
        tracing::trace!(len = bytes.len());
        let mut socket = self.socket.lock().await;
        socket.write_all(&bytes).await
    }
}

pub struct CapTpSession<Socket> {
    remote_vkey: VerifyingKey,
    core: CapTpSessionCore<Socket>,
}

impl<Socket: AsyncRead + AsyncWrite + Unpin> CapTpSession<Socket> {
    pub fn init(socket: Socket) -> CapTpSessionCore<Socket> {
        CapTpSessionCore {
            socket: Mutex::new(socket),
            signing_key: SigningKey::generate(&mut OsRng),
            des_queue: Mutex::new(Vec::new()),
            read_buf: Mutex::new([0; 1024]),
        }
    }

    #[inline]
    pub async fn recv_msg<Msg>(&self) -> Result<Msg, futures::io::Error>
    where
        for<'de> Msg: Deserialize<'de>,
    {
        self.core.recv_msg().await
    }

    #[inline]
    pub async fn send_msg<Msg: Serialize>(&mut self, msg: &Msg) -> Result<(), futures::io::Error> {
        self.core.send_msg(msg).await
    }

    #[inline]
    pub async fn abort(mut self, reason: impl Into<OpAbort>) -> Result<(), futures::io::Error> {
        self.send_msg(&reason.into()).await
    }

    #[inline]
    pub fn signing_key(&self) -> &SigningKey {
        &self.core.signing_key
    }

    #[inline]
    pub fn remote_vkey(&self) -> &VerifyingKey {
        &self.remote_vkey
    }
}
