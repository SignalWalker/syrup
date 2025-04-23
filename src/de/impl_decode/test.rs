use proptest::prelude::*;
use proptest::{sample::SizeRange, test_runner::TestCaseResult};

use crate::{
    de::{DecodeBytesError, DecodeFromBytes},
    symbol::Symbol,
};

#[expect(unsafe_code)]
fn assert_correct_decode<'i, Output, Expected>(
    input: &'i [u8],
    res: Result<(&'i [u8], Output), DecodeBytesError<'i>>,
    expected: Expected,
) -> TestCaseResult
where
    Output: std::fmt::Debug + PartialEq<Expected>,
    Expected: std::fmt::Debug,
{
    prop_assert!(
        res.is_ok(),
        "failed to decode input; input: {}, error: {}",
        String::from_utf8_lossy(input),
        unsafe { res.unwrap_err_unchecked() }
    );
    #[expect(unsafe_code)]
    let (rem, res) = unsafe { res.unwrap_unchecked() };
    prop_assert!(
        rem.is_empty(),
        "failed to consume all input; input: {}, remaining: {}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(rem)
    );
    prop_assert_eq!(res, expected);
    Ok(())
}

#[test]
fn decodes_bools() -> Result<(), DecodeBytesError<'static>> {
    assert_eq!(bool::decode_bytes(b"t")?, ([].as_slice(), true));
    assert_eq!(bool::decode_bytes(b"f")?, ([].as_slice(), false));
    Ok(())
}

proptest! {
    #[test]
    fn decodes_byte_strings(bytes in proptest::collection::vec(proptest::num::u8::ANY, SizeRange::default())) {
        let mut input = bytes.len().to_string().into_bytes();
        input.reserve_exact(1 + bytes.len());
        input.push(b':');
        input.extend(&bytes);
        let res = crate::bytes::decode_bytes::<&[u8]>(&input);
        assert_correct_decode(&input, res, bytes)?;
    }

    #[test]
    fn decodes_strings(s in String::arbitrary()) {
        let mut input = s.len().to_string().into_bytes();
        input.reserve_exact(1 + s.len());
        input.push(b'"');
        input.extend(s.as_bytes());
        let res = crate::decode_bytes!(&input => &str);
        assert_correct_decode(&input, res, s)?;
    }

    #[test]
    fn decodes_symbols(s in String::arbitrary()) {
        let mut input = s.len().to_string().into_bytes();
        input.reserve_exact(1 + s.len());
        input.push(b'\'');
        input.extend(s.as_bytes());
        let res = crate::decode_bytes!(&input => Symbol<&str>);
        assert_correct_decode(&input, res, Symbol(s.as_str()))?;
    }

    // TODO :: test decoding to collections (vec/hashset/btreeset/hashmap/btreemap)

}

// TODO :: test <[T; LEN]>::decode()

mod int {
    use std::num::{
        NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize, NonZeroU8,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
    };

    use crate::de::{DecodeFromBytes, impl_decode::test::assert_correct_decode};

    #[inline]
    fn decodes_int<'i, Int: DecodeFromBytes<'i> + std::fmt::Debug + PartialEq + Eq>(
        original: Int,
        digits: &'i [u8],
    ) -> Result<(), proptest::prelude::TestCaseError> {
        let res = Int::decode_bytes(digits);
        super::assert_correct_decode(digits, res, original)
    }

    macro_rules! decodes_int {
        ($($Int:ty => $test_name:ident),+$(,)?) => {
            proptest::proptest! {
                $(
                    #[test]
                    fn $test_name(i: $Int) {
                        let mut digits = i.unsigned_abs().to_string().into_bytes();
                        digits.push(if i.is_positive() { b'+' } else { b'-' });
                        decodes_int::<$Int>(i, &digits)?;
                    }
                )+
            }
        };
    }

    macro_rules! decodes_uint {
        ($($Int:ty => $test_name:ident),+$(,)?) => {
            proptest::proptest! {
                $(
                    #[test]
                    fn $test_name(i: $Int) {
                        let mut digits = i.to_string().into_bytes();
                        digits.push(b'+');
                        decodes_int::<$Int>(i, &digits)?;
                    }
                )+
            }
        };
    }

    proptest::proptest! {
        #[test]
        fn decodes_f32(f: f32) {
            let mut bytes = Vec::with_capacity(5);
            bytes.push(b'F');
            bytes.extend(f.to_be_bytes());
            let res = f32::decode_bytes(&bytes);
            assert_correct_decode(&bytes, res, f)?;
        }

        #[test]
        fn decodes_f64(d: f64) {
            let mut bytes = Vec::with_capacity(9);
            bytes.push(b'D');
            bytes.extend(d.to_be_bytes());
            let res = f64::decode_bytes(&bytes);
            assert_correct_decode(&bytes, res, d)?;
        }
    }

    decodes_uint!(
        u8 => decodes_u8,
        u16 => decodes_u16,
        u32 => decodes_u32,
        u64 => decodes_u64,
        usize => decodes_usize,
        u128 => decodes_u128,
        NonZeroU8 => decodes_non_zero_u8,
        NonZeroU16 => decodes_non_zero_u16,
        NonZeroU32 => decodes_non_zero_u32,
        NonZeroU64 => decodes_non_zero_u64,
        NonZeroUsize => decodes_non_zero_usize,
        NonZeroU128 => decodes_non_zero_u128,
    );

    decodes_int! {
        i8 => decodes_i8,
        i16 => decodes_i16,
        i32 => decodes_i32,
        i64 => decodes_i64,
        isize => decodes_isize,
        i128 => decodes_i128,
        NonZeroI8 => decodes_non_zero_i8,
        NonZeroI16 => decodes_non_zero_i16,
        NonZeroI32 => decodes_non_zero_i32,
        NonZeroI64 => decodes_non_zero_i64,
        NonZeroIsize => decodes_non_zero_isize,
        NonZeroI128 => decodes_non_zero_i128
    }
}
