use std::borrow::Cow;

use crate::de::{DecodeBytesError, DecodeError, Literal, TokenTree};

/// Decode/encode functions for arrays of bytes.
pub mod array;

pub fn decode<'error>(input: &TokenTree) -> Result<Vec<u8>, DecodeError<'error>> {
    match input {
        TokenTree::Literal(Literal::Bytes(b)) => Ok(b.clone()),
        _ => Err(DecodeError::unexpected(
            Cow::Borrowed("byte string literal"),
            input.clone(),
        )),
    }
}

pub fn decode_bytes<'i>(input: &'i [u8]) -> Result<(&'i [u8], Vec<u8>), DecodeBytesError<'i>> {
    let (rem, tree) = TokenTree::parse::<nom::error::Error<&'i [u8]>>(input)?;
    Ok((rem, decode(&tree)?))
}

#[inline]
pub fn encode(bytes: &[u8]) -> TokenTree {
    TokenTree::Literal(Literal::Bytes(bytes.to_vec()))
}

#[inline]
pub fn encode_bytes(bytes: &[u8]) -> Vec<u8> {
    encode(bytes).to_bytes()
}
