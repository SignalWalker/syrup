use std::{
    borrow::Cow,
    io::Write,
    num::{
        IntErrorKind, NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize,
        NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize, ParseIntError,
    },
};

use borrow_or_share::{BorrowOrShare, Bos};

/// An integer literal.
#[derive(Clone, Copy, Hash)]
pub struct Int<Digits> {
    pub positive: bool,
    /// SAFETY: must only contain ASCII digit characters 0-9
    digits: Digits,
}

impl<LDigits, RDigits> PartialEq<Int<RDigits>> for Int<LDigits>
where
    LDigits: PartialEq<RDigits>,
{
    fn eq(&self, other: &Int<RDigits>) -> bool {
        self.positive == other.positive && self.digits.eq(&other.digits)
    }
}

impl<Digits> std::fmt::Debug for Int<Digits>
where
    Digits: Bos<[u8]>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        if !self.positive {
            f.write_char('-')?;
        }
        f.write_str(self.digits())
    }
}

impl<Digits> Int<Digits> {
    pub fn encode<'i, 'o>(&'i self) -> Cow<'o, [u8]>
    where
        Digits: BorrowOrShare<'i, 'o, [u8]>,
    {
        let digits = self.digits.borrow_or_share();
        let mut res = Vec::with_capacity(digits.len() + 1);
        res.extend_from_slice(digits);
        res.push(if self.positive { b'+' } else { b'-' });
        Cow::Owned(res)
    }

    pub fn encode_into(&self, w: &mut impl Write) -> std::io::Result<usize>
    where
        Digits: Bos<[u8]>,
    {
        let digits = self.digits.borrow_or_share();
        w.write_all(digits)?;
        w.write_all(if self.positive { b"+" } else { b"-" })?;
        Ok(digits.len() + 1)
    }

    /// # Safety
    ///
    /// `digits` must only contain ASCII decimal digits
    #[expect(unsafe_code)]
    pub const unsafe fn new(positive: bool, digits: Digits) -> Self {
        Self { positive, digits }
    }

    #[inline]
    pub fn digits<'i, 'o>(&'i self) -> &'o str
    where
        Digits: BorrowOrShare<'i, 'o, [u8]>,
    {
        #[expect(unsafe_code)]
        unsafe {
            std::str::from_utf8_unchecked(self.digits.borrow_or_share())
        }
    }

    pub fn digits_into<IDigits>(self) -> Int<IDigits>
    where
        Digits: Into<IDigits>,
    {
        Int {
            positive: self.positive,
            digits: self.digits.into(),
        }
    }
}

impl<'i, 'o, IDigits, ODigits> From<&'i Int<IDigits>> for Int<ODigits>
where
    IDigits: BorrowOrShare<'i, 'o, [u8]>,
    &'o [u8]: Into<ODigits>,
{
    fn from(value: &'i Int<IDigits>) -> Self {
        Self {
            positive: value.positive,
            digits: value.digits.borrow_or_share().into(),
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub struct DecodeIntError {
    pub kind: IntErrorKind,
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
        impl<Digits> TryFrom<&Int<Digits>> for $Int where Digits: Bos<[u8]> {
            type Error = DecodeIntError;
            fn try_from(int: &Int<Digits>) -> Result<$Int, Self::Error> {
                if !int.positive {
                    return Err(DecodeIntError { kind: IntErrorKind::NegOverflow });
                }
                int.digits().parse::<$Int>().map_err(From::from)
            }
        }

        impl From<$Int> for Int<Vec<u8>> {
            fn from(val: $Int) -> Self {
                #[expect(unsafe_code)]
                unsafe { Self::new(true, val.to_string().into_bytes()) }
            }
        }
        )+
    };
}

macro_rules! impl_int {
    ($($UInt:ty => $Int:ty),+) => {
        $(
            impl<Digits> TryFrom<&Int<Digits>> for $Int where Digits: Bos<[u8]> {
                type Error = DecodeIntError;
                fn try_from(int: &Int<Digits>) -> Result<$Int, Self::Error> {
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

            impl From<$Int> for Int<Vec<u8>> {
                fn from(val: $Int) -> Self {
                    #[expect(unsafe_code)]
                    unsafe { Self::new(val.is_positive(), val.unsigned_abs().to_string().into_bytes()) }
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
