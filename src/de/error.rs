use std::{
    num::{
        IntErrorKind, NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize,
        NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize, Saturating,
        Wrapping,
    },
    str::Utf8Error,
};

use borrow_or_share::{BorrowOrShare, Bos};

use crate::de::{DecodeIntError, Int, TokenTree};

#[derive(Clone)]
pub enum DecodeErrorKind<Str = String, Bytes = Vec<u8>> {
    Unexpected {
        expected: Str,
        // TODO :: do we have to own this token tree
        found: TokenTree<Vec<u8>>,
    },
    Missing {
        expected: Str,
    },
    Utf8 {
        input: Bytes,
        error: Utf8Error,
    },
    Int {
        input: Int<Bytes>,
        error: DecodeIntError,
    },
}

impl<Str, Bytes> std::fmt::Debug for DecodeErrorKind<Str, Bytes>
where
    Str: Bos<str>,
    Bytes: Bos<[u8]>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unexpected { expected, found } => f
                .debug_struct("Unexpected")
                .field("expected", &expected.borrow_or_share())
                .field("found", found)
                .finish(),
            Self::Missing { expected } => f
                .debug_struct("Missing")
                .field("expected", &expected.borrow_or_share())
                .finish(),
            Self::Utf8 { input, error } => f
                .debug_struct("Utf8")
                .field("input", &input.borrow_or_share())
                .field("error", error)
                .finish(),
            Self::Int { input, error } => f
                .debug_struct("Int")
                .field("input", input)
                .field("error", error)
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntDescription {
    pub signed: bool,
    pub width: usize,
}

pub trait IsSigned {
    const SIGNED: bool;
}

impl IntDescription {
    #[inline]
    pub const fn describe<Int: IsSigned>() -> Self {
        Self {
            signed: Int::SIGNED,
            width: std::mem::size_of::<Int>(),
        }
    }

    #[inline]
    pub const fn new(signed: bool, width: usize) -> Self {
        Self { signed, width }
    }
}

macro_rules! impl_as_int_description {
    ($Int:ty, $signed:expr) => {
        impl IsSigned for $Int {
            const SIGNED: bool = $signed;
        }
    };
}

impl_as_int_description!(u8, false);
impl_as_int_description!(u16, false);
impl_as_int_description!(u32, false);
impl_as_int_description!(u64, false);
impl_as_int_description!(usize, false);
impl_as_int_description!(u128, false);
impl_as_int_description!(NonZeroU8, false);
impl_as_int_description!(NonZeroU16, false);
impl_as_int_description!(NonZeroU32, false);
impl_as_int_description!(NonZeroU64, false);
impl_as_int_description!(NonZeroUsize, false);
impl_as_int_description!(NonZeroU128, false);
impl_as_int_description!(i8, true);
impl_as_int_description!(i16, true);
impl_as_int_description!(i32, true);
impl_as_int_description!(i64, true);
impl_as_int_description!(isize, true);
impl_as_int_description!(i128, true);
impl_as_int_description!(NonZeroI8, true);
impl_as_int_description!(NonZeroI16, true);
impl_as_int_description!(NonZeroI32, true);
impl_as_int_description!(NonZeroI64, true);
impl_as_int_description!(NonZeroIsize, true);
impl_as_int_description!(NonZeroI128, true);

impl<Int: IsSigned> IsSigned for Wrapping<Int> {
    const SIGNED: bool = Int::SIGNED;
}

impl<Int: IsSigned> IsSigned for Saturating<Int> {
    const SIGNED: bool = Int::SIGNED;
}

impl std::fmt::Display for IntDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", if self.signed { 'i' } else { 'u' }, self.width)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SyrupKind {
    Unknown(&'static str),
    // literals
    Bool,
    F32,
    F64,
    Int { desc: Option<IntDescription> },
    Bytes { length: Option<usize> },
    String,
    Symbol(Option<&'static str>),
    // collections
    List { length: Option<usize> },
    Record { label: Option<&'static str> },
    Set,
    Dictionary,
}

impl SyrupKind {
    #[inline]
    pub const fn int<Int: IsSigned>() -> Self {
        Self::Int {
            desc: Some(IntDescription::describe::<Int>()),
        }
    }
}

impl std::fmt::Display for SyrupKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyrupKind::Unknown(msg) => write!(f, "Unknown({msg})"),
            SyrupKind::Bool => f.write_str("bool"),
            SyrupKind::F32 => f.write_str("f32"),
            SyrupKind::F64 => f.write_str("f64"),
            SyrupKind::Int { desc } => {
                if let Some(desc) = desc {
                    desc.fmt(f)
                } else {
                    f.write_str("int")
                }
            }
            SyrupKind::Bytes { length } => {
                if let Some(len) = length {
                    write!(f, "[u8; {len}]")
                } else {
                    f.write_str("[u8]")
                }
            }
            SyrupKind::String => f.write_str("string"),
            SyrupKind::Symbol(sym) => {
                if let Some(sym) = sym {
                    write!(f, "Symbol({sym})")
                } else {
                    f.write_str("Symbol")
                }
            }
            SyrupKind::List { length } => {
                if let Some(len) = length {
                    write!(f, "list({len})")
                } else {
                    f.write_str("list")
                }
            }
            SyrupKind::Record { label } => {
                if let Some(label) = label {
                    write!(f, "record<'{label}>")
                } else {
                    f.write_str("record")
                }
            }
            SyrupKind::Set => f.write_str("set"),
            SyrupKind::Dictionary => f.write_str("dictionary"),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum DecodeError {
    #[error("encountered invalid string or symbol")]
    InvalidUtf8,
    #[error("failed conversion to {0}: {1:?}")]
    ParseInt(IntDescription, IntErrorKind),
    #[error("expected: {expected}, found: {found}")]
    Unexpected {
        expected: SyrupKind,
        found: SyrupKind,
    },
    #[error("missing {0}")]
    Missing(SyrupKind),
}

impl DecodeError {
    #[inline]
    pub const fn int<Int: IsSigned>(error: IntErrorKind) -> Self {
        Self::ParseInt(IntDescription::describe::<Int>(), error)
    }

    pub fn unexpected<Data: Bos<[u8]>>(expected: SyrupKind, input: &TokenTree<Data>) -> Self {
        Self::Unexpected {
            expected,
            found: input.kind(),
        }
    }
}

impl From<Utf8Error> for DecodeError {
    fn from(_: Utf8Error) -> Self {
        Self::InvalidUtf8
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum DecodeBytesError<'input> {
    #[error(transparent)]
    Lex(nom::Err<nom::error::Error<&'input [u8]>>),
    #[error(transparent)]
    Decode(#[from] DecodeError),
}

impl<'i> From<nom::Err<nom::error::Error<&'i [u8]>>> for DecodeBytesError<'i> {
    fn from(value: nom::Err<nom::error::Error<&'i [u8]>>) -> Self {
        Self::Lex(value)
    }
}
