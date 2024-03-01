//! - [Draft Specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Netlayers.md)

use crate::{
    captp::{CapTpSession, SwissRegistry},
    locator::NodeLocator,
};
use std::sync::Arc;
use syrup::Serialize;

#[cfg(feature = "netlayer-tcpip-smol")]
pub mod tcpip;

#[cfg(feature = "netlayer-mock")]
pub mod mock;

#[cfg(feature = "netlayer-onion")]
pub mod onion;

#[allow(async_fn_in_trait)]
pub trait Netlayer: Sized {
    type Reader;
    type Writer;
    type Error;

    /// Attempt to open a new connection to the specified locator.
    async fn connect<HintKey: Serialize, HintValue: Serialize>(
        &self,
        locator: NodeLocator<HintKey, HintValue>,
    ) -> Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error>;

    async fn accept(&self) -> Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error>;

    fn stream(
        &self,
    ) -> impl futures::stream::Stream<Item = Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error>>
    {
        futures::stream::unfold(self, |nl| async move { Some((nl.accept().await, nl)) })
    }
}
