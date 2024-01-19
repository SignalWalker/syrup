use futures::{AsyncRead, Future};
use smol::channel::{Receiver, Sender};
use std::{
    io::Write,
    pin::pin,
    task::{ready, Poll},
};

pub struct MockNetlayer;
