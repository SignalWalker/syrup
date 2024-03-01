#[cfg(all(feature = "futures", not(feature = "tokio")))]
pub use futures::{AsyncRead, AsyncWrite};
#[cfg(all(feature = "tokio", not(feature = "futures")))]
pub use tokio::io::{AsyncRead, AsyncWrite};

#[cfg(all(feature = "futures", not(feature = "tokio")))]
pub type AsyncIoError = futures::io::Error;
#[cfg(all(feature = "tokio", not(feature = "futures")))]
pub type AsyncIoError = std::io::Error;
