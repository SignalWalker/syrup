use proptest::prelude::*;

type E<'i> = nom::error::Error<&'i [u8]>;

use crate::de::lex::{Dictionary, List, Record, Set};

proptest! {
    #[test]
    fn parses_list(list: List) {
        let bytes = list.encode();
        let res = List::parse::<E<'_>>(&bytes);
        prop_assert!(res.is_ok(), "encoded input: `{}`, error: {}", String::from_utf8_lossy(&bytes), res.unwrap_err());
        let (rem, res) = res.unwrap();
        prop_assert_eq!(rem, [].as_slice());
        prop_assert_eq!(res, list);
    }

    #[test]
    fn parses_record(rec: Record) {
        let bytes = rec.encode();
        let res = Record::parse::<E<'_>>(&bytes);
        prop_assert!(res.is_ok(), "encoded input: `{}`, error: {}", String::from_utf8_lossy(&bytes), res.unwrap_err());
        let (rem, res) = res.unwrap();
        prop_assert_eq!(rem, [].as_slice());
        prop_assert_eq!(res, rec);
    }

    #[test]
    fn parses_set(set: Set) {
        let bytes = set.encode();
        let res = Set::parse::<E<'_>>(&bytes);
        prop_assert!(res.is_ok(), "encoded input: `{}`, error: {}", String::from_utf8_lossy(&bytes), res.unwrap_err());
        let (rem, res) = res.unwrap();
        prop_assert_eq!(rem, [].as_slice());
        prop_assert_eq!(res, set);
    }

    #[test]
    fn parses_dictionary(dict: Dictionary) {
        let bytes = dict.encode();
        let res = Dictionary::parse::<E<'_>>(&bytes);
        prop_assert!(res.is_ok(), "encoded input: `{}`, error: {}", String::from_utf8_lossy(&bytes), res.unwrap_err());
        let (rem, res) = res.unwrap();
        prop_assert_eq!(rem, [].as_slice());
        prop_assert_eq!(res, dict);
    }
}
