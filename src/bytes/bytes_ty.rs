use std::{
    borrow::{Borrow, Cow},
    ops::Deref,
};

use borrow_or_share::{BorrowOrShare, Bos};

use crate::{
    Decode, DecodeError, Encode, TokenTree,
    de::{Literal, SyrupKind},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes<B>(pub B);

#[cfg(debug_assertions)]
mod __impl_assertions {
    use std::borrow::{Borrow, Cow};

    use static_assertions_next::assert_impl_all;

    use crate::bytes::Bytes;

    assert_impl_all!(Bytes<&[u8]>:         Borrow<[u8]>, Deref<Target = [u8]>, AsRef<[u8]>);
    assert_impl_all!(Bytes<Vec<u8>>:       Borrow<[u8]>, Deref<Target = [u8]>, AsRef<[u8]>);
    assert_impl_all!(Bytes<Cow<'_, [u8]>>: Borrow<[u8]>, Deref<Target = [u8]>, AsRef<[u8]>);
}

impl<B: Bos<[u8]>> Deref for Bytes<B> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.borrow_or_share()
    }
}

impl<B: Bos<[u8]>> Borrow<[u8]> for Bytes<B> {
    fn borrow(&self) -> &[u8] {
        self.0.borrow_or_share()
    }
}

impl<B: Bos<[u8]>> AsRef<[u8]> for Bytes<B> {
    fn as_ref(&self) -> &[u8] {
        self.0.borrow_or_share()
    }
}

impl<'b, B: AsRef<[u8]>> From<&'b B> for Bytes<&'b [u8]> {
    fn from(value: &'b B) -> Self {
        Self(value.as_ref())
    }
}

impl From<Vec<u8>> for Bytes<Vec<u8>> {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl<'b, B: ToOwned<Owned = Vec<u8>>> From<&'b B> for Bytes<Vec<u8>> {
    fn from(value: &'b B) -> Self {
        Self(value.to_owned())
    }
}

impl From<Bytes<Vec<u8>>> for Vec<u8> {
    fn from(value: Bytes<Vec<u8>>) -> Self {
        value.0
    }
}

impl<'b> From<Bytes<&'b [u8]>> for Vec<u8> {
    fn from(value: Bytes<&'b [u8]>) -> Self {
        value.0.to_owned()
    }
}

impl<'b> From<Bytes<Cow<'b, [u8]>>> for Vec<u8> {
    fn from(value: Bytes<Cow<'b, [u8]>>) -> Self {
        value.0.clone().into_owned()
    }
}

impl<'i, 'o, B: BorrowOrShare<'i, 'o, [u8]>> From<&'i Bytes<B>> for &'o [u8] {
    fn from(value: &'i Bytes<B>) -> Self {
        value.0.borrow_or_share()
    }
}

impl<'input, 'output, IData, OData> Encode<'input, OData> for Bytes<IData>
where
    IData: BorrowOrShare<'input, 'output, [u8]>,
    &'output [u8]: Into<OData>,
{
    fn encode(&'input self) -> TokenTree<OData> {
        TokenTree::Literal(Literal::Bytes(self.0.borrow_or_share().into()))
    }
}

impl<'tree, 'output, IData, OData> Decode<'tree, IData> for Bytes<OData>
where
    IData: BorrowOrShare<'tree, 'output, [u8]>,
    &'output [u8]: Into<OData>,
{
    fn decode(input: &'tree crate::TokenTree<IData>) -> Result<Self, crate::DecodeError> {
        match input {
            TokenTree::Literal(Literal::Bytes(b)) => Ok(Self(b.borrow_or_share().into())),
            _ => Err(DecodeError::unexpected(
                SyrupKind::Bytes { length: None },
                input,
            )),
        }
    }
}
