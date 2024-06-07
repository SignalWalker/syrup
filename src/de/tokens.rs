use std::{borrow::Cow, num::ParseIntError};

use crate::{
    de::{Delimiter, ParseLiteralError},
    Span,
};

use nom::Needed;

mod stream;
pub use stream::*;

mod tree;
pub use tree::*;

#[derive(Debug, Clone)]
pub enum LexErrorKind {
    Incomplete {
        needed: Needed,
    },
    Unexpected {
        expected: &'static str,
    },
    /// Found unmatched closing delimiter.
    Unmatched {
        expected: Delimiter,
    },
    /// Could not parse length of byte/str/symbol literal
    Int(ParseIntError),
}

#[derive(Debug, Clone)]
pub struct LexError {
    //pub input: Cow<'input, [u8]>,
    pub span: Span,
    pub kind: LexErrorKind,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl LexError {
    pub const fn new(span: Span, kind: LexErrorKind) -> Self {
        Self { span, kind }
    }

    pub const fn incomplete(span: Span, needed: Needed) -> Self {
        Self::new(span, LexErrorKind::Incomplete { needed })
    }

    //const fn unknown(input: Cow<'i, [u8]>, span: Span) -> Self {
    //    Self::new(input, span, LexErrorKind::Unknown)
    //}

    const fn unexpected(span: Span, expected: &'static str) -> Self {
        Self::new(span, LexErrorKind::Unexpected { expected })
    }

    const fn unmatched(span: Span, expected: Delimiter) -> Self {
        Self::new(span, LexErrorKind::Unmatched { expected })
    }

    const fn from_parse_literal(span: Span, error: ParseLiteralError) -> Self {
        match error {
            ParseLiteralError::ParseLen(e) => Self::new(span, LexErrorKind::Int(e)),
            ParseLiteralError::Cursor(e) => match e {
                super::CursorError::Error => todo!(),
                super::CursorError::Incomplete { needed } => Self::incomplete(span, needed),
            },
        }
    }

    //const fn from_cursor_error(span: Span, error: CursorError) -> Self {
    //    Self::new(
    //        span,
    //        match error {
    //            CursorError::Error => LexErrorKind::Unknown,
    //            CursorError::Incomplete { needed } => LexErrorKind::Incomplete { needed },
    //        },
    //    )
    //}
}

impl std::error::Error for LexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            LexErrorKind::Int(e) => Some(e),
            _ => None,
        }
    }

    //fn provide<'a>(&'a self, request: &mut std::error::Request<'a>) {}
}
