use common::{initialize_tracing, LogFormat};
use easy_parallel::Parallel;
use rexa::{
    locator::NodeLocator,
    netlayer::{tcpip::TcpIpNetlayer, Netlayer},
};
use smol::{net::SocketAddr, Executor};

mod common;

async fn make_node() -> Result<(TcpIpNetlayer, NodeLocator<String, String>), futures::io::Error> {
    let node = TcpIpNetlayer::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let locator = node.locator()?;
    tracing::debug!("started node at {}", locator.designator);
    Ok((node, locator))
}

#[test]
fn op_start() {
    initialize_tracing(LogFormat::Pretty);

    let ex = Executor::new();

    smol::block_on(async {
        let (node_a, _) = make_node().await.unwrap();
        let (node_b, locator_b) = make_node().await.unwrap();

        let (mut results, _) = Parallel::new()
            .add(|| smol::block_on(ex.run(node_a.connect(locator_b))))
            .add(|| smol::block_on(ex.run(node_b.accept())))
            .finish(|| tracing::info!("connecting nodes a & b..."));

        let session_ba = results.pop().unwrap().unwrap();
        let session_ab = results.pop().unwrap().unwrap();

        assert_eq!(
            session_ab.signing_key().verifying_key(),
            *session_ba.remote_vkey()
        );
    })
}

// #[test]
// fn op_abort() {
//     initialize_tracing(LogFormat::Pretty);
//
//     let ex = Executor::new();
//
//     smol::block_on(async {
//         let (node_a, _) = make_node().await.unwrap();
//         let (node_b, locator_b) = make_node().await.unwrap();
//
//         let (mut results, _) = Parallel::new()
//             .add(|| smol::block_on(ex.run(node_a.connect(locator_b))))
//             .add(|| smol::block_on(ex.run(node_b.accept())))
//             .finish(|| tracing::info!("connecting nodes a & b..."));
//
//         let session_ba = results.pop().unwrap().unwrap();
//         let session_ab = results.pop().unwrap().unwrap();
//
//         assert_eq!(
//             session_ab.signing_key().verifying_key(),
//             *session_ba.remote_vkey()
//         );
//     })
// }
