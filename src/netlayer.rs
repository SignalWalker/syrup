//! - [Draft Specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Netlayers.md)

use crate::{captp::CapTpSession, locator::NodeLocator};

#[cfg(feature = "netlayer-onion")]
pub mod onion;

#[allow(async_fn_in_trait)]
pub trait Netlayer<Socket>: Sized {
    type Error;

    /// Attempt to open a new connection to the specified locator.
    async fn connect<HintKey: PartialEq + Eq + std::hash::Hash, HintValue>(
        &self,
        locator: &NodeLocator<HintKey, HintValue>,
    ) -> Result<CapTpSession<Socket>, Self::Error>;
    fn accept();
}
