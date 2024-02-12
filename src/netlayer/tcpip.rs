use super::Netlayer;
use crate::{
    captp::{CapTpSession, CapTpSessionManager},
    locator::NodeLocator,
};
use smol::{
    lock::RwLock,
    net::{AsyncToSocketAddrs, SocketAddr, TcpListener, TcpStream},
};
use syrup::Serialize;

#[derive(Debug)]
pub struct TcpIpNetlayer {
    listener: TcpListener,
    manager: RwLock<CapTpSessionManager<TcpStream>>,
}

impl Netlayer<TcpStream> for TcpIpNetlayer {
    type Error = smol::io::Error;

    async fn connect<HintKey: Serialize, HintValue: Serialize>(
        &self,
        locator: NodeLocator<HintKey, HintValue>,
    ) -> Result<CapTpSession<TcpStream>, Self::Error> {
        if let Some(session) = self.manager.read().await.get(&locator.designator) {
            return Ok(session);
        }

        let addr = locator.designator.parse::<SocketAddr>();
        tracing::debug!(
            local = ?self.local_addr()?,
            remote = %locator.designator,
            remote_addr = ?addr,
            "starting connection"
        );
        self.manager
            .write()
            .await
            .init_session(TcpStream::connect(addr.unwrap()).await?)
            .and_connect(self.locator::<String, String>()?)
            .await
    }

    async fn accept(&self) -> Result<CapTpSession<TcpStream>, Self::Error> {
        tracing::debug!(
            local = ?self.local_addr()?,
            "accepting connection"
        );

        self.manager
            .write()
            .await
            .init_session(self.listener.accept().await?.0)
            .and_accept(self.locator::<String, String>()?)
            .await
    }
}

impl TcpIpNetlayer {
    pub async fn bind(addr: impl AsyncToSocketAddrs) -> Result<Self, futures::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self {
            listener,
            manager: RwLock::new(CapTpSessionManager::new()),
        })
    }

    #[inline]
    pub fn local_addr(&self) -> Result<smol::net::SocketAddr, futures::io::Error> {
        self.listener.local_addr()
    }

    #[inline]
    pub fn locator<HKey, HVal>(&self) -> Result<NodeLocator<HKey, HVal>, futures::io::Error> {
        Ok(NodeLocator::new(
            self.local_addr()?.to_string(),
            "tcpip".to_owned(),
        ))
    }
}
