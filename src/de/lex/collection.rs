use std::collections::{HashMap, HashSet};

use nom::{
    multi::{many, many0},
    sequence::delimited,
    IResult, Parser,
};

use crate::de::lex::{byte, ParseLiteralError, TokenTree};

#[cfg(test)]
mod test;

#[derive(PartialEq, Eq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(test, proptest(params = "crate::de::lex::test::MaxDepth"))]
pub struct List {
    #[cfg_attr(
        test,
        proptest(
            strategy = "proptest::collection::vec(TokenTree::arbitrary_with(params.next()), proptest::collection::SizeRange::default())"
        )
    )]
    pub elements: Vec<TokenTree>,
}

impl List {
    #[inline]
    pub const fn new(elements: Vec<TokenTree>) -> Self {
        Self { elements }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut res = Vec::new();
        res.push(b'[');
        for element in &self.elements {
            res.extend(element.to_bytes());
        }
        res.push(b']');
        res
    }

    pub fn parse<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Self, E> {
        delimited(byte(b'['), many0(TokenTree::parse), byte(b']'))
            .map(List::new)
            .parse(i)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(test, proptest(params = "crate::de::lex::test::MaxDepth"))]
pub struct Record {
    #[cfg_attr(test, proptest(strategy = "TokenTree::arbitrary_with(params.next())"))]
    pub label: TokenTree,
    #[cfg_attr(
        test,
        proptest(
            strategy = "proptest::collection::vec(TokenTree::arbitrary_with(params.next()), proptest::collection::SizeRange::default())"
        )
    )]
    pub elements: Vec<TokenTree>,
}

impl Record {
    #[inline]
    pub const fn new(label: TokenTree, elements: Vec<TokenTree>) -> Self {
        Self { label, elements }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut res = Vec::new();
        res.push(b'<');
        res.extend(self.label.to_bytes());
        for element in &self.elements {
            res.extend(element.to_bytes());
        }
        res.push(b'>');
        res
    }

    pub fn parse<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Self, E> {
        delimited(
            byte(b'<'),
            TokenTree::parse.and(many0(TokenTree::parse)),
            byte(b'>'),
        )
        .map(|(label, elements)| Record::new(label, elements))
        .parse(i)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(test, proptest(params = "crate::de::lex::test::MaxDepth"))]
pub struct Set {
    #[cfg_attr(
        test,
        proptest(
            strategy = "proptest::collection::hash_set(TokenTree::arbitrary_with(params.next()), proptest::collection::SizeRange::default())"
        )
    )]
    entries: HashSet<TokenTree>,
}

impl Set {
    #[inline]
    pub const fn new(entries: HashSet<TokenTree>) -> Self {
        Self { entries }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut sorted = Vec::with_capacity(self.entries.len());
        let mut total_bytes = 2; // NOTE :: starting at 2 because we need at least 2 bytes for the
                                 // `#` and `$`
        for entry in &self.entries {
            let bytes = entry.to_bytes();
            total_bytes += bytes.len();
            sorted.insert(sorted.partition_point(|oentry| oentry <= &bytes), bytes);
        }

        let mut res = Vec::with_capacity(total_bytes);
        res.push(b'#');
        for entry in sorted {
            res.extend_from_slice(&entry);
        }
        res.push(b'$');
        res
    }

    pub fn parse<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Self, E> {
        delimited(byte(b'#'), many(0.., TokenTree::parse), byte(b'$'))
            .map(Self::new)
            .parse(i)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<'s> IntoIterator for &'s Set {
    type Item = <&'s HashSet<TokenTree> as IntoIterator>::Item;

    type IntoIter = <&'s HashSet<TokenTree> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(test, proptest(params = "crate::de::lex::test::MaxDepth"))]
pub struct Dictionary {
    #[cfg_attr(
        test,
        proptest(
            strategy = "proptest::collection::hash_map(TokenTree::arbitrary_with(params.next()), TokenTree::arbitrary_with(params.next()), proptest::collection::SizeRange::default())"
        )
    )]
    entries: HashMap<TokenTree, TokenTree>,
}

impl Dictionary {
    #[inline]
    pub const fn new(entries: HashMap<TokenTree, TokenTree>) -> Self {
        Self { entries }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut pairs = Vec::with_capacity(self.entries.len());
        let mut total_bytes = 2; // NOTE :: starting at 2 because we need at least 2 bytes for the
                                 // opening and closing braces
        for (key, value) in &self.entries {
            let (key, value) = (key.to_bytes(), value.to_bytes());
            total_bytes += key.len() + value.len();
            pairs.insert(
                pairs.partition_point(|(okey, _)| okey <= &key),
                (key, value),
            );
        }

        let mut res = Vec::with_capacity(total_bytes);
        res.push(b'{');
        for (key, value) in pairs {
            res.extend_from_slice(&key);
            res.extend_from_slice(&value);
        }
        res.push(b'}');
        res
    }

    pub fn parse<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Self, E> {
        delimited(
            byte(b'{'),
            many(0.., TokenTree::parse.and(TokenTree::parse)),
            byte(b'}'),
        )
        .map(Self::new)
        .parse(i)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<'s> IntoIterator for &'s Dictionary {
    type Item = <&'s HashMap<TokenTree, TokenTree> as IntoIterator>::Item;

    type IntoIter = <&'s HashMap<TokenTree, TokenTree> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}
