use std::{
    borrow::Cow,
    num::{NonZeroUsize, ParseIntError},
    ops::RangeBounds,
};

use ::nom::Needed;

use super::{Literal, LiteralValue, Span, TokenTree};
use crate::de::Int;

mod nom;

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum CursorError {
    #[error("todo")]
    Error,
    #[error("incomplete input; needs {needed:?} bytes")]
    Incomplete { needed: Needed },
}

impl From<Needed> for CursorError {
    fn from(needed: Needed) -> Self {
        Self::Incomplete { needed }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseLiteralError {
    #[error(transparent)]
    Cursor(#[from] CursorError),
    #[error("could not parse bytes/string/symbol length: {0}")]
    ParseLen(#[from] ParseIntError),
    // #[error("string/symbol is invalid: {0}")]
    // Utf8(#[from] Utf8Error),
}

// impl From<ParseUtf8Error> for ParseLiteralError {
//     fn from(value: ParseUtf8Error) -> Self {
//         match value {
//             ParseUtf8Error::Cursor(c) => Self::Cursor(c),
//             ParseUtf8Error::Utf8(u) => Self::Utf8(u),
//         }
//     }
// }

pub type CResult<'rem, T, E = CursorError> = Result<(Cursor<'rem>, T), E>;

#[derive(Debug, Clone, Copy)]
pub struct Cursor<'remaining> {
    pub rem: &'remaining [u8],
    pub off: usize,
}

impl<'rem> From<&'rem [u8]> for Cursor<'rem> {
    fn from(rem: &'rem [u8]) -> Self {
        Self { rem, off: 0 }
    }
}

impl<'rem> Cursor<'rem> {
    pub const fn new(rem: &'rem [u8]) -> Self {
        Self { rem, off: 0 }
    }

    pub const fn advance(&self, bytes: usize) -> Cursor<'rem> {
        let (front, rem) = self.rem.split_at(bytes);
        Self {
            rem,
            off: self.off + front.len(),
        }
    }

    pub const fn advance_checked(&self, bytes: usize) -> Result<Cursor<'rem>, CursorError> {
        if self.rem.len() < bytes {
            Err(CursorError::Incomplete {
                #[allow(unsafe_code)]
                needed: Needed::Size(unsafe {
                    NonZeroUsize::new_unchecked(bytes - self.rem.len())
                }),
            })
        } else {
            Ok(self.advance(bytes))
        }
    }

    pub const fn peek_1(&self) -> Result<&u8, CursorError> {
        match self.rem.first() {
            Some(b) => Ok(b),
            None => Err(CursorError::Incomplete {
                #[allow(unsafe_code)]
                needed: Needed::Size(unsafe { NonZeroUsize::new_unchecked(1) }),
            }),
        }
    }

    pub fn slice_to(self, next: Self) -> Option<&'rem [u8]> {
        self.rem.get(..next.off.checked_sub(self.off)?)
    }

    pub const fn byte(self, pat: u8) -> Result<Self, CursorError> {
        match self.peek_1() {
            Ok(&b) if b == pat => Ok(self.advance(1)),
            Ok(_) => Err(CursorError::Error),
            Err(e) => Err(e),
        }
    }

    pub fn one_of(self, chars: &[u8]) -> Result<Self, CursorError> {
        if chars.contains(self.peek_1()?) {
            Ok(self.advance(1))
        } else {
            Err(CursorError::Error)
        }
    }

    pub fn in_range(self, range: impl RangeBounds<u8>) -> Result<Self, CursorError> {
        if range.contains(self.peek_1()?) {
            Ok(self.advance(1))
        } else {
            Err(CursorError::Error)
        }
    }

    pub fn tag(self, tag: &[u8]) -> Result<Self, CursorError> {
        let res = self.advance_checked(tag.len())?;
        if tag == self.slice_to(res).unwrap() {
            Ok(res)
        } else {
            Err(CursorError::Error)
        }
    }
}

