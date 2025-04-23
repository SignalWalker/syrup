use borrow_or_share::Bos;
use proptest::prelude::*;
use proptest::{
    prelude::{BoxedStrategy, Strategy},
    prop_oneof,
};

use crate::de::lex::{Dictionary, List, Literal, Record, Set, TokenTree};

type E<'i> = nom::error::Error<&'i [u8]>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct MaxDepth(pub usize);

impl MaxDepth {
    #[inline]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_sub(1))
    }
}

impl Default for MaxDepth {
    fn default() -> Self {
        Self(1)
    }
}

impl proptest::prelude::Arbitrary for TokenTree<Vec<u8>> {
    type Parameters = MaxDepth;

    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(max_depth: Self::Parameters) -> Self::Strategy {
        if max_depth.0 == 0 {
            Literal::arbitrary().prop_map(Self::Literal).boxed()
        } else {
            let next_depth = max_depth.next();
            prop_oneof![
                Dictionary::arbitrary_with(next_depth).prop_map(TokenTree::Dictionary),
                List::arbitrary_with(next_depth).prop_map(TokenTree::List),
                Record::arbitrary_with(next_depth).prop_map(|rec| TokenTree::Record(Box::new(rec))),
                Set::arbitrary_with(next_depth).prop_map(TokenTree::Set),
                Literal::arbitrary().prop_map(TokenTree::Literal)
            ]
            .boxed()
        }
    }
}

proptest! {
    #[test]
    fn parses_token_tree(tree in TokenTree::arbitrary_with(MaxDepth(2))) {
        let bytes = tree.to_bytes();
        let res = TokenTree::<&[u8]>::parse::<E<'_>>(&bytes);
        prop_assert!(res.is_ok(), "encoded input: `{}`, error: {}", String::from_utf8_lossy(&bytes), res.unwrap_err());
        let (rem, res) = res.unwrap();
        prop_assert_eq!(rem, [].as_slice());
        prop_assert_eq!(&res, &tree);
    }

    // TODO :: make this run faster
    #[test]
    fn parses_incomplete(tokens in proptest::collection::vec(TokenTree::arbitrary(), 1..=3)) {
        fn encode_tokens<Data>(tokens: &[TokenTree<Data>]) -> Vec<u8> where Data: Bos<[u8]> {
            let mut bytes = Vec::new();
            for token in tokens {
                bytes.extend_from_slice(&token.to_bytes());
            }
            bytes
        }
        let bytes = encode_tokens(&tokens);
        prop_assume!(!bytes.is_empty());

        let mut res = Vec::with_capacity(tokens.len());
        let mut start = 0;
        let mut window = bytes.get(start..1).expect("encoded tokens should be longer than 1 byte");
        loop {
            match TokenTree::<&[u8]>::parse::<E<'_>>(window) {
                Ok((rem, token)) => {
                    start = start + window.len() - rem.len();
                    window = rem;
                    res.push(token);
                    if start >= bytes.len() {
                        break
                    }
                },
                Err(nom::Err::Incomplete(needed)) => {
                    let needed = match needed {
                        nom::Needed::Unknown => 1,
                        nom::Needed::Size(non_zero) => non_zero.into(),
                    };
                    let end = start + window.len() + needed;
                    let next = if end > bytes.len() { bytes.get(start..) } else { bytes.get(start..(start + window.len() + needed)) };
                    prop_assert!(next.is_some(), "encoded input: `{}`, window: `{}`, start: {start}, decoded: {res:?}", String::from_utf8_lossy(&bytes), String::from_utf8_lossy(window));
                    window = next.unwrap();
                },
                Err(e) => {
                    prop_assert!(false, "encoded input: `{}`, window: `{}`, error: {}", String::from_utf8_lossy(&bytes), String::from_utf8_lossy(window), e);
                }
            }
        }
        prop_assert_eq!(tokens, res);
    }
}
