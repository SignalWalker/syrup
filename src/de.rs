mod lex;
pub use lex::*;

mod error;
pub use error::*;

/// `[Decode]` implementations for standard library types
mod impl_decode;

pub trait Decode<'input>: Sized {
    fn decode<'error>(input: &'input TokenTree) -> Result<Self, DecodeError<'error>>;
}

mod private {
    /// Exists only to prevent external implementation of certain traits
    pub trait DecodeSealed {}
}

impl<'i, T: Decode<'i>> private::DecodeSealed for T {}

/// Trait implemented for `T where for<'t> T: [Decode]<'t>`. In other words, for any type with a
/// `[Decode]` implementation that doesn't require holding a reference to the input `[TokenTree]`.
pub trait DecodeToOwned: Sized + private::DecodeSealed {
    /// Decode the input bytes into a tuple of `(unconsumed input, Self)`.
    fn decode_bytes<'i>(input: &'i [u8]) -> Result<(&'i [u8], Self), DecodeBytesError<'i>>;
}

impl<T> DecodeToOwned for T
where
    for<'tree> T: Decode<'tree>,
{
    fn decode_bytes<'i>(input: &'i [u8]) -> Result<(&'i [u8], T), DecodeBytesError<'i>> {
        let (rem, tree) = TokenTree::parse::<nom::error::Error<&'i [u8]>>(input)?;
        Ok((rem, T::decode(&tree)?))
    }
}
