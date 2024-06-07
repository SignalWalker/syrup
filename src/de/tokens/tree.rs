use std::borrow::Cow;

use crate::{
    de::{
        Cursor, Decode, DecodeError, Delimiter, Dictionary, LexError, Literal, Record, Sequence,
        Set, Span,
    },
    TokenStream,
};

#[derive(Debug, Clone)]
pub enum TokenTree<'input> {
    // Group(Group<'input>),
    Dictionary(Dictionary<'input>),
    Sequence(Sequence<'input>),
    Record(Record<'input>),
    Set(Set<'input>),
    Literal(Literal<'input>),
    // Punct(Punct),
}

impl PartialEq for TokenTree<'_> {
    fn eq(&self, other: &Self) -> bool {
        (*self.encode()).eq(&*other.encode())
    }
}

impl Eq for TokenTree<'_> {}

impl PartialOrd for TokenTree<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TokenTree<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self.encode()).cmp(&*other.encode())
    }
}

impl<'i> From<Literal<'i>> for TokenTree<'i> {
    fn from(lit: Literal<'i>) -> Self {
        Self::Literal(lit)
    }
}

impl<'i> std::fmt::Display for TokenTree<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenTree::Dictionary(d) => d.fmt(f),
            TokenTree::Sequence(s) => s.fmt(f),
            TokenTree::Record(r) => r.fmt(f),
            TokenTree::Set(s) => s.fmt(f),
            TokenTree::Literal(l) => l.fmt(f),
        }
    }
}

macro_rules! impl_from_group {
    ($Group:ident) => {
        impl<'i> From<$Group<'i>> for TokenTree<'i> {
            fn from(group: $Group<'i>) -> Self {
                Self::$Group(group)
            }
        }
    };
}

impl_from_group!(Dictionary);
impl_from_group!(Sequence);
impl_from_group!(Record);
impl_from_group!(Set);

impl<'input> TokenTree<'input> {
    pub fn span(&self) -> &Span {
        match self {
            TokenTree::Dictionary(g) => &g.span,
            TokenTree::Sequence(g) => &g.span,
            TokenTree::Record(g) => &g.span,
            TokenTree::Set(g) => &g.span,
            TokenTree::Literal(l) => &l.span,
        }
    }

    pub const fn dictionary(stream: TokenStream<'input>, span: Span) -> Self {
        Self::Dictionary(Dictionary { stream, span })
    }

    pub const fn sequence(stream: TokenStream<'input>, span: Span) -> Self {
        Self::Sequence(Sequence { stream, span })
    }

    pub const fn record(stream: TokenStream<'input>, span: Span) -> Self {
        Self::Record(Record { stream, span })
    }

    pub const fn set(stream: TokenStream<'input>, span: Span) -> Self {
        Self::Set(Set { stream, span })
    }

    pub fn span_mut(&mut self) -> &mut Span {
        match self {
            TokenTree::Dictionary(g) => &mut g.span,
            TokenTree::Sequence(g) => &mut g.span,
            TokenTree::Record(g) => &mut g.span,
            TokenTree::Set(g) => &mut g.span,
            TokenTree::Literal(l) => &mut l.span,
            // TokenTree::Punct(_) => todo!(),
        }
    }

    #[inline]
    pub fn decode<T: Decode<'input>>(self) -> Result<T, DecodeError<'input>> {
        T::decode(self)
    }

    pub fn encode(&self) -> Cow<'_, [u8]> {
        match self {
            TokenTree::Dictionary(g) => g.encode().into(),
            TokenTree::Sequence(g) => g.encode().into(),
            TokenTree::Record(g) => g.encode().into(),
            TokenTree::Set(g) => g.encode().into(),
            TokenTree::Literal(lit) => lit.encode(),
        }
    }

    pub const fn to_unexpected(self, expected: Cow<'input, str>) -> DecodeError<'input> {
        DecodeError::unexpected(expected, self)
    }

    //pub fn into_static(self) -> TokenTree<'static> {
    //    match self {
    //        TokenTree::Dictionary(dict) => Self::from(dict.into_static()),
    //        TokenTree::Sequence(seq) => Self::from(seq.into_static()),
    //        TokenTree::Record(rec) => Self::from(rec.into_static()),
    //        TokenTree::Set(set) => Self::from(set.into_static()),
    //        TokenTree::Literal(lit) => ,
    //    }
    //}
}

