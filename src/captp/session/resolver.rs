use std::sync::Arc;

use super::{CapTpDeliver, SendError};
use crate::captp::msg::{DescImport, DescImportObject};
use syrup::Serialize;

#[must_use]
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

impl std::clone::Clone for GenericResolver {
    fn clone(&self) -> Self {
        Self {
            session: self.session.clone(),
            answer_pos: self.answer_pos.clone(),
            resolve_me_desc: self.resolve_me_desc.clone(),
            #[cfg(feature = "extra-diagnostics")]
            resolved: self.resolved,
        }
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

    fn position(&self) -> u64 {
        use crate::captp::msg::DescImportPromise;
        match self.resolve_me_desc {
            DescImport::Object(DescImportObject { position })
            | DescImport::Promise(DescImportPromise { position }) => position,
        }
    }

    pub async fn fulfill<'f, 'arg, Arg: Serialize + 'arg>(
        mut self,
        args: impl IntoIterator<Item = &'arg Arg>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), SendError> {
        let args = syrup::raw_syrup_unwrap![&syrup::Symbol("fulfill"); args];
        #[cfg(feature = "extra-diagnostics")]
        {
            self.resolved = true;
        }
        self.session
            .deliver(self.position(), args, answer_pos, resolve_me_desc)
            .await
    }

    pub async fn break_promise<'f>(mut self, error: impl Serialize) -> Result<(), SendError> {
        let args = syrup::raw_syrup_unwrap![&syrup::Symbol("break"), &error];
        #[cfg(feature = "extra-diagnostics")]
        {
            self.resolved = true;
        }
        self.session.deliver_only(self.position(), args).await
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
    pub async fn fulfill(self, position: u64) -> Result<(), SendError> {
        self.base
            .fulfill([&position], None, DescImportObject::from(0).into())
            .await
    }

    pub async fn break_promise(self, error: impl Serialize) -> Result<(), SendError> {
        self.base.break_promise(error).await
    }
}

impl From<GenericResolver> for FetchResolver {
    fn from(base: GenericResolver) -> Self {
        Self { base }
    }
}
