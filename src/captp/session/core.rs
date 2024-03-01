use syrup::{Deserialize, Serialize};

use crate::async_compat::{AsyncRead, AsyncWrite};

#[cfg(all(feature = "futures", not(feature = "tokio")))]
use futures::{lock::Mutex, AsyncReadExt, AsyncWriteExt, Future};
#[cfg(all(feature = "tokio", not(feature = "futures")))]
use std::future::Future;
#[cfg(all(feature = "tokio", not(feature = "futures")))]
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

use super::AsyncIoError;

#[derive(Debug)]
pub struct CapTpSessionCore<Reader, Writer> {
    pub(crate) reader: Mutex<Reader>,
    pub(crate) writer: Mutex<Writer>,
}

impl<Reader, Writer> CapTpSessionCore<Reader, Writer> {
    pub(crate) fn new(reader: Reader, writer: Writer) -> Self {
        Self {
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
        }
    }

    #[inline]
    pub(crate) async fn recv(&self, buf: &mut [u8]) -> Result<usize, AsyncIoError>
    where
        Reader: AsyncRead + Unpin,
    {
        self.reader.lock().await.read(buf).await
    }

    // #[inline]
    // pub(crate) fn send<'write>(
    //     &'write mut self,
    //     buf: &'write [u8],
    // ) -> impl Future<Output = Result<usize, futures::io::Error>> + 'write
    // where
    //     Socket: AsyncWrite + Unpin,
    // {
    //     self.socket.write(buf)
    // }

    #[inline]
    pub(crate) async fn send_all(&self, buf: &[u8]) -> Result<(), AsyncIoError>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.writer.lock().await.write_all(buf).await
    }

    pub(crate) async fn send_msg<Msg: Serialize>(&self, msg: &Msg) -> Result<(), AsyncIoError>
    where
        Writer: AsyncWrite + Unpin,
    {
        // TODO :: custom error type
        self.send_all(&syrup::ser::to_bytes(msg).unwrap()).await
    }

    pub(crate) async fn flush(&self) -> Result<(), AsyncIoError>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.writer.lock().await.flush().await
    }

    pub(crate) async fn recv_msg<'de, Msg: Deserialize<'de>>(
        &self,
        recv_buf: &'de mut [u8],
    ) -> Result<Msg, AsyncIoError>
    where
        Reader: AsyncRead + Unpin,
    {
        let amt = self.recv(recv_buf).await?;
        Ok(syrup::de::from_bytes(&recv_buf[..amt]).unwrap())
    }
}
