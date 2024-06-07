use std::{borrow::Cow, num::ParseIntError, str::Utf8Error};

mod tokens;
use nom::Needed;
pub use tokens::*;

mod cursor;
pub use cursor::*;

mod literal;
pub use literal::*;

mod group;
pub use group::*;

mod impl_decode;

// #[cfg(feature = "serde")]
// mod serde;
// #[cfg(feature = "serde")]
// pub use serde::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    lo: usize,
    hi: usize,
}

impl Span {
    pub const fn new(lo: usize, hi: usize) -> Self {
        Self { lo, hi }
    }
}

#[derive(Debug, Clone)]
pub enum DecodeErrorKind<'input> {
    Unexpected {
        expected: Cow<'input, str>,
        found: TokenTree<'input>,
    },
    Missing {
        expected: Cow<'input, str>,
    },
    Utf8 {
        input: Cow<'input, [u8]>,
        error: Utf8Error,
    },
    Int {
        input: Int<'input>,
        error: ParseIntError,
    },
}

#[derive(Debug, Clone)]
pub struct DecodeError<'input> {
    pub kind: DecodeErrorKind<'input>,
}

impl<'i> std::fmt::Display for DecodeError<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<'i> std::error::Error for DecodeError<'i> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            DecodeErrorKind::Utf8 { error, .. } => Some(error),
            DecodeErrorKind::Int { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl<'i> DecodeError<'i> {
    pub const fn unexpected(expected: Cow<'i, str>, found: TokenTree<'i>) -> Self {
        Self {
            kind: DecodeErrorKind::Unexpected { expected, found },
        }
    }

    pub const fn missing(expected: Cow<'i, str>) -> Self {
        Self {
            kind: DecodeErrorKind::Missing { expected },
        }
    }

    pub const fn utf8(input: Cow<'i, [u8]>, error: Utf8Error) -> Self {
        Self {
            kind: DecodeErrorKind::Utf8 { input, error },
        }
    }

    pub const fn int(input: Int<'i>, error: ParseIntError) -> Self {
        Self {
            kind: DecodeErrorKind::Int { input, error },
        }
    }
}

#[derive(Debug, Clone)]
pub enum DecodeBytesError<'input> {
    Lex(LexError),
    Decode(DecodeError<'input>),
}

impl<'i> From<LexError> for DecodeBytesError<'i> {
    fn from(value: LexError) -> Self {
        Self::Lex(value)
    }
}

impl<'i> From<DecodeError<'i>> for DecodeBytesError<'i> {
    fn from(value: DecodeError<'i>) -> Self {
        Self::Decode(value)
    }
}

impl<'i> std::fmt::Display for DecodeBytesError<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeBytesError::Lex(error) => std::fmt::Display::fmt(error, f),
            DecodeBytesError::Decode(error) => std::fmt::Display::fmt(error, f),
        }
    }
}

impl std::error::Error for DecodeBytesError<'static> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodeBytesError::Lex(error) => Some(error),
            DecodeBytesError::Decode(error) => Some(error),
        }
    }
}

pub trait Decode<'input>: Sized {
    //const TYPE_STR: &'static str;
    fn decode(input: TokenTree<'input>) -> Result<Self, DecodeError<'input>>;

    fn needed() -> Needed {
        Needed::Unknown
    }

    fn decode_bytes(input: &'input [u8]) -> Result<(Self, &'input [u8]), DecodeBytesError<'input>> {
        let (tree, rem) = TokenTree::tokenize(input.into())?;
        Ok((Self::decode(tree)?, rem.rem))
    }
}
