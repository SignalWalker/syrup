use std::{
    borrow::Cow,
    fmt::Write,
    num::{
        IntErrorKind, NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize,
        NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize, ParseIntError,
    },
};

/// An integer literal.
#[derive(Clone, PartialEq, Eq)]
pub struct Int<'input> {
    pub positive: bool,
    /// SAFETY: must only contain ASCII digit characters 0-9
    digits: Cow<'input, [u8]>,
}

impl<'i> std::fmt::Debug for Int<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.positive {
            f.write_char('-')?;
        }
        f.write_str(self.digits())
    }
}

impl<'i> Int<'i> {
    pub fn encode(&self) -> Vec<u8> {
        let mut res = Vec::with_capacity(self.digits.len() + 1);
        res.extend(&*self.digits);
        res.push(if self.positive { b'+' } else { b'-' });
        res
    }

    /// # Safety
    ///
    /// `digits` must only contain ASCII decimal digits
    #[allow(unsafe_code)]
    pub const unsafe fn new(positive: bool, digits: Cow<'i, [u8]>) -> Self {
        Self { positive, digits }
    }

    #[inline]
    pub fn digits(&self) -> &str {
        #[allow(unsafe_code)]
        unsafe {
            std::str::from_utf8_unchecked(&self.digits)
        }
    }

    pub fn into_static(self) -> Int<'static> {
        match self.digits {
            Cow::Borrowed(d) => Int {
                positive: self.positive,
                digits: Cow::Owned(d.to_vec()),
            },
            Cow::Owned(d) => Int {
                positive: self.positive,
                digits: Cow::Owned(d),
            },
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub struct DecodeIntError {
    kind: IntErrorKind,
}

impl std::fmt::Display for DecodeIntError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl From<ParseIntError> for DecodeIntError {
    fn from(value: ParseIntError) -> Self {
        Self {
            kind: value.kind().clone(),
        }
    }
}

macro_rules! impl_uint {
    ($($Int:ty),+) => {
        $(
        impl<'i> TryFrom<&Int<'i>> for $Int {
            type Error = DecodeIntError;
            fn try_from(int: &Int<'i>) -> Result<$Int, Self::Error> {
                if !int.positive {
                    return Err(DecodeIntError { kind: IntErrorKind::NegOverflow });
                }
                int.digits().parse::<$Int>().map_err(From::from)
            }
        }

        impl<'i> From<$Int> for Int<'i> {
            fn from(val: $Int) -> Self {
                #[allow(unsafe_code)]
                unsafe { Self::new(true, Cow::Owned(val.to_string().into_bytes())) }
            }
        }
        )+
    };
}

macro_rules! impl_int {
    ($($UInt:ty => $Int:ty),+) => {
        $(
            impl<'i> TryFrom<&Int<'i>> for $Int {
                type Error = DecodeIntError;
                fn try_from(int: &Int<'i>) -> Result<$Int, Self::Error> {
                    const MAX_POSITIVE: $UInt = <$Int>::MAX.unsigned_abs();
                    const MAX_NEGATIVE: $UInt = <$Int>::MIN.unsigned_abs();
                    let val = int.digits().parse::<$UInt>()?;
                    // TODO :: this feels inelegant
                    if int.positive {
                        if val > MAX_POSITIVE {
                            return Err(DecodeIntError { kind: IntErrorKind::PosOverflow });
                        }
                        Ok(val.cast_signed())
                    } else {
                        if val > MAX_NEGATIVE {
                            return Err(DecodeIntError { kind: IntErrorKind::NegOverflow });
                        }
                        Ok(val.cast_signed().wrapping_neg())
                    }
                }
            }

            impl<'i> From<$Int> for Int<'i> {
                fn from(val: $Int) -> Self {
                    #[allow(unsafe_code)]
                    unsafe { Self::new(val.is_positive(), Cow::Owned(val.unsigned_abs().to_string().into_bytes())) }
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
    u8 => i8,
    u16 => i16,
    u32 => i32,
    u64 => i64,
    usize => isize,
    u128 => i128,
    NonZeroU8 => NonZeroI8,
    NonZeroU16 => NonZeroI16,
    NonZeroU32 => NonZeroI32,
    NonZeroU64 => NonZeroI64,
    NonZeroUsize => NonZeroIsize,
    NonZeroU128 => NonZeroI128
);
