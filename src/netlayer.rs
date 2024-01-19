//! - [Draft Specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Netlayers.md)

use syrup::Serialize;

use crate::{captp::CapTpSession, locator::NodeLocator};

pub mod tcpip;

#[cfg(feature = "netlayer-mock")]
pub mod mock;

#[cfg(feature = "netlayer-onion")]
pub mod onion;

#[allow(async_fn_in_trait)]
pub trait Netlayer<Socket>: Sized {
    type Error;

    /// Attempt to open a new connection to the specified locator.
    async fn connect<HintKey: Serialize, HintValue: Serialize>(
        &self,
        locator: NodeLocator<HintKey, HintValue>,
    ) -> Result<CapTpSession<Socket>, Self::Error>;

    async fn accept(&self) -> Result<CapTpSession<Socket>, Self::Error>;
}
