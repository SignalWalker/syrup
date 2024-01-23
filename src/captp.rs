//! - [Draft Specification](<https://github.com/ocapn/ocapn/blob/main/draft-specifications/CapTP Specification.md>)

pub mod msg;

mod session;
pub use session::*;

// pub struct RecvMsg<'recv, Reader: Unpin + ?Sized> {
//     reader: &'recv mut Reader,
//     buf: Vec<u8>,
// }
//
// impl<'recv, R: Unpin + ?Sized> RecvMsg<'recv, R> {
//     fn new(reader: &'recv mut R, initial_capacity: usize) -> Self {
//         Self {
//             reader,
//             buf: Vec::with_capacity(initial_capacity),
//         }
//     }
// }
//
// impl<'de, 'recv, R: Unpin + ?Sized, Msg: Deserialize<'de>> Unpin for RecvMsg<'de, 'recv, R, Msg> {}
//
// impl<'de, 'recv: 'de, R: AsyncRead + Unpin + ?Sized, Msg: Deserialize<'de>> Future
//     for RecvMsg<'de, 'recv, R, Msg>
// {
//     type Output = Result<Msg, syrup::Error<'static>>;
//
//     fn poll(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Self::Output> {
//         // let buf = self.get_mut().buf;
//         let recv = self.get_mut();
//         let amt = ready!(Pin::new(&mut *recv.reader).poll_read(cx, &mut *recv.buf)).unwrap();
//         match syrup::de::from_bytes::<Msg>(recv.buf) {
//             Ok(res) => Poll::Ready(Ok(res)),
//             Err(_) => todo!(),
//         }
//     }
// }
