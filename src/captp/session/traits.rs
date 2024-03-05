use super::{CapTpSessionInternal, Event, RecvError, SendError};
use crate::async_compat::{AsyncRead, AsyncWrite};
use crate::captp::msg::{OpDeliver, OpDeliverOnly};
use crate::captp::object::Resolver;
use crate::captp::{msg::DescImport, object::Answer};
use futures::FutureExt;

pub(crate) trait AbstractCapTpSession {
    fn deliver_only<'f>(
        &'f self,
        position: u64,
        args: Vec<syrup::RawSyrup>,
    ) -> futures::future::BoxFuture<'f, Result<(), SendError>>;
    fn deliver<'f>(
        &'f self,
        position: u64,
        args: Vec<syrup::RawSyrup>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> futures::future::BoxFuture<'f, Result<(), SendError>>;
    fn deliver_and<'f>(
        &'f self,
        position: u64,
        args: Vec<syrup::RawSyrup>,
    ) -> futures::future::BoxFuture<'f, Result<Answer, SendError>>;
}

impl<Reader: Send, Writer: AsyncWrite + Unpin + Send> AbstractCapTpSession
    for CapTpSessionInternal<Reader, Writer>
{
    fn deliver_only<'f>(
        &'f self,
        position: u64,
        args: Vec<syrup::RawSyrup>,
    ) -> futures::future::BoxFuture<'f, Result<(), SendError>> {
        async move {
            let del = OpDeliverOnly::new(position, args);
            self.send_msg(&del).await
        }
        .boxed()
    }

    fn deliver<'f>(
        &'f self,
        position: u64,
        args: Vec<syrup::RawSyrup>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> futures::future::BoxFuture<'f, Result<(), SendError>> {
        async move {
            let del = OpDeliver::new(position, args, answer_pos, resolve_me_desc);
            self.send_msg(&del).await
        }
        .boxed()
    }

    fn deliver_and<'f>(
        &'f self,
        position: u64,
        args: Vec<syrup::RawSyrup>,
    ) -> futures::future::BoxFuture<'f, Result<Answer, SendError>> {
        let (resolver, answer) = Resolver::new();
        let pos = self.export(resolver);
        async move {
            self.deliver(position, args, None, DescImport::Object(pos.into()))
                .await?;
            Ok(answer)
        }
        .boxed()
    }
}
