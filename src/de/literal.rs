use std::{
    array::TryFromSliceError,
    borrow::{Borrow, Cow},
    fmt::Write,
    marker::PhantomData,
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
    },
    ops::Deref,
};

use crate::de::Span;

#[derive(Debug, Clone, Eq)]
pub struct Literal<'input> {
    pub repr: LiteralValue<'input>,
    pub span: Span,
}

impl<'i> std::fmt::Display for Literal<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.repr.fmt(f)
    }
}

impl<'input> Literal<'input> {
    pub const fn new(repr: LiteralValue<'input>, span: Span) -> Self {
        Self { repr, span }
    }

    pub fn encode(&self) -> Cow<'_, [u8]> {
        self.repr.as_bytes()
    }

    pub fn into_static(self) -> Literal<'static> {
        Literal {
            span: self.span,
            repr: self.repr.into_static(),
        }
    }
}

impl PartialEq for Literal<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.repr.eq(&other.repr)
    }
}

impl PartialOrd for Literal<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Literal<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.repr.cmp(&other.repr)
    }
}

impl std::hash::Hash for Literal<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.repr.hash(state);
    }
}

#[derive(Debug, Clone)]
pub enum LiteralValue<'input> {
    Bool(bool),
    F32(f32),
    F64(f64),
    Int(Int<'input>),
    Bytes(Cow<'input, [u8]>),
    String(Cow<'input, [u8]>),
    Symbol(Cow<'input, [u8]>),
}

impl<'i> std::fmt::Display for LiteralValue<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiteralValue::Bool(false) => f.write_char('f'),
            LiteralValue::Bool(true) => f.write_char('t'),
            LiteralValue::F32(fl) => fl.fmt(f),
            LiteralValue::F64(d) => d.fmt(f),
            LiteralValue::Int(i) => i.fmt(f),
            LiteralValue::Bytes(b) => write!(f, ":{} bytes:", b.len()),
            LiteralValue::String(s) => write!(f, "\"{}\"", String::from_utf8_lossy(s)),
            LiteralValue::Symbol(s) => f.write_str(&*String::from_utf8_lossy(s)),
        }
    }
}

impl<'i> LiteralValue<'i> {
    pub fn as_bytes<'bytes>(&'bytes self) -> Cow<'bytes, [u8]>
    where
        'i: 'bytes,
    {
        match self {
            LiteralValue::Bool(false) => Cow::Borrowed(b"f"),
            LiteralValue::Bool(true) => Cow::Borrowed(b"t"),
            LiteralValue::F32(f) => Cow::Owned(f.to_be_bytes().to_vec()),
            LiteralValue::F64(d) => Cow::Owned(d.to_be_bytes().to_vec()),
            LiteralValue::Int(i) => Cow::Owned(i.clone().into_bytes()),
            LiteralValue::Bytes(b) | LiteralValue::String(b) | LiteralValue::Symbol(b) => b.clone(),
        }
    }

    pub fn into_bytes(self) -> Cow<'i, [u8]> {
        match self {
            LiteralValue::Bool(false) => Cow::Borrowed(b"f"),
            LiteralValue::Bool(true) => Cow::Borrowed(b"t"),
            LiteralValue::F32(f) => Cow::Owned(f.to_be_bytes().to_vec()),
            LiteralValue::F64(d) => Cow::Owned(d.to_be_bytes().to_vec()),
            LiteralValue::Int(i) => Cow::Owned(i.into_bytes()),
            LiteralValue::Bytes(b) | LiteralValue::String(b) | LiteralValue::Symbol(b) => b,
        }
    }

    pub fn into_static(self) -> LiteralValue<'static> {
        match self {
            LiteralValue::Bool(b) => LiteralValue::Bool(b),
            LiteralValue::F32(f) => LiteralValue::F32(f),
            LiteralValue::F64(d) => LiteralValue::F64(d),
            LiteralValue::Int(i) => LiteralValue::Int(i.into_static()),
            LiteralValue::Bytes(b) => LiteralValue::Bytes(Cow::Owned(b.into_owned())),
            LiteralValue::String(s) => LiteralValue::String(Cow::Owned(s.into_owned())),
            LiteralValue::Symbol(s) => LiteralValue::Symbol(Cow::Owned(s.into_owned())),
        }
    }
}

