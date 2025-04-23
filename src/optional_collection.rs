use crate::{
    de::{Decode, DecodeError, Literal, TokenTree},
    ser::Encode,
};

// TODO :: surely there's a better way to do this

/// Encode a collection, where an empty collection encodes to `false`.
pub fn encode<'t, T, OData>(collection: &'t T) -> TokenTree<OData>
where
    T: Encode<'t, OData>,
    // these rules are so we can check whether it's empty
    &'t T: IntoIterator,
    <&'t T as IntoIterator>::IntoIter: ExactSizeIterator,
{
    if collection.into_iter().len() == 0 {
        false.encode()
    } else {
        collection.encode()
    }
}

/// Decode a collection, where `false` decodes to an empty collection.
pub fn decode<'input, IData, T>(input: &'input TokenTree<IData>) -> Result<T, DecodeError>
where
    T: Decode<'input, IData> + Default,
{
    match input {
        TokenTree::Literal(Literal::Bool(false)) => Ok(T::default()),
        tree => T::decode(tree),
    }
}
