use borrow_or_share::BorrowOrShare;

use crate::de::{DecodeBytesError, DecodeError, Literal, SyrupKind, TokenTree};

/// Decode/encode functions for arrays of bytes.
pub mod array;

mod bytes_ty;
pub use bytes_ty::*;

pub fn decode<'input, 'output, IData, OData>(
    input: &'input TokenTree<IData>,
) -> Result<OData, DecodeError>
where
    IData: BorrowOrShare<'input, 'output, [u8]>,
    &'output [u8]: Into<OData>,
{
    match input {
        TokenTree::Literal(Literal::Bytes(b)) => Ok(b.borrow_or_share().into()),
        _ => Err(DecodeError::unexpected(
            SyrupKind::Bytes { length: None },
            input,
        )),
    }
}

pub fn decode_bytes<'i, Output>(input: &'i [u8]) -> Result<(&'i [u8], Output), DecodeBytesError<'i>>
where
    &'i [u8]: Into<Output>,
{
    let (rem, tree) = TokenTree::<&'i [u8]>::parse::<nom::error::Error<&'i [u8]>>(input)?;
    Ok((rem, decode::<&'i [u8], &'i [u8]>(&tree)?.into()))
}

#[inline]
pub fn encode<IData, OData>(bytes: IData) -> TokenTree<OData>
where
    IData: Into<OData>,
{
    TokenTree::Literal(Literal::Bytes(bytes.into()))
}

#[inline]
pub fn encode_bytes(bytes: &[u8]) -> Vec<u8> {
    encode::<_, &[u8]>(bytes).to_bytes().into_owned()
}

pub fn encode_into(bytes: &[u8], w: &mut impl std::io::Write) -> std::io::Result<usize> {
    encode::<_, &[u8]>(bytes).write_bytes(w)
}