macro_rules! impl_tokenize {
    ($input:ident, $leaf_token_fn:path) => {
        // this is for storing tokens within group frames; allocate with 0 initially because this
        // will be reallocated regardless when opening a group. We're just assigning it here
        // because, otherwise, the compiler gets mad.
        let mut trees: Vec<TokenTree<'_>> = Vec::with_capacity(0);
        // this is for storing state when entering a new group
        let mut stack: Vec<(usize, (Delimiter, Vec<TokenTree<'_>>))> = Vec::new();
        loop {
            let lo = $input.off;
            let Some(&first) = $input.rem.first() else {
                match stack.last() {
                    None => {
                        // empty input
                        return Err(LexError::incomplete(
                            Span::new(lo, lo),
                            nom::Needed::Unknown,
                        ));
                    }
                    Some((lo, _frame)) => {
                        // unclosed group
                        // TODO :: maybe we could include the already-lexed tokens in the error, so
                        // we could reuse them later?
                        return Err(LexError::incomplete(
                            Span::new(*lo, $input.off),
                            nom::Needed::Unknown,
                        ));
                    }
                }
            };
            if let Some(open_delim) = Delimiter::from_open(first) {
                // opening a group
                $input = $input.advance(1);
                let frame = (lo, (open_delim, trees));
                stack.push(frame);
                trees = Vec::new();
            } else if let Some(close_delim) = Delimiter::from_close(first) {
                // closing a group
                let Some((lo, (open_delim, outer))) = stack.pop() else {
                    return Err(LexError::unexpected(
                        Span::new($input.off, $input.off),
                        "group element or closing delimiter",
                    ));
                };
                if open_delim != close_delim {
                    return Err(LexError::unmatched(
                        Span::new($input.off, $input.off),
                        open_delim,
                    ));
                }
                $input = $input.advance(1);
                let span = Span::new(lo, $input.off);
                let group: TokenTree<'_> = match open_delim {
                    Delimiter::Curly => Dictionary::new(TokenStream::new(trees), span).into(),
                    Delimiter::Square => Sequence::new(TokenStream::new(trees), span).into(),
                    Delimiter::Angle => Record::new(TokenStream::new(trees), span).into(),
                    Delimiter::HashDollar => Set::new(TokenStream::new(trees), span).into(),
                };
                if stack.is_empty() {
                    return Ok((group, $input));
                } else {
                    trees = outer;
                    trees.push(group);
                }
            } else {
                // now expecting a leaf...
                let (rest, mut tt) = match $leaf_token_fn($input) {
                    Ok(v) => v,
                    Err(e) => {
                        return Err(LexError::from_parse_literal(
                            Span::new($input.off, $input.off),
                            e,
                        ))
                    }
                };
                *tt.span_mut() = Span::new(lo, rest.off);
                if stack.is_empty() {
                    // we're not in a group; this is a bare literal
                    return Ok((tt, rest));
                } else {
                    // we're in a group; move to the next token
                    $input = rest;
                    trees.push(tt);
                }
            }
        }
    };
}

impl<'input> TokenTree<'input> {
    /// Lex the first token from an input slice, returning the resulting token and the remaining
    /// input.
    pub fn tokenize(mut input: Cursor<'input>) -> Result<(Self, Cursor<'input>), LexError> {
        impl_tokenize! {input, Cursor::leaf_token}
    }
}

impl TokenTree<'static> {
    pub fn tokenize_static(mut input: Cursor<'_>) -> Result<(Self, Cursor<'_>), LexError> {
        impl_tokenize! {input, Cursor::leaf_token_static}
    }
}
