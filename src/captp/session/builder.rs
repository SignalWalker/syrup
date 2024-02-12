use super::CapTpSession;
use crate::{
    captp::{msg::OpStartSession, session::CapTpSessionManager, CapTpSessionCore},
    locator::NodeLocator,
    CAPTP_VERSION,
};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use futures::{AsyncRead, AsyncWrite};
use rand::rngs::OsRng;
use syrup::{Deserialize, Serialize};

pub struct CapTpSessionBuilder<'manager, Socket> {
    manager: &'manager mut CapTpSessionManager<Socket>,
    socket: Socket,
    signing_key: SigningKey,
}

impl<'m, Socket> CapTpSessionBuilder<'m, Socket> {
    pub fn new(manager: &'m mut CapTpSessionManager<Socket>, socket: Socket) -> Self {
        Self {
            manager,
            socket,
            signing_key: SigningKey::generate(&mut OsRng),
        }
    }

    pub async fn and_accept<HKey, HVal>(
        self,
        local_locator: NodeLocator<HKey, HVal>,
    ) -> Result<CapTpSession<Socket>, futures::io::Error>
    where
        Socket: AsyncRead + AsyncWrite + Unpin,
        NodeLocator<HKey, HVal>: Serialize,
        OpStartSession<HKey, HVal>: Serialize,
    {
        tracing::debug!(local = %local_locator.designator, "accepting OpStartSession");

        let start_msg = self.generate_start_msg(local_locator);
        let mut core = CapTpSessionCore {
            socket: self.socket,
        };

        let (remote_vkey, remote_loc) =
            Self::recv_start_session::<String, String>(&mut core).await?;

        core.send_msg(&start_msg).await?;
        core.flush().await?;

        Ok(self
            .manager
            .finalize_session(core, self.signing_key, remote_vkey, remote_loc))
    }

    pub async fn and_connect<HKey, HVal>(
        self,
        local_locator: NodeLocator<HKey, HVal>,
    ) -> Result<CapTpSession<Socket>, futures::io::Error>
    where
        Socket: AsyncRead + AsyncWrite + Unpin,
        NodeLocator<HKey, HVal>: Serialize,
        OpStartSession<HKey, HVal>: Serialize,
    {
        let local_designator = local_locator.designator.clone();
        tracing::debug!(local = %local_designator, "connecting with OpStartSession");

        let start_msg = self.generate_start_msg(local_locator);
        let mut core = CapTpSessionCore {
            socket: self.socket,
        };

        core.send_msg(&start_msg).await?;
        core.flush().await?;

        tracing::debug!(local = %local_designator, "sent OpStartSession, receiving response");

        let (remote_vkey, remote_loc) =
            Self::recv_start_session::<String, String>(&mut core).await?;

        Ok(self
            .manager
            .finalize_session(core, self.signing_key, remote_vkey, remote_loc))
    }

    fn generate_start_msg<HKey, HVal>(
        &self,
        local_locator: NodeLocator<HKey, HVal>,
    ) -> OpStartSession<HKey, HVal>
    where
        NodeLocator<HKey, HVal>: Serialize,
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

    pub(crate) async fn recv_start_session<HKey, HVal>(
        core: &mut CapTpSessionCore<Socket>,
    ) -> Result<(VerifyingKey, NodeLocator<HKey, HVal>), futures::io::Error>
    where
        Socket: AsyncRead + Unpin,
        HKey: Serialize,
        HVal: Serialize,
        for<'de> OpStartSession<HKey, HVal>: Deserialize<'de>,
    {
        let mut resp_buf = [0u8; 1024];
        let response = core
            .recv_msg::<OpStartSession<HKey, HVal>>(&mut resp_buf)
            .await?;

        if response.captp_version != CAPTP_VERSION {
            todo!()
        }

        if let Err(_) = response.verify_location() {
            todo!()
        }

        Ok((response.session_pubkey.ecc, response.acceptable_location))
    }
}
