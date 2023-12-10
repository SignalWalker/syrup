use super::Netlayer;

pub struct OnionNetlayer {
    client: TorClient,
}

impl OnionNetLayer {
    pub async fn new() -> Result<Self, arti_client::Error> {
        Self {
            client: TorClient::create_bootstrapped(TorClientConfig::default()).await?,
        }
    }
}

impl Netlayer for OnionNetlayer {
    fn connect() {
        todo!()
    }

    fn accept() {
        todo!()
    }
}
