use super::{DeliverError, DeliverOnlyError, RemoteObject};
use crate::captp::msg::DescExport;
use crate::captp::msg::{DescHandoffReceive, DescImport};
use crate::captp::CapTpDeliver;
use std::future::Future;
use std::sync::Arc;
use syrup::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error(transparent)]
    Deliver(#[from] DeliverError),
    #[error("fetch returned nothing")]
    MissingResult,
    #[error("otherwise successful fetch returned something other than an export position: {0:?}")]
    UnexpectedArgument(syrup::Item),
}

pub trait Fetch: Sized {
    type Swiss<'swiss>;
    fn fetch<'swiss>(
        bootstrap: &RemoteBootstrap,
        swiss: Self::Swiss<'swiss>,
    ) -> impl Future<Output = Result<Self, FetchError>> + Send;
}

pub struct RemoteBootstrap {
    base: RemoteObject,
}

impl RemoteBootstrap {
    pub(crate) fn new(session: Arc<dyn CapTpDeliver + Send + Sync + 'static>) -> Self {
        Self {
            base: RemoteObject::new(session, 0.into()),
        }
    }
}

impl RemoteBootstrap {
    pub async fn fetch(&self, swiss_number: &[u8]) -> Result<RemoteObject, FetchError> {
        let mut args = self
            .base
            .call_and("fetch", &[syrup::Bytes(swiss_number)])
            .await?;
        let session = self.base.session.clone();
        match args.pop() {
            Some(i) => Ok(RemoteObject {
                position: <DescExport as syrup::FromSyrupItem>::from_syrup_item(&i)
                    .map_err(|_| FetchError::UnexpectedArgument(i))?,
                session,
            }),
            None => Err(FetchError::MissingResult),
        }
    }

    pub async fn fetch_to(
        &self,
        swiss_number: &[u8],
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), DeliverError> {
        tracing::trace!(
            swiss_hash = crate::hash(&swiss_number),
            answer_pos,
            ?resolve_me_desc,
            "fetching object"
        );
        self.base
            .call(
                "fetch",
                &[syrup::Bytes(swiss_number)],
                answer_pos,
                resolve_me_desc,
            )
            .await
            .map_err(From::from)
    }

    pub async fn fetch_with<'swiss, Obj: Fetch>(
        &self,
        swiss: Obj::Swiss<'swiss>,
    ) -> Result<Obj, FetchError> {
        Obj::fetch(self, swiss).await
    }

    pub async fn deposit_gift(
        &self,
        gift_id: u64,
        desc: DescImport,
    ) -> Result<(), DeliverOnlyError> {
        self.base
            .call_only("deposit_gift", &syrup::raw_syrup![&gift_id, &desc])
            .await
    }

    pub async fn withdraw_gift<HKey, HVal>(
        self: Arc<Self>,
        handoff_receive: DescHandoffReceive<HKey, HVal>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), DeliverError>
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
