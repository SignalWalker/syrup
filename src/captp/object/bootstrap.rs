use super::RemoteObject;
use crate::async_compat::AsyncWrite;
use crate::captp::CapTpDeliver;
use crate::captp::{
    msg::{DescHandoffReceive, DescImport},
    SendError,
};
use std::sync::Arc;
use syrup::Serialize;

pub struct RemoteBootstrap {
    base: RemoteObject,
}

impl RemoteBootstrap {
    pub(crate) fn new(session: Arc<dyn CapTpDeliver + Send + Sync + 'static>) -> Self {
        Self {
            base: RemoteObject::new(session, 0),
        }
    }
}

impl RemoteBootstrap {
    pub async fn fetch(
        &self,
        swiss_number: &[u8],
        // answer_pos: Option<u64>,
        // resolve_me_desc: DescImport,
    ) -> Result<impl std::future::Future<Output = Result<RemoteObject, syrup::Item>>, SendError>
    {
        use futures::FutureExt;
        let swiss_hash = crate::hash(&swiss_number);
        tracing::trace!(%swiss_hash, "fetching object");
        let session = self.base.session.clone();
        Ok(self
            .base
            .call_and("fetch", &[syrup::Bytes(swiss_number)])
            .await?
            .map(move |res| -> Result<RemoteObject, syrup::Item> {
                match res {
                    Ok(res) => {
                        let mut args = res?.into_iter();
                        match args.next() {
                            Some(i) => Ok(RemoteObject {
                                position: <u64 as syrup::FromSyrupItem>::from_syrup_item(&i)
                                    .map_err(Clone::clone)?,
                                session,
                            }),
                            None => todo!(),
                        }
                    }
                    Err(_) => todo!("canceled answer"),
                }
            }))
    }

    pub async fn deposit_gift(&self, gift_id: u64, desc: DescImport) -> Result<(), SendError> {
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
