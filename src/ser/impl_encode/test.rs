use proptest::prelude::*;

use crate::ser::EncodeExt;

#[test]
fn encodes_bools() {
    assert_eq!(true.encode_bytes(), b"t");
    assert_eq!(false.encode_bytes(), b"f");
}

#[allow(edition_2024_expr_fragment_specifier)]
macro_rules! prop_assert_eq_utf8 {
    ($left:expr, $right:expr $(,)?) => {{
        let (left, right) = ($left, $right);
        prop_assert_eq!(
            &left,
            &right,
            "left: {}, right: {}",
            String::from_utf8_lossy(&left),
            String::from_utf8_lossy(&right)
        );
    }}; // ($left:expr, $right:expr, $($arg:tt)+) => {
        //
        // };
}

proptest! {
    #[test]
    fn encodes_byte_strings(s: Vec<u8>) {
        let len_digits = s.len().to_string().into_bytes();
        let mut expected = Vec::with_capacity(len_digits.len() + 1 + s.len());
        expected.extend_from_slice(&len_digits);
        expected.push(b':');
        expected.extend_from_slice(&s);
        prop_assert_eq_utf8!(crate::bytes::encode_bytes(&s), expected);
    }

    #[test]
    fn encodes_strings(s: String) {
        let len_digits = s.len().to_string().into_bytes();
        let mut expected = Vec::with_capacity(len_digits.len() + 1 + s.len());
        expected.extend_from_slice(&len_digits);
        expected.push(b'"');
        expected.extend_from_slice(s.as_bytes());
        prop_assert_eq_utf8!(s.encode_bytes(), expected);
    }

    #[test]
    fn encodes_symbols(s: String) {
        let len_digits = s.len().to_string().into_bytes();
        let mut expected = Vec::with_capacity(len_digits.len() + 1 + s.len());
        expected.extend_from_slice(&len_digits);
        expected.push(b'\'');
        expected.extend_from_slice(s.as_bytes());
        prop_assert_eq_utf8!(crate::symbol::encode_bytes(&s), expected);
    }

    // TODO :: test encoding collections
}

mod int {
    use proptest::prelude::*;

    use crate::ser::EncodeExt;

    proptest! {
        #[test]
        fn encodes_f32(f: f32) {
            let mut expected = Vec::with_capacity(5);
            expected.push(b'F');
            expected.extend_from_slice(&f.to_be_bytes());
            prop_assert_eq!(f.encode_bytes(), expected);
        }

        #[test]
        fn encodes_f64(d: f64) {
            let mut expected = Vec::with_capacity(9);
            expected.push(b'D');
            expected.extend_from_slice(&d.to_be_bytes());
            prop_assert_eq!(d.encode_bytes(), expected);
        }
    }

    macro_rules! encodes_uint {
        ($($Int:ty => $test_name:ident),+$(,)?) => {
            proptest::proptest! {
                $(
                    #[test]
                    fn $test_name(i: $Int) {
                        let mut expected = i.to_string().into_bytes();
                        expected.push(b'+');
                        prop_assert_eq!(i.encode_bytes(), expected);
                    }
                )+
            }
        };
    }

    macro_rules! encodes_int {
        ($($Int:ty => $test_name:ident),+$(,)?) => {
            proptest::proptest! {
                $(
                    #[test]
                    fn $test_name(i: $Int) {
                        let mut expected = i.unsigned_abs().to_string().into_bytes();
                        expected.push(if i.is_positive() { b'+' } else { b'-' });
                        prop_assert_eq!(i.encode_bytes(), expected);
                    }
                )+
            }
        };
    }

    encodes_uint!(
        u8 => encodes_u8,
        u16 => encodes_u16,
        u32 => encodes_u32,
        u64 => encodes_u64,
        usize => encodes_usize,
        u128 => encodes_u128,
        std::num::NonZeroU8 => encodes_non_zero_u8,
        std::num::NonZeroU16 => encodes_non_zero_u16,
        std::num::NonZeroU32 => encodes_non_zero_u32,
        std::num::NonZeroU64 => encodes_non_zero_u64,
        std::num::NonZeroUsize => encodes_non_zero_usize,
        std::num::NonZeroU128 => encodes_non_zero_u128,
    );

    encodes_int! {
        i8 => encodes_i8,
        i16 => encodes_i16,
        i32 => encodes_i32,
        i64 => encodes_i64,
        isize => encodes_isize,
        i128 => encodes_i128,
        std::num::NonZeroI8 => encodes_non_zero_i8,
        std::num::NonZeroI16 => encodes_non_zero_i16,
        std::num::NonZeroI32 => encodes_non_zero_i32,
        std::num::NonZeroI64 => encodes_non_zero_i64,
        std::num::NonZeroIsize => encodes_non_zero_isize,
        std::num::NonZeroI128 => encodes_non_zero_i128
    }
}
