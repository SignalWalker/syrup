mod lex;
pub use lex::*;

mod error;
pub use error::*;

/// `[Decode]` implementations for standard library types
mod impl_decode;

pub trait Decode<'tree, IData>: Sized {
    fn decode(input: &'tree TokenTree<IData>) -> Result<Self, DecodeError>;
}

/// Trait implemented for `T where for<'t> T: [Decode]<'t, &'input [u8]>`. In other words, for any type with a
/// `[Decode]` implementation that doesn't require holding a reference to the input `[TokenTree]`.
pub trait DecodeFromBytes<'input>: Sized {
    /// Decode the input bytes into a tuple of `(unconsumed input, Self)`.
    fn decode_bytes(input: &'input [u8]) -> Result<(&'input [u8], Self), DecodeBytesError<'input>>;
}

impl<'input, T> DecodeFromBytes<'input> for T
where
    // NOTE :: `due to current limitations in the borrow checker, this implies a 'static lifetime`,
    // so this isn't actually useful for any `'input: !'static` right now
    for<'tree> T: Decode<'tree, &'input [u8]>,
{
    fn decode_bytes(input: &'input [u8]) -> Result<(&'input [u8], T), DecodeBytesError<'input>> {
        let (rem, tree) = TokenTree::<&[u8]>::parse::<nom::error::Error<&'input [u8]>>(input)?;
        Ok((rem, T::decode(&tree)?))
    }
}

/// Decode the input byte slice to the given type.
///
/// Does the same thing as [`DecodeFromBytes::decode_bytes`], but that trait isn't ergonomic with
/// current compiler limitations.
#[macro_export]
macro_rules! decode_bytes {
    ($input:expr => $Output:ty) => {
        $crate::de::TokenTree::<&[u8]>::parse::<$crate::nom::error::Error<&[u8]>>($input)
            .map_err($crate::de::DecodeBytesError::from)
            .and_then(|(rem, tree)| Ok((rem, tree.decode::<'_, $Output>()?)))
    };
}
