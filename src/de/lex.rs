use std::{borrow::Cow, fmt::Write, hash::Hash};

use borrow_or_share::{BorrowOrShare, Bos};
use nom::{IResult, Parser};

mod literal;
pub use literal::*;

mod collection;
pub use collection::*;

use crate::de::{Decode, DecodeError, SyrupKind};

#[cfg(test)]
mod test;

#[derive(Clone)]
pub enum TokenTree<Data> {
    Dictionary(Dictionary<Data>),
    List(List<Data>),
    Record(Box<Record<Data>>),
    Set(Set<Data>),
    Literal(Literal<Data>),
}

impl<LData, RData> PartialEq<TokenTree<RData>> for TokenTree<LData>
where
    LData: PartialEq<RData> + Bos<[u8]>,
    RData: Bos<[u8]>,
{
    fn eq(&self, other: &TokenTree<RData>) -> bool {
        match (self, other) {
            (TokenTree::Dictionary(l0), TokenTree::Dictionary(r0)) => l0 == r0,
            (TokenTree::List(l0), TokenTree::List(r0)) => l0 == r0,
            (TokenTree::Record(l0), TokenTree::Record(r0)) => **l0 == **r0,
            (TokenTree::Set(l0), TokenTree::Set(r0)) => l0 == r0,
            (TokenTree::Literal(l0), TokenTree::Literal(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl<Data> Eq for TokenTree<Data> where Data: Eq + Bos<[u8]> {}

impl<Data> Hash for TokenTree<Data>
where
    Data: Bos<[u8]>,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_bytes().hash(state);
    }
}

impl<Data> std::fmt::Debug for TokenTree<Data>
where
    Data: Bos<[u8]>,
{
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

impl<Data> TokenTree<Data> {
    /// Encode this token tree as syrup data
    pub fn to_bytes<'i, 'o>(&'i self) -> Cow<'o, [u8]>
    where
        Data: BorrowOrShare<'i, 'o, [u8]>,
    {
        match self {
            TokenTree::Dictionary(d) => Cow::Owned(d.encode()),
            TokenTree::List(l) => Cow::Owned(l.encode()),
            TokenTree::Record(r) => Cow::Owned(r.encode()),
            TokenTree::Set(s) => Cow::Owned(s.encode()),
            TokenTree::Literal(l) => l.encode(),
        }
    }

    pub fn write_bytes(&self, w: &mut impl std::io::Write) -> std::io::Result<usize>
    where
        Data: Bos<[u8]>,
    {
        match self {
            TokenTree::Dictionary(d) => d.encode_into(w),
            TokenTree::List(l) => l.encode_into(w),
            TokenTree::Record(r) => r.encode_into(w),
            TokenTree::Set(s) => s.encode_into(w),
            TokenTree::Literal(l) => l.encode_into(w),
        }
    }

    pub fn parse<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Self, E>
    where
        &'i [u8]: Into<Data>,
    {
        Literal::parse
            .map(Self::Literal)
            .or(List::parse.map(Self::List))
            .or(Record::parse.map(|rec| Self::Record(Box::new(rec))))
            .or(Set::parse.map(Self::Set))
            .or(Dictionary::parse.map(Self::Dictionary))
            .parse(i)
    }

    #[inline]
    pub fn decode<'input, Output: Decode<'input, Data>>(
        &'input self,
    ) -> Result<Output, DecodeError> {
        Output::decode(self)
    }

    pub fn kind(&self) -> SyrupKind
    where
        Data: Bos<[u8]>,
    {
        match self {
            TokenTree::Dictionary(_) => SyrupKind::Dictionary,
            TokenTree::List(list) => SyrupKind::List {
                length: Some(list.elements.len()),
            },
            TokenTree::Record(_) => SyrupKind::Record { label: None },
            TokenTree::Set(_) => SyrupKind::Set,
            TokenTree::Literal(literal) => literal.kind(),
        }
    }
}

impl<'i, 'o, IData, OData> From<&'i TokenTree<IData>> for TokenTree<OData>
where
    IData: BorrowOrShare<'i, 'o, [u8]>,
    &'o [u8]: Into<OData>,
{
    fn from(value: &'i TokenTree<IData>) -> Self {
        match value {
            TokenTree::Dictionary(dictionary) => Self::Dictionary(dictionary.into()),
            TokenTree::List(list) => Self::List(list.into()),
            TokenTree::Record(record) => Self::Record(Box::new((&**record).into())),
            TokenTree::Set(set) => Self::Set(set.into()),
            TokenTree::Literal(literal) => Self::Literal(literal.into()),
        }
    }
}
