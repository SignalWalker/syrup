use std::{borrow::Cow, collections::BinaryHeap, fmt::Write};

use crate::{Encode, Span, TokenStream, TokenTree};

// pub struct RecordBuilder<'i> {
//     span: Span,
//     stream: Vec<TokenTree<'i>>,
// }
//
// impl<'i> RecordBuilder<'i> {
//     pub(super) fn new(span: Span, label: TokenTree<'i>) -> Self {
//         Self {
//             span,
//             stream: vec![label],
//         }
//     }
//
//     pub fn with_fields(mut self, mut fields: Vec<TokenTree<'i>>) -> Self {
//         self.stream.append(&mut fields);
//         self
//     }
//
//     pub fn build(self) -> Group<'i> {
//         Group {
//             delimiter: super::Delimiter::Angle,
//             stream: TokenStream::new(self.stream),
//             span: self.span,
//         }
//     }
// }

#[derive(Debug, Clone)]
pub struct Dictionary<'input> {
    pub stream: TokenStream<'input>,
    pub span: Span,
}

impl<'i> std::fmt::Display for Dictionary<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('{')?;
        let mut is_first = true;
        for entries in self.stream.as_slice().chunks(2) {
            if is_first {
                write!(f, " {}: {}", entries[0], entries[1])?;
                is_first = false;
            } else {
                write!(f, ", {}: {}", entries[0], entries[1])?;
            }
        }
        f.write_str(" }")
    }
}

impl<'i> Dictionary<'i> {
    pub const fn new(stream: TokenStream<'i>, span: Span) -> Self {
        Self { stream, span }
    }

    pub fn encode(&self) -> Vec<u8> {
        #[derive(PartialEq, Eq)]
        struct DictEntry<'i>(Cow<'i, [u8]>, Cow<'i, [u8]>);
        impl<'i> PartialOrd for DictEntry<'i> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.0.cmp(&other.0))
            }
        }
        impl<'i> Ord for DictEntry<'i> {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.0.cmp(&other.0)
            }
        }

        let mut res = vec![b'{'];

        let pairs = self
            .stream
            .as_slice()
            .chunks(2)
            .map(|chunk| DictEntry(chunk[0].encode(), chunk[1].encode()))
            .collect::<BinaryHeap<_>>()
            .into_sorted_vec();

        for DictEntry(key, value) in pairs {
            res.extend_from_slice(&key);
            res.extend_from_slice(&value);
        }

        res.push(b'}');
        res
    }
}

#[derive(Debug, Clone)]
pub struct Sequence<'input> {
    pub stream: TokenStream<'input>,
    pub span: Span,
}

impl<'i> std::fmt::Display for Sequence<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('[')?;
        let mut is_first = true;
        for element in self.stream.as_slice() {
            if is_first {
                write!(f, " {element}")?;
                is_first = false;
            } else {
                write!(f, ", {element}")?;
            }
        }
        f.write_str(" ]")
    }
}

impl<'i> Sequence<'i> {
    pub const fn new(stream: TokenStream<'i>, span: Span) -> Self {
        Self { stream, span }
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut res = vec![b'['];
        for token in self.stream.as_slice() {
            res.extend_from_slice(&token.encode());
        }
        res.push(b']');
        res
    }
}

impl<'i, E: Encode<'i>> FromIterator<E> for Sequence<'i> {
    fn from_iter<Iter: IntoIterator<Item = E>>(iter: Iter) -> Self {
        Self {
            span: Default::default(),
            stream: TokenStream::new(iter.into_iter().map(Encode::to_tokens).collect()),
        }
    }
}

#[macro_export]
macro_rules! sequence {
    () => {
        $crate::Sequence::new($crate::TokenStream::new(::std::vec![]), $crate::de::Span::new(0, 0))
    };
    ($($elem:expr),+ $(,)?) => {
        $crate::Sequence::new($crate::TokenStream::new(::std::vec![$($crate::Encode::to_tokens($elem)),+]), $crate::de::Span::new(0, 0))
    };
}

#[macro_export]
macro_rules! call_sequence {
    ($label:expr $(,)?) => {
        $crate::sequence![$crate::Symbol::from($label)]
    };
    ($label:expr, $($elem:expr),+ $(,)?) => {
        $crate::sequence![$crate::Symbol::from($label), $($elem),+]
    };
}

#[derive(Debug, Clone)]
pub struct Record<'input> {
    pub stream: TokenStream<'input>,
    pub span: Span,
}

