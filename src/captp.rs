//! - [Draft Specification](<https://github.com/ocapn/ocapn/blob/main/draft-specifications/CapTP Specification.md>)

pub struct CapTpSession<Socket> {
    pub(crate) socket: Socket,
}

impl<Socket> CapTpSession<Socket> {
    pub(crate) async fn start_with(socket: Socket) -> Self {
        todo!()
    }
}
