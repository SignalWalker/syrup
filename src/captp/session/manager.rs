use super::{CapTpSession, CapTpSessionBuilder, CapTpSessionCore, CapTpSessionInternal};
use crate::locator::NodeLocator;
use ed25519_dalek::{SigningKey, VerifyingKey};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub struct CapTpSessionManager<Reader, Writer> {
    sessions: HashMap<String, CapTpSession<Reader, Writer>>,
    outgoing: HashMap<String, (SigningKey, VerifyingKey)>,
}

impl<Reader, Writer> std::fmt::Debug for CapTpSessionManager<Reader, Writer> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapTpSessionManager")
            .field("sessions", &self.sessions)
            .field("outgoing", &self.outgoing)
            .finish()
    }
}

impl<Reader, Writer> CapTpSessionManager<Reader, Writer> {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            outgoing: HashMap::new(),
        }
    }

    pub fn get(&self, designator: impl AsRef<str>) -> Option<&CapTpSession<Reader, Writer>> {
        self.sessions.get(designator.as_ref())
    }

    pub fn init_session<'manager>(
        &'manager mut self,
        reader: Reader,
        writer: Writer,
    ) -> CapTpSessionBuilder<'manager, Reader, Writer> {
        CapTpSessionBuilder::new(self, reader, writer)
    }

    pub(crate) fn finalize_session<HKey, HVal>(
        &mut self,
        core: CapTpSessionCore<Reader, Writer>,
        signing_key: SigningKey,
        remote_vkey: VerifyingKey,
        remote_loc: NodeLocator<HKey, HVal>,
        // registry: Arc<super::SwissRegistry<Socket>>,
    ) -> CapTpSession<Reader, Writer> {
        let internal = Arc::new(CapTpSessionInternal::new(
            core,
            signing_key,
            remote_vkey,
            // registry,
        ));
        let res = CapTpSession { base: internal };
        self.sessions.insert(remote_loc.designator, res.clone());
        res
    }
}
