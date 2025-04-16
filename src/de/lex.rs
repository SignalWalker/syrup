use std::{fmt::Write, hash::Hash};

use nom::{IResult, Parser};

mod literal;
pub use literal::*;

mod collection;
pub use collection::*;

use crate::de::{Decode, DecodeError};

#[cfg(test)]
mod test;

#[derive(PartialEq, Eq, Clone)]
pub enum TokenTree {
    Dictionary(Dictionary),
    List(List),
    Record(Box<Record>),
    Set(Set),
    Literal(Literal),
}

impl std::fmt::Debug for TokenTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TT(")?;
        match self {
            TokenTree::Dictionary(d) => d.fmt(f)?,
            TokenTree::List(l) => l.fmt(f)?,
            TokenTree::Record(r) => r.fmt(f)?,
            TokenTree::Set(s) => s.fmt(f)?,
            TokenTree::Literal(l) => l.fmt(f)?,
        }
        f.write_char(')')
    }
}

impl PartialOrd for TokenTree {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TokenTree {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_bytes().cmp(&other.to_bytes())
    }
}

impl Hash for TokenTree {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_bytes().hash(state);
    }
}

impl TokenTree {
    /// Encode this token tree as syrup data
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            TokenTree::Dictionary(d) => d.encode(),
            TokenTree::List(l) => l.encode(),
            TokenTree::Record(r) => r.encode(),
            TokenTree::Set(s) => s.encode(),
            TokenTree::Literal(l) => l.encode(),
        }
    }

    pub fn parse<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Self, E> {
        Literal::parse
            .map(Self::Literal)
            .or(List::parse.map(Self::List))
            .or(Record::parse.map(|rec| Self::Record(Box::new(rec))))
            .or(Set::parse.map(Self::Set))
            .or(Dictionary::parse.map(Self::Dictionary))
            .parse(i)
    }

    #[inline]
    pub fn decode<'input, 'error, Output: Decode<'input>>(
        &'input self,
    ) -> Result<Output, DecodeError<'error>> {
        Output::decode(self)
    }
}
