use std::sync::Arc;

use syrup::{RawSyrup, Serialize};

use super::CapTpDeliver;
use crate::captp::{
    msg::{DescExport, DescImport, DescImportObject},
    object::{DeliverError, DeliverOnlyError},
};

#[must_use]
#[derive(Clone)]
pub struct GenericResolver {
    session: std::sync::Arc<dyn CapTpDeliver + Send + Sync>,
    answer_pos: Option<u64>,
    resolve_me_desc: DescImport,
    #[cfg(feature = "extra-diagnostics")]
    resolved: bool,
}

#[cfg(feature = "extra-diagnostics")]
impl Drop for GenericResolver {
    fn drop(&mut self) {
        if !self.resolved {
            tracing::warn!(resolver = ?self, "dropping unresolved resolver");
        }
    }
}

impl std::fmt::Debug for GenericResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericResolver")
            // .field("session", &self.session)
            .field("answer_pos", &self.answer_pos)
            .field("resolve_me_desc", &self.resolve_me_desc)
            .finish()
    }
}

impl GenericResolver {
    pub(super) fn new(
        session: Arc<dyn CapTpDeliver + Send + Sync>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Self {
        Self {
            session,
            answer_pos,
            resolve_me_desc,
            #[cfg(feature = "extra-diagnostics")]
            resolved: false,
        }
    }

    fn position(&self) -> DescExport {
        use crate::captp::msg::DescImportPromise;
        match self.resolve_me_desc {
            DescImport::Object(DescImportObject { position })
            | DescImport::Promise(DescImportPromise { position }) => position.into(),
        }
    }

    pub async fn fulfill<'arg, Arg: Serialize + ?Sized + 'arg>(
        mut self,
        args: impl IntoIterator<Item = &'arg Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), DeliverError> {
        #[cfg(feature = "extra-diagnostics")]
        {
            self.resolved = true;
        }
        self.session
            .deliver(
                self.position(),
                &RawSyrup::vec_from_ident_iter("fulfill", args.into_iter())?,
                answer_pos,
                resolve_me_desc,
            )
            .await
            .map_err(From::from)
    }

    pub async fn fulfill_and<'arg, Arg: Serialize + ?Sized + 'arg>(
        mut self,
        args: impl IntoIterator<Item = &'arg Arg>,
    ) -> Result<Vec<syrup::Item>, DeliverError> {
        #[cfg(feature = "extra-diagnostics")]
        {
            self.resolved = true;
        }
        self.session
            .deliver_and(
                self.position(),
                &RawSyrup::vec_from_ident_iter("fulfill", args.into_iter())?,
            )
            .await
    }

    pub async fn break_promise<'f>(
        mut self,
        error: &(impl Serialize + ?Sized),
    ) -> Result<(), DeliverOnlyError> {
        #[cfg(feature = "extra-diagnostics")]
        {
            self.resolved = true;
        }
        self.session
            .deliver_only(
                self.position(),
                &[
                    RawSyrup::try_from_serialize("break")?,
                    RawSyrup::try_from_serialize(error)?,
                ],
            )
            .await
            .map_err(From::from)
    }
}

#[must_use]
pub struct FetchResolver {
    base: GenericResolver,
}

impl std::fmt::Debug for FetchResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FetchResolver")
            .field("base", &self.base)
            .finish()
    }
}

impl std::clone::Clone for FetchResolver {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
        }
    }
}

impl FetchResolver {
    pub async fn fulfill(
        self,
        position: DescExport,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), DeliverError> {
        self.base
            .fulfill([&position], answer_pos, resolve_me_desc)
            .await
    }

    pub async fn fulfill_and(self, position: DescExport) -> Result<Vec<syrup::Item>, DeliverError> {
        self.base.fulfill_and(&[position]).await
    }

    pub async fn break_promise(
        self,
        error: &(impl Serialize + ?Sized),
    ) -> Result<(), DeliverOnlyError> {
        self.base.break_promise(error).await
    }
}

impl From<GenericResolver> for FetchResolver {
    fn from(base: GenericResolver) -> Self {
        Self { base }
    }
}
