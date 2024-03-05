use common::{
    netlayers::{self as nl, BoxError, NlFuture},
    LogFormat,
};
use rexa::netlayer::Netlayer;

mod common;

#[cfg(feature = "netlayer-mock")]
test_nl!(nl::make_mock_netlayer => {
    fetch: fetch_mock
});

fn fetch<Nl: Netlayer, F: NlFuture<Nl>>(
    make_nl: impl Fn(&'static str, usize) -> F,
) -> Result<(), BoxError>
where
    Nl: Send + 'static,
    Nl::Reader: Send,
    Nl::Writer: Send,
    Nl::Error: std::error::Error + Send + Sync + 'static,
{
    let rt = common::initialize(LogFormat::Pretty)?;

    rt.block_on(async move {
        let node_a = make_nl("fetch", 0).await?;
        let node_b = make_nl("fetch", 1).await?;

        let (session_ab, session_ba) = common::connect_nodes(node_a, node_b).await?;

        todo!();

        Ok(())
    })
}
