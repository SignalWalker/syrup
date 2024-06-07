use std::collections::HashMap;

use crate::{
    de::{DecodeError, Literal, LiteralValue},
    Decode, Encode, Span, TokenTree,
};

pub fn to_tokens_spanned<'map, 'output, K, V, S>(
    map: &'map HashMap<K, V, S>,
    span: Span,
) -> TokenTree<'output>
where
    &'map HashMap<K, V, S>: Encode<'output>,
{
    if map.is_empty() {
        false.to_tokens_spanned(span)
    } else {
        map.to_tokens_spanned(span)
    }
}

pub fn decode<'input, K, V, S>(
    input: TokenTree<'input>,
) -> Result<HashMap<K, V, S>, DecodeError<'input>>
where
    HashMap<K, V, S>: Decode<'input>,
    S: Default,
{
    match input {
        TokenTree::Literal(Literal {
            repr: LiteralValue::Bool(false),
            ..
        }) => Ok(HashMap::with_capacity_and_hasher(0, S::default())),
        tree => HashMap::decode(tree),
    }
}
