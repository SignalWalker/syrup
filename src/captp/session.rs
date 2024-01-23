use super::msg::OpAbort;
use ed25519_dalek::{SigningKey, VerifyingKey};
use futures::{AsyncRead, AsyncWrite};
use syrup::Serialize;

mod builder;
pub use builder::*;

mod core;
pub use core::*;

mod message_queue;
pub use message_queue::*;

pub struct CapTpSession<Socket> {
    core: CapTpSessionCore<Socket>,
    signing_key: SigningKey,
    remote_vkey: VerifyingKey,
}

impl<Socket: AsyncRead + AsyncWrite + Unpin> CapTpSession<Socket> {
    pub fn init(socket: Socket) -> CapTpSessionBuilder<Socket> {
        CapTpSessionBuilder::new(socket)
    }

    // #[inline]
    // pub async fn recv_msg<Msg>(&self) -> Result<Msg, futures::io::Error>
    // where
    //     for<'de> Msg: Deserialize<'de>,
    // {
    //     self.core.recv_msg().await
    // }

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
        &self.signing_key
    }

    #[inline]
    pub fn remote_vkey(&self) -> &VerifyingKey {
        &self.remote_vkey
    }
}
