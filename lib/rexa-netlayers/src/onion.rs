use arti_client::{DataReader, DataWriter, TorClient, TorClientConfig};
use futures::{lock::Mutex, stream::BoxStream, StreamExt};
use rexa::{
    captp::{CapTpSession, CapTpSessionManager},
    locator::NodeLocator,
    netlayer::Netlayer,
};
use std::future::Future;
use std::sync::Arc;
use tor_cell::relaycell::msg::Connected;
use tor_hsservice::{OnionServiceConfig, RunningOnionService, StreamRequest};
use tor_rtcompat::Runtime;

#[cfg(feature = "tokio")]
use tokio::sync::RwLock;

#[repr(transparent)]
struct TorLocator<'l>(&'l NodeLocator);

impl<'l> From<&'l NodeLocator> for TorLocator<'l> {
    fn from(value: &'l NodeLocator) -> Self {
        Self(value)
    }
}

impl<'l> AsRef<NodeLocator> for TorLocator<'l> {
    fn as_ref(&self) -> &NodeLocator {
        self.0
    }
}

impl<'l> arti_client::IntoTorAddr for TorLocator<'l> {
    fn into_tor_addr(self) -> Result<arti_client::TorAddr, arti_client::TorAddrError> {
        format!("{}.onion", self.0.designator)
            .as_str()
            .into_tor_addr()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Tor(#[from] arti_client::Error),
    #[error(transparent)]
    Client(#[from] tor_hsservice::ClientError),
    #[error("session manager lock poisoned")]
    LockPoisoned,
}

impl<Guard> From<std::sync::PoisonError<Guard>> for Error {
    fn from(_: std::sync::PoisonError<Guard>) -> Self {
        Self::LockPoisoned
    }
}

pub struct OnionNetlayer<AsyncRuntime: Runtime> {
    service: Arc<RunningOnionService>,
    req_stream: Mutex<BoxStream<'static, StreamRequest>>,
    client: TorClient<AsyncRuntime>,
    manager: RwLock<CapTpSessionManager<<Self as Netlayer>::Reader, <Self as Netlayer>::Writer>>,
}

impl<Rt: Runtime> std::fmt::Debug for OnionNetlayer<Rt> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnionNetlayer")
            .field("locator", &self.locators().pop().unwrap())
            .finish_non_exhaustive()
    }
}

impl<Rt: Runtime> OnionNetlayer<Rt> {
    pub fn new(
        client: TorClient<Rt>,
        service_config: OnionServiceConfig,
    ) -> Result<Self, arti_client::Error> {
        let (service, stream) = client.launch_onion_service(service_config)?;
        Ok(Self {
            service,
            req_stream: tor_hsservice::handle_rend_requests(stream).boxed().into(),
            client,
            manager: RwLock::new(CapTpSessionManager::new()),
        })
    }

    pub fn service(&self) -> &RunningOnionService {
        &self.service
    }

    pub async fn new_bootstrapped(
        runtime: Rt,
        client_config: TorClientConfig,
        service_config: OnionServiceConfig,
    ) -> Result<Self, arti_client::Error> {
        let client = TorClient::with_runtime(runtime)
            .config(client_config)
            .create_bootstrapped()
            .await?;
        Self::new(client, service_config)
    }
}

impl<R: Runtime> Netlayer for OnionNetlayer<R> {
    type Reader = DataReader;
    type Writer = DataWriter;
    type Error = Error;

    #[inline]
    fn connect(
        &self,
        locator: &NodeLocator,
    ) -> impl Future<Output = Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error>> {
        let local_locator = self.locators().pop().unwrap();
        async move {
            let (reader, writer) = self
                .client
                .connect(TorLocator::from(locator))
                .await?
                .split();
            self.manager
                .write()
                .await
                .init_session(reader, writer)
                .and_connect(local_locator)
                .await
                .map_err(From::from)
        }
    }

    async fn accept(&self) -> Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error> {
        let (reader, writer) = self
            .req_stream
            .lock()
            .await
            .next()
            .await
            .expect("req_stream should always return Some(..)")
            .accept(Connected::new_empty())
            .await?
            .split();

        self.manager
            .write()
            .await
            .init_session(reader, writer)
            .and_accept(self.locators().pop().unwrap())
            .await
            .map_err(From::from)
    }

    fn locators(&self) -> Vec<NodeLocator> {
        // HACK :: there's probably a better way to do this
        let mut name = self
            .service
            .onion_name()
            .expect("OnionNetlayer should know own onion service name")
            .to_string();
        name.truncate(name.len() - ".onion".len());
        vec![NodeLocator::new(name, "onion".to_string())]
    }
}
