use super::CapTpSession;
use crate::{
    async_compat::{AsyncRead, AsyncWrite},
    captp::{msg::OpStartSession, session::CapTpSessionCore, session::CapTpSessionManager},
    locator::NodeLocator,
    CAPTP_VERSION,
};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use std::future::Future;
use syrup::{Deserialize, Serialize};

pub struct CapTpSessionBuilder<'manager, Reader, Writer> {
    manager: &'manager mut CapTpSessionManager<Reader, Writer>,
    reader: Reader,
    writer: Writer,
    signing_key: SigningKey,
    // registry: Option<Arc<super::SwissRegistry<Socket>>>,
}

impl<'m, Reader, Writer> CapTpSessionBuilder<'m, Reader, Writer> {
    pub fn new(
        manager: &'m mut CapTpSessionManager<Reader, Writer>,
        reader: Reader,
        writer: Writer,
    ) -> Self {
        Self {
            manager,
            reader,
            writer,
            signing_key: SigningKey::generate(&mut OsRng),
            // registry: None,
        }
    }

    // pub fn with_registry(mut self, registry: Option<Arc<super::SwissRegistry<Socket>>>) -> Self {
    //     self.registry = registry;
    //     self
    // }

    pub fn and_accept(
        self,
        local_locator: NodeLocator,
    ) -> impl Future<Output = Result<CapTpSession<Reader, Writer>, std::io::Error>> + 'm
    where
        Reader: AsyncRead + Unpin,
        Writer: AsyncWrite + Unpin,
    {
        tracing::debug!(local = %local_locator.designator, "accepting OpStartSession");

        let start_msg = self.generate_start_msg(local_locator);
        let core = CapTpSessionCore::new(self.reader, self.writer);

        async move {
            let (remote_vkey, remote_loc) = Self::recv_start_session(&core).await?;

            core.send_msg(&start_msg).await?;
            core.flush().await?;

            Ok(self
                .manager
                .finalize_session(core, self.signing_key, remote_vkey, remote_loc))
        }
    }

    pub fn and_connect(
        self,
        local_locator: NodeLocator,
    ) -> impl Future<Output = Result<CapTpSession<Reader, Writer>, std::io::Error>> + 'm
    where
        Reader: AsyncRead + Unpin,
        Writer: AsyncWrite + Unpin,
        NodeLocator: Serialize,
        OpStartSession: Serialize + 'm,
        for<'de> NodeLocator: Deserialize<'de>,
        for<'de> OpStartSession: Deserialize<'de>,
    {
        let local_designator = local_locator.designator.clone();
        tracing::debug!(local = %local_designator, "connecting with OpStartSession");

        let start_msg = self.generate_start_msg(local_locator);
        let core = CapTpSessionCore::new(self.reader, self.writer);

        async move {
            core.send_msg(&start_msg).await?;
            core.flush().await?;

            tracing::debug!(local = %local_designator, "sent OpStartSession, receiving response");

            let (remote_vkey, remote_loc) = Self::recv_start_session(&core).await?;

            Ok(self
                .manager
                .finalize_session(core, self.signing_key, remote_vkey, remote_loc))
        }
    }

    fn generate_start_msg(&self, local_locator: NodeLocator) -> OpStartSession
    where
        NodeLocator: Serialize,
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

    pub(super) async fn recv_start_session(
        core: &CapTpSessionCore<Reader, Writer>,
    ) -> Result<(VerifyingKey, NodeLocator), std::io::Error>
    where
        Reader: AsyncRead + Unpin,
    {
        let mut resp_buf = [0u8; 1024];
        let response = core.recv_msg::<OpStartSession>(&mut resp_buf).await?;

        if response.captp_version != CAPTP_VERSION {
            todo!("handle mismatched captp versions")
        }

        if response.verify_location().is_err() {
            todo!()
        }

        Ok((response.session_pubkey.ecc, response.acceptable_location))
    }
}
