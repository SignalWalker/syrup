use std::borrow::Cow;

use borrow_or_share::{BorrowOrShare, Bos};
use nom::{
    IResult, Parser,
    multi::{many, many0},
    sequence::delimited,
};

use crate::de::lex::{ParseLiteralError, TokenTree, byte};

#[cfg(test)]
mod test;

#[derive(Clone)]
// #[cfg_attr(test, derive(proptest_derive::Arbitrary))]
// #[cfg_attr(test, proptest(params = "crate::de::lex::test::MaxDepth"))]
pub struct List<Data> {
    // #[cfg_attr(
    //     test,
    //     proptest(
    //         strategy = "proptest::collection::vec(TokenTree::arbitrary_with(params.next()), proptest::collection::SizeRange::default())"
    //     )
    // )]
    pub elements: Vec<TokenTree<Data>>,
}

impl<'i, 'o, IData, OData> From<&'i List<IData>> for List<OData>
where
    IData: BorrowOrShare<'i, 'o, [u8]>,
    &'o [u8]: Into<OData>,
{
    fn from(value: &'i List<IData>) -> Self {
        Self {
            elements: value.elements.iter().map(|el| el.into()).collect(),
        }
    }
}

impl<Data> std::fmt::Debug for List<Data>
where
    Data: Bos<[u8]>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("List")
            .field("elements", &self.elements)
            .finish()
    }
}

impl<LData, RData> PartialEq<List<RData>> for List<LData>
where
    LData: PartialEq<RData> + Bos<[u8]>,
    RData: Bos<[u8]>,
{
    fn eq(&self, other: &List<RData>) -> bool {
        self.elements.eq(&other.elements)
    }
}

impl<Data> Eq for List<Data> where Data: Eq + Bos<[u8]> {}

impl<Data> List<Data> {
    #[inline]
    pub const fn new(elements: Vec<TokenTree<Data>>) -> Self {
        Self { elements }
    }

    pub fn encode(&self) -> Vec<u8>
    where
        Data: Bos<[u8]>,
    {
        let mut res = Vec::new();
        res.push(b'[');
        for element in &self.elements {
            drop(element.write_bytes(&mut res));
        }
        res.push(b']');
        res
    }

    pub fn encode_into(&self, w: &mut impl std::io::Write) -> std::io::Result<usize>
    where
        Data: Bos<[u8]>,
    {
        let mut amt = 2; // starting at 2 because of the []
        w.write_all(b"[")?;
        for element in &self.elements {
            amt += element.write_bytes(w)?;
        }
        w.write_all(b"]")?;
        Ok(amt)
    }

    pub fn parse<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Self, E>
    where
        &'i [u8]: Into<Data>,
    {
        delimited(byte(b'['), many0(TokenTree::<Data>::parse), byte(b']'))
            .map(List::new)
            .parse(i)
    }
}

#[derive(Clone)]
// #[cfg_attr(test, derive(proptest_derive::Arbitrary))]
// #[cfg_attr(test, proptest(params = "crate::de::lex::test::MaxDepth"))]
pub struct Record<Data> {
    // #[cfg_attr(test, proptest(strategy = "TokenTree::arbitrary_with(params.next())"))]
    pub label: TokenTree<Data>,
    // #[cfg_attr(
    //     test,
    //     proptest(
    //         strategy = "proptest::collection::vec(TokenTree::arbitrary_with(params.next()), proptest::collection::SizeRange::default())"
    //     )
    // )]
    pub elements: Vec<TokenTree<Data>>,
}

impl<'i, 'o, IData, OData> From<&'i Record<IData>> for Record<OData>
where
    IData: BorrowOrShare<'i, 'o, [u8]>,
    &'o [u8]: Into<OData>,
{
    fn from(value: &'i Record<IData>) -> Self {
        Self {
            label: (&value.label).into(),
            elements: value.elements.iter().map(|el| el.into()).collect(),
        }
    }
}

impl<Data> std::fmt::Debug for Record<Data>
where
    Data: Bos<[u8]>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Record")
            .field("label", &self.label)
            .field("elements", &self.elements)
            .finish()
    }
}

impl<LData, RData> PartialEq<Record<RData>> for Record<LData>
where
    LData: PartialEq<RData> + Bos<[u8]>,
    RData: Bos<[u8]>,
{
    fn eq(&self, other: &Record<RData>) -> bool {
        self.label.eq(&other.label) && self.elements.eq(&other.elements)
    }
}

impl<Data> Eq for Record<Data> where Data: Eq + Bos<[u8]> {}

impl<Data> Record<Data> {
    #[inline]
    pub const fn new(label: TokenTree<Data>, elements: Vec<TokenTree<Data>>) -> Self {
        Self { label, elements }
    }

