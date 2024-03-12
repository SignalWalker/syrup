//! - [Draft Specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Netlayers.md)

use crate::{captp::CapTpSession, locator::NodeLocator};
use std::future::Future;
use syrup::Serialize;

#[cfg(feature = "netlayer-datastream")]
pub mod datastream;

#[cfg(feature = "netlayer-mock")]
pub mod mock;

#[cfg(feature = "netlayer-onion")]
pub mod onion;

pub trait Netlayer {
    type Reader;
    type Writer;
    type Error;

    /// Attempt to open a new connection to the specified locator.
    fn connect<HintKey: Serialize, HintValue: Serialize>(
        &self,
        locator: &NodeLocator<HintKey, HintValue>,
    ) -> impl Future<Output = Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error>> + Send
    where
        Self: Sized,
        NodeLocator<HintKey, HintValue>: Sync;

    /// Accept a connection.
    fn accept(
        &self,
    ) -> impl Future<Output = Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error>> + Send
    where
        Self: Sized;

    /// Get a locator pointing to this node.
    fn locator<HintKey, HintValue>(&self) -> NodeLocator<HintKey, HintValue>;

    /// Get a [Stream](futures::stream::Stream) of accepted connections.
    fn stream(
        &self,
    ) -> impl futures::stream::Stream<Item = Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error>>
    where
        Self: Sized,
    {
        futures::stream::unfold(self, |nl| async move { Some((nl.accept().await, nl)) })
    }
}
