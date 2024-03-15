use super::{AsyncDataStream, AsyncStreamListener};
use rexa::locator::NodeLocator;
use std::net::SocketAddr;

pub type TcpIpNetlayer = super::DataStreamNetlayer<tokio::net::TcpListener>;

impl AsyncStreamListener for tokio::net::TcpListener {
    const TRANSPORT: &'static str = "tcpip";
    /// FIX :: https://github.com/rust-lang/rust/issues/63063
    type AddressInput<'addr> = &'addr SocketAddr;
    type AddressOutput = std::net::SocketAddr;
    type Error = std::io::Error;
    type Stream = tokio::net::TcpStream;

    async fn bind<'addr>(addr: Self::AddressInput<'addr>) -> Result<Self, Self::Error> {
        tokio::net::TcpListener::bind(addr).await
    }

    fn accept(
        &self,
    ) -> impl std::future::Future<Output = Result<(Self::Stream, SocketAddr), Self::Error>> + Send
    {
        tokio::net::TcpListener::accept(&self)
    }

    fn local_addr(&self) -> Result<Self::AddressOutput, Self::Error> {
        tokio::net::TcpListener::local_addr(&self)
    }

    fn designator(&self) -> Result<String, Self::Error> {
        Ok(self.local_addr()?.to_string())
    }
}

impl AsyncDataStream for tokio::net::TcpStream {
    type ReadHalf = tokio::net::tcp::OwnedReadHalf;
    type WriteHalf = tokio::net::tcp::OwnedWriteHalf;
    type Error = std::io::Error;

    fn connect<HKey, HVal>(
        addr: &NodeLocator<HKey, HVal>,
    ) -> impl std::future::Future<Output = Result<Self, Self::Error>> + std::marker::Send {
        tokio::net::TcpStream::connect(&addr.designator)
    }

    fn split(self) -> (Self::ReadHalf, Self::WriteHalf) {
        tokio::net::TcpStream::into_split(self)
    }
}
