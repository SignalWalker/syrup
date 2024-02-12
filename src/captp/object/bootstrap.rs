use super::{Answer, RemoteObject};
use crate::captp::msg::{DescHandoffReceive, DescImport};
use futures::AsyncWrite;
use std::{any::Any, sync::Arc};
use syrup::Serialize;

pub struct RemoteBootstrap<Socket> {
    base: RemoteObject<Socket>,
}

impl<Socket> RemoteBootstrap<Socket> {
    pub(crate) fn new(session: crate::captp::CapTpSession<Socket>) -> Self {
        Self {
            base: RemoteObject::new(session, 0),
        }
    }
}

impl<Socket> RemoteBootstrap<Socket> {
    pub async fn fetch(
        &self,
        swiss_number: &[u8],
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        self.base
            .call(
                "fetch",
                &[syrup::Bytes(swiss_number)],
                answer_pos,
                resolve_me_desc,
            )
            .await
    }

    pub async fn deposit_gift(
        &self,
        gift_id: u64,
        desc: DescImport,
    ) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        self.base
            .call_only("deposit_gift", &syrup::raw_syrup_unwrap![&gift_id, &desc])
            .await
    }

    pub async fn withdraw_gift<HKey, HVal>(
        self: Arc<Self>,
        handoff_receive: DescHandoffReceive<HKey, HVal>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
        DescHandoffReceive<HKey, HVal>: Serialize,
    {
        self.base
            .call(
                "withdraw_gift",
                &[handoff_receive],
                answer_pos,
                resolve_me_desc,
            )
            .await
    }
}