    pub fn encode(&self) -> Vec<u8>
    where
        Data: Bos<[u8]>,
    {
        let mut res = Vec::new();
        res.push(b'<');
        drop(self.label.write_bytes(&mut res));
        for element in &self.elements {
            drop(element.write_bytes(&mut res));
        }
        res.push(b'>');
        res
    }

    pub fn encode_into(&self, w: &mut impl std::io::Write) -> std::io::Result<usize>
    where
        Data: Bos<[u8]>,
    {
        let mut amt = 2; // starting at 2 for the <>
        w.write_all(b"<")?;
        amt += self.label.write_bytes(w)?;
        for element in &self.elements {
            amt += element.write_bytes(w)?;
        }
        w.write_all(b">")?;
        Ok(amt)
    }

    pub fn parse<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Self, E>
    where
        &'i [u8]: Into<Data>,
    {
        delimited(
            byte(b'<'),
            TokenTree::parse.and(many0(TokenTree::parse)),
            byte(b'>'),
        )
        .map(|(label, elements)| Record::new(label, elements))
        .parse(i)
    }
}

#[derive(Clone)]
// #[cfg_attr(test, derive(proptest_derive::Arbitrary))]
// #[cfg_attr(test, proptest(params = "crate::de::lex::test::MaxDepth"))]
pub struct Set<Data> {
    // #[cfg_attr(
    //     test,
    //     proptest(
    //         strategy = "proptest::collection::hash_set(TokenTree::arbitrary_with(params.next()), proptest::collection::SizeRange::default())"
    //     )
    // )]
    entries: Vec<TokenTree<Data>>,
}

impl<'i, 'o, IData, OData> From<&'i Set<IData>> for Set<OData>
where
    IData: BorrowOrShare<'i, 'o, [u8]>,
    &'o [u8]: Into<OData>,
{
    fn from(value: &'i Set<IData>) -> Self {
        Self {
            entries: value.entries.iter().map(|el| el.into()).collect(),
        }
    }
}

impl<Data> std::fmt::Debug for Set<Data>
where
    Data: Bos<[u8]>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Set")
            .field("entries", &self.entries)
            .finish()
    }
}

impl<LData, RData> PartialEq<Set<RData>> for Set<LData>
where
    LData: PartialEq<RData> + Bos<[u8]>,
    RData: Bos<[u8]>,
{
    fn eq(&self, other: &Set<RData>) -> bool {
        // TODO :: performance
        self.encode().eq(&other.encode())
    }
}

impl<Data> Eq for Set<Data> where Data: Eq + Bos<[u8]> {}

pub(crate) fn set_encoded_entries<'i, S>(set: S) -> (usize, Vec<Cow<'i, [u8]>>)
where
    S: IntoIterator<Item = Cow<'i, [u8]>>,
    <S as IntoIterator>::IntoIter: ExactSizeIterator,
{
    let entries = set.into_iter();
    let mut sorted = Vec::with_capacity(entries.len());
    let mut total_bytes = 0;
    for entry in entries {
        total_bytes += entry.len();
        let ppoint = sorted.partition_point(|oentry| oentry < &entry);
        if sorted.get(ppoint).is_some() && sorted[ppoint] == entry {
            // duplicate entry; skip
            continue;
        }
        sorted.insert(ppoint, entry);
    }
    (total_bytes, sorted)
}

pub(crate) fn encode_into_as_set<'i>(
    entries: &[Cow<'i, [u8]>],
    w: &mut impl std::io::Write,
) -> std::io::Result<usize> {
    let mut amt = 2; // starting at 2 for the #$
    w.write_all(b"#")?;
    for entry in entries {
        w.write_all(entry)?;
        amt += entry.len();
    }
    w.write_all(b"$")?;
    Ok(amt)
}

impl<Data> Set<Data> {
    /// `entries` should not contain any duplicates; they won't be encoded.
    #[inline]
    pub const fn new(entries: Vec<TokenTree<Data>>) -> Self {
        Self { entries }
    }

    pub fn encode(&self) -> Vec<u8>
    where
        Data: Bos<[u8]>,
    {
        let (total_bytes, sorted) = set_encoded_entries(self.into_iter().map(|e| e.to_bytes()));

        let mut res = Vec::with_capacity(total_bytes + 2);
        res.push(b'#');
        for entry in &sorted {
            res.extend_from_slice(entry);
        }
        res.push(b'$');
        res
    }

    pub fn encode_into(&self, w: &mut impl std::io::Write) -> std::io::Result<usize>
    where
        Data: Bos<[u8]>,
    {
        let (_, sorted) = set_encoded_entries(self.into_iter().map(|e| e.to_bytes()));
        encode_into_as_set(&sorted, w)
    }

