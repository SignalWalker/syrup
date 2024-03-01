use crate::{
    async_compat::AsyncIoError,
    captp::msg::{OpDeliver, OpDeliverOnly},
};

#[derive(Debug, thiserror::Error)]
pub enum RecvError {
    #[error("failed to parse syrup: {0:?}")]
    Parse(syrup::ErrorKind),
    #[error(transparent)]
    Io(#[from] AsyncIoError),
    #[error("received abort message from remote; reason: {0}")]
    SessionAborted(String),
    #[error("attempted recv on locally aborted session")]
    SessionAbortedLocally,
    #[error("unknown delivery target: {0}, args: {1:?}")]
    UnknownTarget(u64, Vec<syrup::Item>),
}

impl<'input> From<syrup::Error<'input>> for RecvError {
    fn from(value: syrup::Error<'input>) -> Self {
        Self::Parse(value.kind)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error(transparent)]
    Io(#[from] AsyncIoError),
    #[error("received abort message from remote; reason: {0}")]
    SessionAborted(String),
    #[error("attempted send on locally aborted session")]
    SessionAbortedLocally,
}
