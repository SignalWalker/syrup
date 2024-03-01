use super::{Answer, ObjectInbox, RemoteObject};
use crate::async_compat::AsyncWrite;
use crate::captp::{
    msg::{DescHandoffReceive, DescImport},
    SendError,
};
use std::{any::Any, sync::Arc};
use syrup::Serialize;

pub struct RemoteBootstrap<Reader, Writer> {
    base: RemoteObject<Reader, Writer>,
}

impl<Reader, Writer> RemoteBootstrap<Reader, Writer> {
    pub(crate) fn new(session: crate::captp::CapTpSession<Reader, Writer>) -> Self {
        Self {
            base: RemoteObject::new(session, 0),
        }
    }
}

impl<Reader, Writer> RemoteBootstrap<Reader, Writer> {
    pub async fn fetch(
        &self,
        swiss_number: &[u8],
        // answer_pos: Option<u64>,
        // resolve_me_desc: DescImport,
    ) -> Result<impl std::future::Future<Output = Result<u64, syrup::Item>>, SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        use futures::FutureExt;
        let swiss_hash = crate::hash(&swiss_number);
        tracing::trace!(%swiss_hash, "fetching object");
        Ok(self
            .base
            .call_and("fetch", &[syrup::Bytes(swiss_number)])
            .await?
            .map(|res| match res {
                Ok(res) => res.map(|args| {
                    let mut args = args.into_iter();
                    match args.next() {
                        Some(i) => match <u64 as syrup::FromSyrupItem>::from_syrup_item(i) {
                            Some(pos) => pos,
                            None => todo!(),
                        },
                        None => todo!(),
                    }
                }),
                Err(_) => todo!("canceled answer"),
            }))
    }

    pub async fn deposit_gift(&self, gift_id: u64, desc: DescImport) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
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
    ) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
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

pub struct LocalBootstrap {
    inbox: ObjectInbox,
}

impl LocalBootstrap {}
