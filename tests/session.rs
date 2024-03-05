use common::netlayers::BoxError;
use common::netlayers::{self as nl, NlFuture};
use common::LogFormat;
use rexa::{
    async_compat::{AsyncRead, AsyncWrite},
    captp::Event,
    netlayer::Netlayer,
};

mod common;

#[cfg(feature = "netlayer-mock")]
test_nl!(nl::make_mock_netlayer => {
    op_start: op_start_mock,
    op_abort: op_abort_mock
});

#[cfg(feature = "netlayer-datastream")]
test_nl!(nl::make_tcp_netlayer => {
    op_start: op_start_tcpip,
    op_abort: op_abort_tcpip
});

#[cfg(all(target_family = "unix", feature = "netlayer-datastream"))]
test_nl!(nl::make_unix_netlayer => {
    op_start: op_start_unix,
    op_abort: op_abort_unix
});

#[cfg(feature = "netlayer-onion")]
test_nl!(nl::make_onion_netlayer => {
    op_start: op_start_onion,
    op_abort: op_abort_onion
});

fn op_start<Nl: Netlayer, F: NlFuture<Nl>>(
    make_nl: impl Fn(&'static str, usize) -> F,
) -> Result<(), BoxError>
where
    Nl: Send + 'static,
    Nl::Reader: Send,
    Nl::Writer: Send,
    Nl::Error: std::error::Error + Send + Sync,
{
    let rt = common::initialize(LogFormat::Pretty)?;

    let (session_ab_key, session_ba_key) = match rt.block_on(async move {
        let node_a = make_nl("op_start", 0).await?;
        let node_b = make_nl("op_start", 1).await?;

        let (session_ab, session_ba) = common::connect_nodes(node_a, node_b).await?;

        Result::<_, BoxError>::Ok((
            session_ab.signing_key().verifying_key(),
            *session_ba.remote_vkey(),
        ))
    }) {
        Ok(res) => res,
        Err(error) => {
            tracing::error!(error, "failed");
            return Err(error);
        }
    };
    assert_eq!(session_ab_key, session_ba_key);
    Ok(())
}

fn op_abort<Nl: Netlayer, F: NlFuture<Nl>>(
    make_nl: impl Fn(&'static str, usize) -> F,
) -> Result<(), BoxError>
where
    Nl: Send + 'static,
    Nl::Reader: AsyncRead + Unpin + Send,
    Nl::Writer: AsyncWrite + Unpin + Send,
    Nl::Error: std::error::Error + Send + Sync,
{
    const ABORT_REASON: &'static str = "stage_0 test";

    match common::initialize(LogFormat::Pretty)?.block_on(async move {
        let node_a = make_nl("op_abort", 0).await?;
        let node_b = make_nl("op_abort", 1).await?;

        let (session_ab, session_ba) = common::connect_nodes(node_a, node_b).await?;

        session_ab.clone().abort(ABORT_REASON).await?;
        assert!(session_ab.is_aborted());
        let received_reason = match session_ba.recv_event().await? {
            Event::Abort(reason) => reason,
            ev => panic!("received event other than op:abort: {ev:?}"),
        };
        assert_eq!(received_reason, ABORT_REASON);
        Ok(())
    }) {
        Ok(_) => Ok(()),
        Err(error) => {
            tracing::error!(error, "failed");
            return Err(error);
        }
    }
}

// TODO :: crossed hellos