/// Literal parsing.
impl<'rem> Cursor<'rem> {
    pub const fn bool(self) -> CResult<'rem, bool> {
        match self.peek_1() {
            Ok(b't') => Ok((self.advance(1), true)),
            Ok(b'f') => Ok((self.advance(1), false)),
            Ok(_) => Err(CursorError::Error),
            Err(e) => Err(e),
        }
    }

    pub fn digits<'digits>(self) -> CResult<'rem, DigitSpan<'digits>>
    where
        'rem: 'digits,
    {
        let mut len = 0;
        for b in self.rem {
            if b.is_ascii_digit() {
                len += 1;
            } else {
                break;
            }
        }
        match len {
            0 => Err(match self.rem.len() {
                0 => CursorError::Incomplete {
                    needed: Needed::Unknown,
                },
                _ => CursorError::Error,
            }),
            _ => {
                let res = self.advance(len);
                Ok((
                    res,
                    DigitSpan {
                        #[allow(unsafe_code)] // reason = we already checked that the whole slice is just ascii digits  
                        digits: unsafe {
                            std::str::from_utf8_unchecked(&self.rem[..(res.off - self.off)])
                        },
                    },
                ))
            }
        }
    }

    pub fn int(self) -> CResult<'rem, Int<'rem>> {
        let (rem, digits) = self.digits()?;
        rem.preparsed_int(digits)
    }

    pub const fn preparsed_int<'int>(self, digits: DigitSpan<'int>) -> CResult<'rem, Int<'int>> {
        let (rem, sign) = match self.peek_1() {
            Ok(b'+') => (self.advance(1), true),
            Ok(b'-') => (self.advance(1), false),
            Ok(_) => return Err(CursorError::Error),
            Err(e) => return Err(e),
        };
        Ok((rem, Int::new(sign, Cow::Borrowed(digits.digits))))
    }

    fn netstring(self, len: usize, separator: u8) -> CResult<'rem, &'rem [u8]> {
        let sep = self.byte(separator)?;
        let rem = sep.advance_checked(len)?;
        Ok((rem, sep.slice_to(rem).unwrap()))
    }

    pub fn bytes(self, len: usize) -> CResult<'rem, &'rem [u8]> {
        self.netstring(len, b':')
    }

    pub fn string(self, len: usize) -> CResult<'rem, &'rem [u8]> {
        self.netstring(len, b'"')
    }

    pub fn symbol(self, len: usize) -> CResult<'rem, &'rem [u8]> {
        self.netstring(len, b'\'')
    }

    fn float<const PREFIX: u8, const BYTE_AMT: usize>(self) -> CResult<'rem, &'rem [u8; BYTE_AMT]> {
        let prefix = self.byte(PREFIX)?;
        let rem = prefix.advance_checked(BYTE_AMT)?;
        let bytes: &[u8; BYTE_AMT] = prefix.slice_to(rem).unwrap().try_into().unwrap();
        Ok((rem, bytes))
    }

    pub fn f32(self) -> CResult<'rem, f32> {
        let (rem, bytes) = self.float::<b'F', 4>()?;
        Ok((rem, f32::from_be_bytes(*bytes)))
    }

    pub fn f64(self) -> CResult<'rem, f64> {
        let (rem, bytes) = self.float::<b'D', 8>()?;
        Ok((rem, f64::from_be_bytes(*bytes)))
    }
}

macro_rules! try_literal {
    ($try:expr, $Literal:ident, $res:ident, $into:expr) => {
        match $try {
            Ok((rem, $res)) => return Ok((rem, LiteralValue::$Literal($into))),
            Err(e @ CursorError::Incomplete { .. }) => return Err(e.into()),
            _ => {}
        }
    };
    ($try:expr, $Literal:ident) => {
        try_literal!($try, $Literal, res, res.into())
    };
}

impl<'rem> Cursor<'rem> {
    pub fn literal(self) -> CResult<'rem, Literal<'rem>, ParseLiteralError> {
        #[inline]
        fn inner(i: Cursor<'_>) -> CResult<'_, LiteralValue<'_>, ParseLiteralError> {
            try_literal!(i.bool(), Bool);
            try_literal!(i.f32(), F32);
            try_literal!(i.f64(), F64);
            let (rem, digits) = i.digits()?;
            try_literal!(rem.preparsed_int(digits), Int);
            let len = usize::try_from(digits)?;
            try_literal!(rem.symbol(len), Symbol);
            try_literal!(rem.bytes(len), Bytes);
            try_literal!(rem.string(len), String);
            Err(ParseLiteralError::Cursor(CursorError::Error))
        }
        let (rem, value) = inner(self)?;
        Ok((rem, Literal::new(value, Span::new(self.off, rem.off))))
    }

    pub fn literal_static(self) -> CResult<'rem, Literal<'static>, ParseLiteralError> {
        #[inline]
        fn inner(i: Cursor<'_>) -> CResult<'_, LiteralValue<'static>, ParseLiteralError> {
            try_literal!(i.bool(), Bool);
            try_literal!(i.f32(), F32);
            try_literal!(i.f64(), F64);
            let (rem, digits) = i.digits()?;
            try_literal!(rem.preparsed_int(digits), Int, res, res.into_static());
            let len = usize::try_from(digits)?;
            try_literal!(rem.symbol(len), Symbol, res, res.to_owned().into());
            try_literal!(rem.bytes(len), Bytes, res, res.to_owned().into());
            try_literal!(rem.string(len), String, res, res.to_owned().into());
            Err(ParseLiteralError::Cursor(CursorError::Error))
        }
        let (rem, value) = inner(self)?;
        Ok((rem, Literal::new(value, Span::new(self.off, rem.off))))
    }

    pub fn leaf_token(self) -> CResult<'rem, TokenTree<'rem>, ParseLiteralError> {
        self.literal()
            .map(|(input, lit)| (input, TokenTree::Literal(lit)))
    }

    pub fn leaf_token_static(self) -> CResult<'rem, TokenTree<'static>, ParseLiteralError> {
        self.literal_static()
            .map(|(input, lit)| (input, TokenTree::Literal(lit)))
    }
}

/// An artifact of [`Cursor::digits`].
#[derive(Debug, Clone, Copy)]
pub struct DigitSpan<'digits> {
    digits: &'digits str,
}

impl TryFrom<DigitSpan<'_>> for usize {
    type Error = ParseIntError;

    fn try_from(digits: DigitSpan<'_>) -> Result<Self, Self::Error> {
        digits.digits.parse::<usize>()
    }
}