impl<'i> PartialEq for LiteralValue<'i> {
    fn eq(&self, other: &Self) -> bool {
        (*self.as_bytes()).eq(&*other.as_bytes())
    }
}

impl<'i> Eq for LiteralValue<'i> {}

impl<'i> PartialOrd for LiteralValue<'i> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'i> Ord for LiteralValue<'i> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self.as_bytes()).cmp(&*other.as_bytes())
    }
}

impl<'i> std::hash::Hash for LiteralValue<'i> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}

impl From<bool> for LiteralValue<'_> {
    fn from(val: bool) -> Self {
        Self::Bool(val)
    }
}

impl From<f32> for LiteralValue<'_> {
    fn from(val: f32) -> Self {
        Self::F32(val)
    }
}

impl From<f64> for LiteralValue<'_> {
    fn from(val: f64) -> Self {
        Self::F64(val)
    }
}

impl<'lit> From<Int<'lit>> for LiteralValue<'lit> {
    fn from(int: Int<'lit>) -> Self {
        Self::Int(int)
    }
}

impl<'lit> From<Bytes<'lit>> for LiteralValue<'lit> {
    fn from(bytes: Bytes<'lit>) -> Self {
        Self::Bytes(bytes.0)
    }
}

impl<'lit> From<Cow<'lit, str>> for LiteralValue<'lit> {
    fn from(string: Cow<'lit, str>) -> Self {
        Self::String(match string {
            Cow::Borrowed(s) => Cow::<'lit, [u8]>::Borrowed(s.as_bytes()),
            Cow::Owned(s) => Cow::<'lit, [u8]>::Owned(s.into_bytes()),
        })
    }
}

impl<'lit> From<Symbol<'lit>> for LiteralValue<'lit> {
    fn from(sym: Symbol<'lit>) -> Self {
        Self::Symbol(match sym.0 {
            Cow::Borrowed(s) => Cow::<'lit, [u8]>::Borrowed(s.as_bytes()),
            Cow::Owned(s) => Cow::<'lit, [u8]>::Owned(s.into_bytes()),
        })
    }
}

/// An integer literal.
#[derive(Debug, Clone, PartialEq)]
pub struct Int<'digits> {
    pub positive: bool,
    digits: Cow<'digits, str>,
}

impl<'d> std::fmt::Display for Int<'d> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.positive {
            f.write_char('-')?;
        }
        f.write_str(&*self.digits)
    }
}

impl<'d> Int<'d> {
    pub const fn new(positive: bool, digits: Cow<'d, str>) -> Self {
        Self { positive, digits }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let mut res = self.digits.into_owned().into_bytes();
        res.reserve_exact(1);
        res.push(match self.positive {
            true => b'+',
            false => b'-',
        });
        res
    }

    pub fn into_static(self) -> Int<'static> {
        Int {
            positive: self.positive,
            digits: Cow::Owned(self.digits.into_owned()),
        }
    }
}

macro_rules! impl_uint {
    ($($Int:ty),+) => {
        $(
        impl<'d> TryFrom<&Int<'d>> for $Int {
            type Error = std::num::ParseIntError;
            fn try_from(int: &Int<'d>) -> Result<$Int, Self::Error> {
                int.digits.parse::<$Int>()
            }
        }

        impl<'d> From<$Int> for Int<'d> {
            fn from(val: $Int) -> Self {
                Self {
                    positive: true,
                    digits: val.to_string().into()
                }
            }
        }
        )+
    };
}

macro_rules! impl_int {
    ($($Int:ty),+) => {
        $(
        impl<'d> TryFrom<&Int<'d>> for $Int {
            type Error = std::num::ParseIntError;
            fn try_from(int: &Int<'d>) -> Result<$Int, Self::Error> {
                let mut val = int.digits.parse::<$Int>()?;
                if !int.positive {
                    val = match val.checked_neg() {
                        Some(val) => val,
                        None => todo!()
                    };
                }
                Ok(val)
            }
        }

        impl<'d> From<$Int> for Int<'d> {
            fn from(val: $Int) -> Self {
                Self {
                    positive: val.is_positive(),
                    digits: {
                        let mut digits = val.to_string();
                        if let Some('-') = digits.chars().nth(0) {
                            digits.remove(0);
                        }
                        digits.into()
                    }
                }
            }
        }
        )+
    };
}

