use std::{collections::HashMap, sync::Arc};

use ed25519_dalek::{SigningKey, VerifyingKey};

use crate::locator::NodeLocator;

use super::{CapTpSession, CapTpSessionBuilder, CapTpSessionCore, CapTpSessionInternal};

#[derive(Debug, Clone)]
pub struct CapTpSessionManager<Socket> {
    sessions: HashMap<String, Arc<CapTpSessionInternal<Socket>>>,
    outgoing: HashMap<String, (SigningKey, VerifyingKey)>,
}

impl<Socket> CapTpSessionManager<Socket> {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            outgoing: HashMap::new(),
        }
    }

    pub fn get(&self, designator: impl AsRef<str>) -> Option<CapTpSession<Socket>> {
        self.sessions
            .get(designator.as_ref())
            .map(CapTpSession::from)
    }

    pub fn init_session<'manager>(
        &'manager mut self,
        socket: Socket,
    ) -> CapTpSessionBuilder<'manager, Socket> {
        CapTpSessionBuilder::new(self, socket)
    }

    pub(crate) fn finalize_session<HKey, HVal>(
        &mut self,
        core: CapTpSessionCore<Socket>,
        signing_key: SigningKey,
        remote_vkey: VerifyingKey,
        remote_loc: NodeLocator<HKey, HVal>,
    ) -> CapTpSession<Socket> {
        let internal = Arc::new(CapTpSessionInternal::new(core, signing_key, remote_vkey));
        self.sessions
            .insert(remote_loc.designator, internal.clone());
        CapTpSession { base: internal }
    }
}
