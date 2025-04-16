use std::{borrow::Cow, str::Utf8Error};

use crate::de::{DecodeIntError, Int, TokenTree};

#[derive(Debug, Clone)]
pub enum DecodeErrorKind<'data> {
    Unexpected {
        expected: Cow<'data, str>,
        // TODO :: do we have to own this token tree
        found: TokenTree,
    },
    Missing {
        expected: Cow<'data, str>,
    },
    Utf8 {
        input: Cow<'data, [u8]>,
        error: Utf8Error,
    },
    Int {
        input: Int<'static>,
        error: DecodeIntError,
    },
}

#[derive(Debug, Clone)]
pub struct DecodeError<'input> {
    pub kind: DecodeErrorKind<'input>,
}

impl<'i> std::fmt::Display for DecodeError<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO :: improve decode error display
        std::fmt::Debug::fmt(self, f)
    }
}

impl<'i> std::error::Error for DecodeError<'i> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            DecodeErrorKind::Utf8 { error, .. } => Some(error),
            DecodeErrorKind::Int { error, .. } => Some(error),
            // FIX :: ???
            _ => None,
        }
    }
}

impl<'i> DecodeError<'i> {
    pub const fn unexpected(expected: Cow<'i, str>, found: TokenTree) -> Self {
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

    pub const fn int(input: Int<'static>, error: DecodeIntError) -> Self {
        Self {
            kind: DecodeErrorKind::Int { input, error },
        }
    }
}

#[derive(Debug, Clone)]
pub enum DecodeBytesError<'input> {
    Lex(nom::Err<nom::error::Error<&'input [u8]>>),
    Decode(DecodeError<'input>),
}

impl<'i> From<nom::Err<nom::error::Error<&'i [u8]>>> for DecodeBytesError<'i> {
    fn from(value: nom::Err<nom::error::Error<&'i [u8]>>) -> Self {
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

impl<'i> std::error::Error for DecodeBytesError<'i> {
    // TODO :: source?
}
