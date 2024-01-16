use arti_client::{DataStream, IntoTorAddr, TorClient, TorClientConfig};
use tor_rtcompat::Runtime;

use crate::{captp::CapTpSession, locator::NodeLocator};

use super::Netlayer;

#[derive(Clone)]
pub struct OnionNetlayer<AsyncRuntime: Runtime> {
    client: TorClient<AsyncRuntime>,
}

impl<R: Runtime> OnionNetlayer<R> {
    pub fn new(client: TorClient<R>) -> Self {
        Self { client }
    }

    pub async fn new_bootstrapped(runtime: R) -> Result<Self, arti_client::Error> {
        Ok(Self {
            client: TorClient::with_runtime(runtime)
                .config(TorClientConfig::default())
                .create_bootstrapped()
                .await?,
        })
    }

    #[inline]
    pub async fn connect<A: IntoTorAddr>(&self, target: A) -> arti_client::Result<DataStream> {
        self.client.connect(target).await
    }
}

impl<R: Runtime> Netlayer<DataStream> for OnionNetlayer<R> {
    type Error = arti_client::Error;

    #[inline]
    async fn connect<HintKey: PartialEq + Eq + std::hash::Hash, HintValue>(
        &self,
        locator: &NodeLocator<HintKey, HintValue>,
    ) -> Result<CapTpSession<DataStream>, Self::Error> {
        let socket = self.connect(locator).await?;
        Ok(CapTpSession::start_with(socket).await)
    }

    fn accept() {
        todo!()
    }
}
