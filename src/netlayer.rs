//! - [Draft Specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Netlayers.md)

use crate::{
    captp::{CapTpSession, SwissRegistry},
    locator::NodeLocator,
};
use std::future::Future;
use std::sync::Arc;
use syrup::Serialize;

#[cfg(feature = "netlayer-datastream")]
pub mod datastream;

#[cfg(feature = "netlayer-mock")]
pub mod mock;

#[cfg(feature = "netlayer-onion")]
pub mod onion;

pub trait Netlayer: Sized {
    type Reader;
    type Writer;
    type Error;

    /// Attempt to open a new connection to the specified locator.
    fn connect<HintKey: Serialize, HintValue: Serialize>(
        &self,
        locator: &NodeLocator<HintKey, HintValue>,
    ) -> impl Future<Output = Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error>> + Send
    where
        NodeLocator<HintKey, HintValue>: Sync;

    fn accept(
        &self,
    ) -> impl Future<Output = Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error>> + Send;

    fn locator<HintKey, HintValue>(&self) -> NodeLocator<HintKey, HintValue>;

    fn stream(
        &self,
    ) -> impl futures::stream::Stream<Item = Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error>>
    {
        futures::stream::unfold(self, |nl| async move { Some((nl.accept().await, nl)) })
    }
}
