use nom::Parser;
use proptest::prelude::*;
use proptest::{sample::SizeRange, string::StringParam};

use crate::de::{Int, Literal};

type E<'i> = nom::error::Error<&'i [u8]>;

macro_rules! parses_sized_literal {
    ($SEP:expr, $parse:path, $bytes:expr) => {
        let mut input = $bytes.len().to_string().into_bytes();
        input.reserve_exact(1 + $bytes.len());
        input.push($SEP);
        input.extend($bytes);
        let res = $parse(&input);
        prop_assert!(
            res.is_ok(),
            "parse sized literal failed for '{}' with {}",
            String::from_utf8_lossy(&input),
            res.unwrap_err()
        );
        let (rem, res) = res.unwrap();
        prop_assert!(rem.is_empty());
        prop_assert_eq!($bytes, res);
    };
}

impl proptest::prelude::Arbitrary for Int<Vec<u8>> {
    type Parameters = ();

    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        // TODO :: numbers larger than can be represented with an i128?
        i128::arbitrary_with(()).prop_map_into().boxed()
    }
}

impl proptest::prelude::Arbitrary for Literal<Vec<u8>> {
    type Parameters = (SizeRange, StringParam);
    type Strategy = BoxedStrategy<Literal<Vec<u8>>>;

    fn arbitrary_with((size_range, string_param): Self::Parameters) -> Self::Strategy {
        prop_oneof![
            bool::arbitrary().prop_map(Literal::Bool),
            f32::arbitrary().prop_map(Literal::F32),
            f64::arbitrary().prop_map(Literal::F64),
            Int::<Vec<u8>>::arbitrary().prop_map(Literal::Int),
            Vec::<u8>::arbitrary_with((size_range, ())).prop_map(Literal::Bytes),
            String::arbitrary_with(string_param).prop_map(|s| Literal::String(s.into_bytes())),
            String::arbitrary_with(string_param).prop_map(|s| Literal::Symbol(s.into_bytes())),
        ]
        .boxed()
    }
}

proptest! {
    #[test]
    fn parses_byte(b in proptest::num::u8::ANY) {
        let input = [b];
        let res = super::byte::<E<'_>>(b).parse_complete(input.as_slice());
        prop_assert_eq!(res, Ok(([].as_slice(), ())));
    }

    #[test]
    fn parses_bools(b in proptest::bool::ANY) {
        let c = if b { b't' } else { b'f' };
        let bytes = [c];
        prop_assert_eq!(super::bool_literal::<E<'_>>(&bytes), Ok(([].as_slice(), b)));
    }

    #[test]
    fn parses_f32s(f in proptest::num::f32::ANY) {
        let mut bytes = Vec::<u8>::with_capacity(5);
        bytes.push(b'F');
        bytes.extend(f.to_be_bytes());
        let res = super::f32_literal::<E<'_>>(&bytes);
        prop_assert!(res.is_ok());
        let (rem, res) = res.unwrap();
        prop_assert_eq!(rem, [].as_slice());
        prop_assert_eq!(res.to_be_bytes(), f.to_be_bytes());
    }

    #[test]
    fn parses_f64s(f in proptest::num::f64::ANY) {
        let mut bytes = Vec::<u8>::with_capacity(9);
        bytes.push(b'D');
        bytes.extend(f.to_be_bytes());
        let res = super::f64_literal::<E<'_>>(&bytes);
        prop_assert!(res.is_ok());
        let (rem, res) = res.unwrap();
        prop_assert_eq!(rem, [].as_slice());
        prop_assert_eq!(res.to_be_bytes(), f.to_be_bytes());
    }

    #[test]
    fn parses_ints(num: Int<Vec<u8>>) {
        let input = num.encode();
        let res = super::int_literal::<&[u8], E<'_>>(&input);
        prop_assert!(res.is_ok());
        let (rem, res) = res.unwrap();
        prop_assert_eq!(rem, [].as_slice());
        prop_assert_eq!(&res, &num);
    }

    #[test]
    fn parses_bytes(bytes in proptest::collection::vec(proptest::num::u8::ANY, SizeRange::default())) {
        parses_sized_literal!(b':', super::bytes_literal::<&[u8], E<'_>>, &bytes);
    }

    #[test]
    fn parses_strings(input in String::arbitrary()) {
        let bytes = input.as_bytes();
        parses_sized_literal!(b'"', super::string_literal::<&[u8], E<'_>>, bytes);
    }

    #[test]
    fn parses_symbols(input in String::arbitrary()) {
        let bytes = input.as_bytes();
        parses_sized_literal!(b'\'', super::symbol_literal::<&[u8], E<'_>>, bytes);
    }

    #[test]
    fn parses_literals(literal: super::Literal<Vec<u8>>) {
        let input = literal.encode();
        let res = super::Literal::<&[u8]>::parse::<E<'_>>(&input);
        prop_assert!(res.is_ok(), "encoded input: `{}`, error: {}", String::from_utf8_lossy(&input), res.unwrap_err());
        let (rem, res) = res.unwrap();
        prop_assert_eq!(rem, [].as_slice());
        prop_assert_eq!(&res, &literal);
    }
}
