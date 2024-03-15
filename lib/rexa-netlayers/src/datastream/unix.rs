use super::{AsyncDataStream, AsyncStreamListener};
use rexa::locator::NodeLocator;

pub type UnixNetlayer = super::DataStreamNetlayer<tokio::net::UnixListener>;

impl AsyncStreamListener for tokio::net::UnixListener {
    const TRANSPORT: &'static str = "unix";
    type AddressInput<'addr> = &'addr std::os::unix::net::SocketAddr;
    type AddressOutput = tokio::net::unix::SocketAddr;
    type Error = std::io::Error;
    type Stream = tokio::net::UnixStream;

    fn bind<'addr>(
        addr: Self::AddressInput<'addr>,
    ) -> impl std::future::Future<Output = Result<Self, Self::Error>> {
        async move {
            // tokio doesn't provide bind_addr
            let std_listener = std::os::unix::net::UnixListener::bind_addr(addr)?;
            // required for tokio to work as expected
            std_listener.set_nonblocking(true)?;
            tokio::net::UnixListener::from_std(std_listener)
        }
    }

    fn accept(
        &self,
    ) -> impl std::future::Future<Output = Result<(Self::Stream, Self::AddressOutput), Self::Error>>
           + std::marker::Send {
        tokio::net::UnixListener::accept(&self)
    }

    fn local_addr(&self) -> Result<Self::AddressOutput, Self::Error> {
        tokio::net::UnixListener::local_addr(&self)
    }

    fn designator(&self) -> Result<String, Self::Error> {
        // FIX :: tokio unix socketaddr does not suport as_abstract_namespace
        match self
            .local_addr()?
            .as_pathname()
            .and_then(std::path::Path::to_str)
        {
            Some(p) => Ok(p.to_owned()),
            None => todo!("handle unnamed unix streams"),
        }
    }
}

#[cfg(target_family = "unix")]
impl AsyncDataStream for tokio::net::UnixStream {
    type ReadHalf = tokio::net::unix::OwnedReadHalf;
    type WriteHalf = tokio::net::unix::OwnedWriteHalf;
    type Error = std::io::Error;

    fn connect<HKey, HVal>(
        addr: &NodeLocator<HKey, HVal>,
    ) -> impl std::future::Future<Output = Result<Self, Self::Error>> + std::marker::Send {
        tokio::net::UnixStream::connect(&addr.designator)
    }

    fn split(self) -> (Self::ReadHalf, Self::WriteHalf) {
        tokio::net::UnixStream::into_split(self)
    }
}
