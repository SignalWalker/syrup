use std::borrow::Cow;

use crate::de::{DecodeBytesError, DecodeError, Literal, TokenTree};

pub fn decode<'error>(input: &TokenTree) -> Result<String, DecodeError<'error>> {
    match input {
        TokenTree::Literal(Literal::Symbol(s)) => match std::str::from_utf8(s) {
            Ok(s) => Ok(s.to_owned()),
            Err(error) => Err(DecodeError::utf8(Cow::Owned(s.clone()), error)),
        },
        _ => Err(DecodeError::unexpected(
            Cow::Borrowed("symbol literal"),
            input.clone(),
        )),
    }
}

pub fn decode_bytes<'i>(input: &'i [u8]) -> Result<(&'i [u8], String), DecodeBytesError<'i>> {
    let (rem, tree) = TokenTree::parse::<nom::error::Error<&'i [u8]>>(input)?;
    Ok((rem, decode(&tree)?))
}

#[inline]
pub fn encode(sym: &str) -> TokenTree {
    TokenTree::Literal(Literal::Symbol(sym.as_bytes().to_vec()))
}

#[inline]
pub fn encode_bytes(sym: &str) -> Vec<u8> {
    encode(sym).to_bytes()
}

pub trait EncodeAsSymbol {
    /// Encode self as a syrup symbol.
    fn encode_as_symbol(&self) -> TokenTree;
}

impl EncodeAsSymbol for str {
    #[inline]
    fn encode_as_symbol(&self) -> TokenTree {
        encode(self)
    }
}
