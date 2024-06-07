use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use crate::{
    de::{
        Bytes, Dictionary, Int, Literal, LiteralValue, Sequence, Set, Span, Symbol, TokenStream,
        TokenTree,
    },
    literal, Encode,
};

mod _impl_tuple {
    use crate as syrup;

    syrup_proc::impl_encode_for_tuple!(32);
}

impl<'o> Encode<'o> for TokenTree<'o> {
    fn to_tokens_spanned(mut self, span: Span) -> TokenTree<'o> {
        *self.span_mut() = span;
        self
    }

    fn to_tokens(self) -> TokenTree<'o> {
        self
    }
}

impl<'o> Encode<'o> for &'_ TokenTree<'o> {
    fn to_tokens_spanned(self, span: Span) -> TokenTree<'o> {
        self.clone().to_tokens_spanned(span)
    }

    fn to_tokens(self) -> TokenTree<'o> {
        self.clone().to_tokens()
    }
}

macro_rules! impl_encode_copy {
    ($Ty:ty, $Id:ident) => {
        impl<'s> Encode<'s> for $Ty {
            fn to_tokens_spanned(self, span: Span) -> TokenTree<'s> {
                TokenTree::Literal(Literal::new(LiteralValue::$Id(self), span))
            }
        }

        impl<'s> Encode<'s> for &'_ $Ty {
            fn to_tokens_spanned(self, span: Span) -> TokenTree<'s> {
                TokenTree::Literal(Literal::new(LiteralValue::$Id(*self), span))
            }
        }
    };
}

macro_rules! impl_encode_int {
    ($($Int:ty),+) => {
        $(
        impl<'o> Encode<'o> for $Int {
            fn to_tokens_spanned(self, span: Span) -> TokenTree<'o> {
                TokenTree::Literal(Literal::new(LiteralValue::Int(Int::from(self)), span))
            }
        }
        )+
    };
}

impl_encode_int!(u8, u16, u32, u64, usize, u128, i8, i16, i32, i64, isize, i128);

impl_encode_copy! {bool, Bool}
impl_encode_copy! {f32, F32}
impl_encode_copy! {f64, F64}

impl<'o> Encode<'o> for &'o str {
    fn to_tokens_spanned(self, span: Span) -> TokenTree<'o> {
        literal![Cow::Borrowed(self.as_bytes()), span => String]
    }
}

impl<'o> Encode<'o> for String {
    fn to_tokens_spanned(self, span: Span) -> TokenTree<'o> {
        literal![Cow::Owned(self.into_bytes()), span => String]
    }
}

impl<'o> Encode<'o> for Bytes<'o> {
    fn to_tokens_spanned(self, span: Span) -> TokenTree<'o> {
        literal![self.0, span => Bytes]
    }
}

impl<'o> Encode<'o> for &'o Bytes<'o> {
    fn to_tokens_spanned(self, span: Span) -> TokenTree<'o> {
        literal![Cow::Borrowed(&*self.0), span => Bytes]
    }
}

macro_rules! impl_encode_strlike {
    ($self:ident, $Ty:ty, $Id:ident, $match:expr) => {
        impl<'o> Encode<'o> for $Ty {
            fn to_tokens_spanned(self, span: Span) -> TokenTree<'o> {
                let $self = self;
                TokenTree::Literal(Literal::new(
                    LiteralValue::$Id(match $match {
                        Cow::Borrowed(b) => Cow::Borrowed(b.as_bytes()),
                        Cow::Owned(b) => Cow::Owned(b.into_bytes()),
                    }),
                    span,
                ))
            }
        }

        impl<'o> Encode<'o> for &'o $Ty {
            fn to_tokens_spanned(self, span: Span) -> TokenTree<'o> {
                let $self = self;
                TokenTree::Literal(Literal::new(
                    LiteralValue::$Id(Cow::Borrowed($match.as_bytes())),
                    span,
                ))
            }
        }
    };
}

impl_encode_strlike! { s, Cow<'o, str>, String, s }
impl_encode_strlike! { sym, Symbol<'o>, Symbol, sym.0 }

impl<'o, T> Encode<'o> for &'o [T]
where
    &'o T: Encode<'o>,
{
    fn to_tokens_spanned(self, span: Span) -> TokenTree<'o> {
        TokenTree::Sequence(Sequence {
            stream: TokenStream::new(self.iter().map(<&T>::to_tokens).collect::<Vec<_>>()),
            span,
        })
    }
}

impl<'o, T> Encode<'o> for Vec<T>
where
    T: Encode<'o>,
{
    fn to_tokens_spanned(self, span: Span) -> TokenTree<'o> {
        TokenTree::Sequence(Sequence {
            stream: TokenStream::new(self.into_iter().map(T::to_tokens).collect::<Vec<_>>()),
            span,
        })
    }
}

impl<'map, 'o, K, V, S> Encode<'o> for &'map HashMap<K, V, S>
where
    &'map K: Encode<'o>,
    &'map V: Encode<'o>,
{
    fn to_tokens_spanned(self, span: Span) -> TokenTree<'o> {
        let mut stream = Vec::with_capacity(self.len() * 2);
        for (k, v) in self.iter() {
            stream.push(k.to_tokens());
            stream.push(v.to_tokens());
        }

        TokenTree::Dictionary(Dictionary {
            stream: TokenStream::new(stream),
            span,
        })
    }
}

impl<'set, 'o, T, S> Encode<'o> for &'set HashSet<T, S>
where
    &'set T: Encode<'o>,
{
    fn to_tokens_spanned(self, span: Span) -> TokenTree<'o> {
        TokenTree::Set(Set {
            stream: TokenStream::new(self.iter().map(<&T>::to_tokens).collect::<Vec<_>>()),
            span,
        })
    }
}