impl<'i> std::fmt::Display for Record<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('<')?;
        let mut is_first = true;
        for element in self.stream.as_slice() {
            if is_first {
                write!(f, " {element}")?;
                is_first = false;
            } else {
                write!(f, ", {element}")?;
            }
        }
        f.write_str(" >")
    }
}

impl<'i> Record<'i> {
    pub const fn new(stream: TokenStream<'i>, span: Span) -> Self {
        Self { stream, span }
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut res = vec![b'<'];
        for token in self.stream.as_slice() {
            res.extend_from_slice(&token.encode());
        }
        res.push(b'>');
        res
    }
}

#[derive(Debug, Clone)]
pub struct Set<'input> {
    pub stream: TokenStream<'input>,
    pub span: Span,
}

impl<'i> std::fmt::Display for Set<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('#')?;
        let mut is_first = true;
        for element in self.stream.as_slice() {
            if is_first {
                write!(f, " {element}")?;
                is_first = false;
            } else {
                write!(f, ", {element}")?;
            }
        }
        f.write_str(" $")
    }
}

impl<'i> Set<'i> {
    pub fn new(stream: TokenStream<'i>, span: Span) -> Self {
        Self { stream, span }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut res = vec![b'#'];

        for entry in self
            .stream
            .as_slice()
            .iter()
            .map(TokenTree::encode)
            .collect::<BinaryHeap<_>>()
            .into_sorted_vec()
        {
            res.extend_from_slice(&entry);
        }

        res.push(b'$');
        res
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Delimiter {
    /// `{ ... }` (used for dictionaries)
    Curly,
    /// `[ ... ]` (used for sequences)
    Square,
    /// `< ... >` (used for records)
    Angle,
    /// `# ... $` (used for sets)
    HashDollar,
}

impl Delimiter {
    pub const fn open(&self) -> u8 {
        match self {
            Delimiter::Curly => b'{',
            Delimiter::Square => b'[',
            Delimiter::Angle => b'<',
            Delimiter::HashDollar => b'#',
        }
    }

    pub const fn close(&self) -> u8 {
        match self {
            Delimiter::Curly => b'}',
            Delimiter::Square => b']',
            Delimiter::Angle => b'>',
            Delimiter::HashDollar => b'$',
        }
    }

    pub const fn from_open(byte: u8) -> Option<Self> {
        match byte {
            b'{' => Some(Self::Curly),
            b'<' => Some(Self::Angle),
            b'[' => Some(Self::Square),
            b'#' => Some(Self::HashDollar),
            _ => None,
        }
    }

    pub const fn from_close(byte: u8) -> Option<Self> {
        match byte {
            b'}' => Some(Self::Curly),
            b'>' => Some(Self::Angle),
            b']' => Some(Self::Square),
            b'$' => Some(Self::HashDollar),
            _ => None,
        }
    }
}

// /// A [`TokenStream`] inside a [`Delimiter`] pair.
// #[derive(Debug, Clone)]
// pub struct Group<'input> {
//     pub delimiter: Delimiter,
//     pub stream: TokenStream<'input>,
//     pub span: Span,
// }
//
// impl<'i> Group<'i> {
//     pub const fn new(delimiter: Delimiter, stream: TokenStream<'i>, span: Span) -> Self {
//         Self {
//             delimiter,
//             stream,
//             span,
//         }
//     }
//
//     pub fn encode(&self) -> Vec<u8> {
//         let mut res = Vec::new();
//         res.push(self.delimiter.open());
//         for token in self.stream.as_slice() {
//             res.extend_from_slice(&token.encode());
//         }
//         res.push(self.delimiter.close());
//         res
//     }
//
//     pub fn sequence(stream: TokenStream<'i>, span: Span) -> Self {
//         Self {
//             delimiter: Delimiter::Square,
//             stream,
//             span,
//         }
//     }
//
//     // pub fn set(mut stream: TokenStream<'i>, span: Span) -> Self {
//     //     stream.sort_unstable();
//     //     Self {
//     //         delimiter: Delimiter::HashDollar,
//     //         stream,
//     //         span,
//     //     }
//     // }
//
//     pub fn dictionary(stream: TokenStream<'i>, span: Span) -> Self {
//         // TODO :: ensure sorted
//         Self {
//             delimiter: Delimiter::Curly,
//             stream,
//             span,
//         }
//     }
//
//     pub fn record_builder(span: Span, label: TokenTree<'i>) -> RecordBuilder<'i> {
//         RecordBuilder::new(span, label)
//     }
// }
