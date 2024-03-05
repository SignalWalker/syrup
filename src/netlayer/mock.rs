use tokio::io::DuplexStream;

use crate::{
    async_compat::{
        mpsc, oneshot, AsyncRead, AsyncWrite, Mutex as AsyncMutex, RwLock as AsyncRwLock,
    },
    captp::{CapTpSession, CapTpSessionManager},
    locator::NodeLocator,
};
use std::{
    collections::HashMap,
    future::Future,
    io::Write,
    pin::pin,
    sync::{Arc, Mutex, PoisonError, RwLock, Weak},
    task::{ready, Poll},
};

use super::Netlayer;

type MockReader = <Arc<MockNetlayer> as Netlayer>::Reader;
type MockWriter = <Arc<MockNetlayer> as Netlayer>::Writer;
type MockSession = CapTpSession<MockReader, MockWriter>;
type StreamSend = oneshot::Sender<(MockReader, MockWriter)>;
type StreamRecv = oneshot::Receiver<(MockReader, MockWriter)>;

lazy_static::lazy_static! {
    static ref MOCK_REGISTRY: RwLock<HashMap<String, (Weak<MockNetlayer>, mpsc::UnboundedSender<StreamSend>)>> = RwLock::default();
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("name already in use")]
    NameInUse,
    #[error("address not found")]
    NotFound,
    #[error("MockNetlayer registry poisoned")]
    RegistryPoisoned,
    #[error("pipe broken during accept")]
    Accept,
    #[error(transparent)]
    Connect(#[from] oneshot::error::RecvError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl<Guard> From<PoisonError<Guard>> for Error {
    fn from(_: PoisonError<Guard>) -> Self {
        Self::RegistryPoisoned
    }
}

pub struct MockNetlayer {
    name: String,
    connect_recv: AsyncMutex<mpsc::UnboundedReceiver<StreamSend>>,
    manager: AsyncRwLock<CapTpSessionManager<MockReader, MockWriter>>,
}

impl MockNetlayer {
    pub fn bind(name: String) -> Result<Arc<Self>, Error> {
        let mut reg = MOCK_REGISTRY.write()?;
        if let Some(res) = reg.get(&name).and_then(|(p, _)| Weak::upgrade(p)) {
            Ok(res)
        } else {
            let (connect_send, connect_recv) = mpsc::unbounded_channel();
            let res = Arc::new(Self {
                name: name.clone(),
                connect_recv: AsyncMutex::new(connect_recv),
                manager: AsyncRwLock::new(CapTpSessionManager::new()),
            });
            reg.insert(name, (Arc::downgrade(&res), connect_send));
            Ok(res)
        }
    }

    pub fn close(self) -> Result<(), Error> {
        MOCK_REGISTRY.write()?.remove(&self.name);
        Ok(())
    }
}

impl Netlayer for Arc<MockNetlayer> {
    type Reader = DuplexStream;
    type Writer = DuplexStream;
    type Error = Error;

    fn connect<HintKey: syrup::Serialize, HintValue: syrup::Serialize>(
        &self,
        locator: &crate::locator::NodeLocator<HintKey, HintValue>,
    ) -> impl Future<
        Output = Result<crate::captp::CapTpSession<Self::Reader, Self::Writer>, Self::Error>,
    > + Send
    where
        crate::locator::NodeLocator<HintKey, HintValue>: Sync,
    {
        let remote_name = &locator.designator;
        async move {
            if let Some(session) = self.manager.read().await.get(remote_name) {
                return Ok(session.clone());
            }

            let (stream_send, stream_recv) = oneshot::channel();
            if let Err(_) = MOCK_REGISTRY
                .read()?
                .get(&locator.designator)
                .ok_or(Error::NotFound)?
                .1
                .send(stream_send)
            {
                // send failed, clean registry
                MOCK_REGISTRY.write()?.remove(&locator.designator);
                return Err(Error::NotFound);
            }

            let (reader, writer) = stream_recv.await.map_err(Error::from)?;
            self.manager
                .write()
                .await
                .init_session(reader, writer)
                .and_connect(self.locator::<String, String>())
                .await
                .map_err(From::from)
        }
    }

    fn accept(
        &self,
    ) -> impl Future<
        Output = Result<crate::captp::CapTpSession<Self::Reader, Self::Writer>, Self::Error>,
    > + Send {
        async move {
            let stream_send = self.connect_recv.lock().await.recv().await.unwrap();
            let (reader, writer) = {
                // HACK :: there's probably a better way to set this number but whatever
                let (local_reader, remote_writer) = tokio::io::duplex(1024);
                let (remote_reader, local_writer) = tokio::io::duplex(1024);
                stream_send
                    .send((remote_reader, remote_writer))
                    .map_err(|_| Error::Accept)?;
                (local_reader, local_writer)
            };
            self.manager
                .write()
                .await
                .init_session(reader, writer)
                .and_connect(self.locator::<String, String>())
                .await
                .map_err(From::from)
        }
    }

    fn locator<HintKey, HintValue>(&self) -> crate::locator::NodeLocator<HintKey, HintValue> {
        NodeLocator::new(self.name.clone(), "mock".to_owned())
    }
}