impl_uint!(
    u8,
    u16,
    u32,
    u64,
    usize,
    u128,
    NonZeroU8,
    NonZeroU16,
    NonZeroU32,
    NonZeroU64,
    NonZeroUsize,
    NonZeroU128
);
impl_int!(
    i8,
    i16,
    i32,
    i64,
    isize,
    i128,
    NonZeroI8,
    NonZeroI16,
    NonZeroI32,
    NonZeroI64,
    NonZeroIsize,
    NonZeroI128
);

/// A symbol literal.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol<'symbol>(pub Cow<'symbol, str>);

impl<'s> Deref for Symbol<'s> {
    type Target = Cow<'s, str>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'s> Borrow<str> for Symbol<'s> {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl<'s, S: Into<Cow<'s, str>>> From<S> for Symbol<'s> {
    fn from(string: S) -> Self {
        Self(string.into())
    }
}

/// A byte vector literal.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes<'bytes>(pub Cow<'bytes, [u8]>);

impl<'b> Deref for Bytes<'b> {
    type Target = Cow<'b, [u8]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'b, B: Into<Cow<'b, [u8]>>> From<B> for Bytes<'b> {
    fn from(bytes: B) -> Self {
        Self(bytes.into())
    }
}

impl<'b, const LEN: usize> TryFrom<Bytes<'b>> for [u8; LEN] {
    type Error = TryFromSliceError;

    fn try_from(value: Bytes<'b>) -> Result<Self, Self::Error> {
        (&*value.0).try_into()
    }
}

/// A byte vector literal.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteArray<'bytes, const LEN: usize>(Cow<'bytes, [u8]>, PhantomData<[u8; LEN]>);

impl<'b, const LEN: usize> ByteArray<'b, LEN> {
    pub fn repr(array: &Self) -> &Cow<'b, [u8]> {
        &array.0
    }

    pub fn bytes(array: &Self) -> &[u8; LEN] {
        let cow = Self::repr(array);
        if cfg!(debug_assertions) {
            <&[u8; LEN]>::try_from(&**cow).unwrap()
        } else {
            #[allow(unsafe_code)] // reason = this is checked at construction
            unsafe {
                <&[u8; LEN]>::try_from(&**cow).unwrap_unchecked()
            }
        }
    }
}

impl<'b, const LEN: usize> Deref for ByteArray<'b, LEN> {
    type Target = [u8; LEN];

    fn deref(&self) -> &Self::Target {
        Self::bytes(self)
    }
}

impl<'b, const LEN: usize> From<[u8; LEN]> for ByteArray<'b, LEN> {
    fn from(bytes: [u8; LEN]) -> Self {
        Self(Cow::Owned(bytes.into()), PhantomData)
    }
}

impl<'b, const LEN: usize> From<&'b [u8; LEN]> for ByteArray<'b, LEN> {
    fn from(bytes: &'b [u8; LEN]) -> Self {
        Self(Cow::Borrowed(bytes), PhantomData)
    }
}

impl<'b, const LEN: usize> TryFrom<&'b [u8]> for ByteArray<'b, LEN> {
    type Error = TryFromSliceError;

    fn try_from(bytes: &'b [u8]) -> Result<Self, Self::Error> {
        Ok(Self(
            Cow::Borrowed(<&'b [u8; LEN]>::try_from(bytes)?),
            PhantomData,
        ))
    }
}

impl<'b, const LEN: usize> TryFrom<Vec<u8>> for ByteArray<'b, LEN> {
    type Error = TryFromSliceError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let _ = <&[u8; LEN]>::try_from(bytes.as_slice())?;
        Ok(Self(Cow::Owned(bytes), PhantomData))
    }
}

impl<'b, const LEN: usize> TryFrom<Cow<'b, [u8]>> for ByteArray<'b, LEN> {
    type Error = TryFromSliceError;

    fn try_from(value: Cow<'b, [u8]>) -> Result<Self, Self::Error> {
        match value {
            Cow::Borrowed(bytes) => Self::try_from(bytes),
            Cow::Owned(bytes) => Self::try_from(bytes),
        }
    }
}
