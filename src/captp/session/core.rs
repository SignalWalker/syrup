use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Future};
use syrup::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct CapTpSessionCore<Socket> {
    pub(crate) socket: Socket,
}

impl<Socket> CapTpSessionCore<Socket> {
    #[inline]
    pub(crate) fn recv<'read>(
        &'read mut self,
        buf: &'read mut [u8],
    ) -> impl Future<Output = Result<usize, futures::io::Error>> + 'read
    where
        Socket: AsyncRead + Unpin,
    {
        self.socket.read(buf)
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
    pub(crate) fn send_all<'write>(
        &'write mut self,
        buf: &'write [u8],
    ) -> impl Future<Output = Result<(), futures::io::Error>> + 'write
    where
        Socket: AsyncWrite + Unpin,
    {
        self.socket.write_all(buf)
    }

    pub(crate) async fn send_msg<Msg: Serialize>(
        &mut self,
        msg: &Msg,
    ) -> Result<(), futures::io::Error>
    where
        Socket: AsyncWrite + Unpin,
    {
        // TODO :: custom error type
        self.send_all(&syrup::ser::to_bytes(msg).unwrap()).await
    }

    pub(crate) fn flush<'flush>(
        &'flush mut self,
    ) -> impl Future<Output = Result<(), futures::io::Error>> + 'flush
    where
        Socket: AsyncWrite + Unpin,
    {
        self.socket.flush()
    }

    pub(crate) async fn recv_msg<'de, Msg: Deserialize<'de>>(
        &mut self,
        recv_buf: &'de mut [u8],
    ) -> Result<Msg, futures::io::Error>
    where
        Socket: AsyncRead + Unpin,
    {
        let amt = self.recv(recv_buf).await?;
        Ok(syrup::de::from_bytes(&recv_buf[..amt]).unwrap())
    }
}
