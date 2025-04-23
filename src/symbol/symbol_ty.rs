use std::{borrow::Borrow, ops::Deref};

use borrow_or_share::{BorrowOrShare, Bos};

use crate::{
    Decode, Encode,
    de::{DecodeError, Literal, SyrupKind, TokenTree},
};

/// A wrapper around [`Cow<'_, str>`] that encodes/decodes as a symbol literal
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Symbol<S>(pub S);

impl<S: Bos<str>> Borrow<str> for Symbol<S> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[cfg(debug_assertions)]
mod __impl_assertions {
    use std::borrow::{Borrow, Cow};

    use static_assertions_next::assert_impl_all;

    use crate::symbol::Symbol;

    assert_impl_all!(Symbol<&str>: Borrow<str>);
    assert_impl_all!(Symbol<String>: Borrow<str>);
    assert_impl_all!(Symbol<Cow<'_, str>>: Borrow<str>);
}

impl<S: Bos<str>> AsRef<str> for Symbol<S> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<S: Bos<str>> Deref for Symbol<S> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<S: AsRef<str>> std::fmt::Debug for Symbol<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}", self.0.as_ref())
    }
}

impl<'i, 'o, S: BorrowOrShare<'i, 'o, str>> Symbol<S> {
    #[inline]
    pub fn as_symbol_ref(&'i self) -> Symbol<&'o str> {
        Symbol(self.0.borrow_or_share())
    }

    #[inline]
    pub fn as_str(&'i self) -> &'o str {
        self.0.borrow_or_share()
    }
}

impl<'s, S: AsRef<str>> From<&'s S> for Symbol<&'s str> {
    fn from(value: &'s S) -> Self {
        Self(value.as_ref())
    }
}

impl From<String> for Symbol<String> {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl<'s, S: ToOwned<Owned = String>> From<&'s S> for Symbol<String> {
    fn from(value: &'s S) -> Self {
        Self(value.to_owned())
    }
}

impl From<Symbol<String>> for String {
    fn from(value: Symbol<String>) -> Self {
        value.0
    }
}

impl<'s> From<Symbol<&'s str>> for String {
    fn from(value: Symbol<&'s str>) -> Self {
        value.0.to_owned()
    }
}

impl<'sym, 'str, S: BorrowOrShare<'sym, 'str, str>> From<&'sym Symbol<S>> for &'str str {
    fn from(value: &'sym Symbol<S>) -> Self {
        value.0.borrow_or_share()
    }
}

impl<'input, 'output, Str, OData> Encode<'input, OData> for Symbol<Str>
where
    Str: BorrowOrShare<'input, 'output, str>,
    &'output [u8]: Into<OData>,
{
    fn encode(&'input self) -> TokenTree<OData> {
        TokenTree::Literal(Literal::Symbol(self.0.borrow_or_share().as_bytes().into()))
    }
}

impl<'i, 'o, IData, Str> Decode<'i, IData> for Symbol<Str>
where
    IData: BorrowOrShare<'i, 'o, [u8]>,
    &'o str: Into<Str>,
{
    fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
        match input {
            TokenTree::Literal(Literal::Symbol(s)) => {
                Ok(Self(std::str::from_utf8(s.borrow_or_share())?.into()))
            }
            _ => Err(DecodeError::Unexpected {
                expected: SyrupKind::Symbol(None),
                found: input.kind(),
            }),
        }
    }
}