    pub fn parse<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Self, E>
    where
        &'i [u8]: Into<Data>,
    {
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

impl<'s, Data> IntoIterator for &'s Set<Data> {
    type Item = <&'s Vec<TokenTree<Data>> as IntoIterator>::Item;

    type IntoIter = <&'s Vec<TokenTree<Data>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

#[derive(Clone)]
// #[cfg_attr(test, derive(proptest_derive::Arbitrary))]
// #[cfg_attr(test, proptest(params = "crate::de::lex::test::MaxDepth"))]
pub struct Dictionary<Data> {
    // #[cfg_attr(
    //     test,
    //     proptest(
    //         strategy = "proptest::collection::hash_map(TokenTree::arbitrary_with(params.next()), TokenTree::arbitrary_with(params.next()), proptest::collection::SizeRange::default())"
    //     )
    // )]
    entries: Vec<(TokenTree<Data>, TokenTree<Data>)>,
}

impl<'i, 'o, IData, OData> From<&'i Dictionary<IData>> for Dictionary<OData>
where
    IData: BorrowOrShare<'i, 'o, [u8]>,
    &'o [u8]: Into<OData>,
{
    fn from(value: &'i Dictionary<IData>) -> Self {
        Self {
            entries: value
                .entries
                .iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }
}

impl<Data> std::fmt::Debug for Dictionary<Data>
where
    Data: Bos<[u8]>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dictionary")
            .field("entries", &self.entries)
            .finish()
    }
}

impl<LData, RData> PartialEq<Dictionary<RData>> for Dictionary<LData>
where
    LData: PartialEq<RData> + Bos<[u8]>,
    RData: Bos<[u8]>,
{
    fn eq(&self, other: &Dictionary<RData>) -> bool {
        // TODO :: performance
        self.encode().eq(&other.encode())
    }
}

impl<Data> Eq for Dictionary<Data> where Data: Eq + Bos<[u8]> {}

pub(crate) type SortedDictEntries<'i> = Vec<(Cow<'i, [u8]>, Cow<'i, [u8]>)>;

pub(crate) fn dict_encoded_entries<'i, Dict>(dict: Dict) -> (usize, SortedDictEntries<'i>)
where
    Dict: IntoIterator<Item = (Cow<'i, [u8]>, Cow<'i, [u8]>)>,
    <Dict as IntoIterator>::IntoIter: ExactSizeIterator,
{
    let entries = dict.into_iter();
    let mut pairs = Vec::with_capacity(entries.len());
    let mut total_bytes = 0;
    for (key, value) in entries {
        total_bytes += key.len() + value.len();
        let ppoint = pairs.partition_point(|(okey, _)| okey < &key);
        if pairs.get(ppoint).is_some() && pairs[ppoint].0 == key {
            // key already present; skip
            continue;
        }
        pairs.insert(ppoint, (key, value));
    }
    (total_bytes, pairs)
}

pub(crate) fn encode_into_as_dict<'i>(
    entries: &SortedDictEntries<'i>,
    w: &mut impl std::io::Write,
) -> std::io::Result<usize> {
    let mut amt = 2; // starting at 2 for the {}
    w.write_all(b"{")?;
    for (key, value) in entries {
        w.write_all(key)?;
        w.write_all(value)?;
        amt += key.len() + value.len();
    }
    w.write_all(b"}")?;
    Ok(amt)
}

impl<Data> Dictionary<Data> {
    /// `entries` should not contain any duplicate keys; they won't be encoded.
    #[inline]
    pub const fn new(entries: Vec<(TokenTree<Data>, TokenTree<Data>)>) -> Self {
        Self { entries }
    }

    pub fn encode(&self) -> Vec<u8>
    where
        Data: Bos<[u8]>,
    {
        let (total_bytes, pairs) =
            dict_encoded_entries(self.into_iter().map(|(k, v)| (k.to_bytes(), v.to_bytes())));
        let mut res = Vec::with_capacity(total_bytes + 2);
        res.push(b'{');
        for (key, value) in &pairs {
            res.extend_from_slice(key);
            res.extend_from_slice(value);
        }
        res.push(b'}');
        res
    }

    pub fn encode_into(&self, w: &mut impl std::io::Write) -> std::io::Result<usize>
    where
        Data: Bos<[u8]>,
    {
        let (_, pairs) =
            dict_encoded_entries(self.into_iter().map(|(k, v)| (k.to_bytes(), v.to_bytes())));
        encode_into_as_dict(&pairs, w)
    }

    pub fn parse<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Self, E>
    where
        &'i [u8]: Into<Data>,
    {
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

impl<'s, Data> IntoIterator for &'s Dictionary<Data> {
    type Item = <&'s Vec<(TokenTree<Data>, TokenTree<Data>)> as IntoIterator>::Item;

    type IntoIter = <&'s Vec<(TokenTree<Data>, TokenTree<Data>)> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}
