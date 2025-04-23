use proptest::prelude::*;

type E<'i> = nom::error::Error<&'i [u8]>;

use crate::{
    TokenTree,
    de::lex::{Dictionary, List, Record, Set, test::MaxDepth},
};

impl Arbitrary for List<Vec<u8>> {
    type Parameters = MaxDepth;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        Vec::<TokenTree<Vec<u8>>>::arbitrary_with((Default::default(), args.next()))
            .prop_map(|elements| List { elements })
            .boxed()
    }
}

impl Arbitrary for Record<Vec<u8>> {
    type Parameters = MaxDepth;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        <(TokenTree<Vec<u8>>, Vec<TokenTree<Vec<u8>>>)>::arbitrary_with((
            args.next(),
            (Default::default(), args.next()),
        ))
        .prop_map(|(label, elements)| Record { label, elements })
        .boxed()
    }
}

impl Arbitrary for Set<Vec<u8>> {
    type Parameters = MaxDepth;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        Vec::<TokenTree<Vec<u8>>>::arbitrary_with((Default::default(), args.next()))
            .prop_map(|entries| Set { entries })
            .boxed()
    }
}

impl Arbitrary for Dictionary<Vec<u8>> {
    type Parameters = MaxDepth;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        Vec::<(TokenTree<Vec<u8>>, TokenTree<Vec<u8>>)>::arbitrary_with((
            Default::default(),
            (args.next(), args.next()),
        ))
        .prop_map(|entries| Dictionary { entries })
        .boxed()
    }
}

proptest! {
    #[test]
    fn parses_list(list: List<Vec<u8>>) {
        let bytes = list.encode();
        let res = List::<&[u8]>::parse::<E<'_>>(&bytes);
        prop_assert!(res.is_ok(), "encoded input: `{}`, error: {}", String::from_utf8_lossy(&bytes), res.unwrap_err());
        let (rem, res) = res.unwrap();
        prop_assert_eq!(rem, [].as_slice());
        prop_assert_eq!(res, list);
    }

    #[test]
    fn parses_record(rec: Record<Vec<u8>>) {
        let bytes = rec.encode();
        let res = Record::<&[u8]>::parse::<E<'_>>(&bytes);
        prop_assert!(res.is_ok(), "encoded input: `{}`, error: {}", String::from_utf8_lossy(&bytes), res.unwrap_err());
        let (rem, res) = res.unwrap();
        prop_assert_eq!(rem, [].as_slice());
        prop_assert_eq!(res, rec);
    }

    #[test]
    fn parses_set(set: Set<Vec<u8>>) {
        let bytes = set.encode();
        let res = Set::<&[u8]>::parse::<E<'_>>(&bytes);
        prop_assert!(res.is_ok(), "encoded input: `{}`, error: {}", String::from_utf8_lossy(&bytes), res.unwrap_err());
        let (rem, res) = res.unwrap();
        prop_assert_eq!(rem, [].as_slice());
        prop_assert_eq!(res, set);
    }

    #[test]
    fn parses_dictionary(dict: Dictionary<Vec<u8>>) {
        let bytes = dict.encode();
        let res = Dictionary::<&[u8]>::parse::<E<'_>>(&bytes);
        prop_assert!(res.is_ok(), "encoded input: `{}`, error: {}", String::from_utf8_lossy(&bytes), res.unwrap_err());
        let (rem, res) = res.unwrap();
        prop_assert_eq!(rem, [].as_slice());
        prop_assert_eq!(res, dict);
    }
}
