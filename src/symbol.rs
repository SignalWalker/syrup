use crate::de::{DecodeBytesError, DecodeError, Literal, SyrupKind, TokenTree};

use borrow_or_share::BorrowOrShare;

mod symbol_ty;
pub use symbol_ty::*;

/// Helper trait for encoding something as a symbol literal.
pub trait EncodeAsSymbol<'input, OData> {
    /// Encode self as a syrup symbol.
    fn encode_as_symbol(&'input self) -> TokenTree<OData>;
}

impl<'input, OData> EncodeAsSymbol<'input, OData> for str
where
    &'input str: Into<OData>,
{
    #[inline]
    fn encode_as_symbol(&'input self) -> TokenTree<OData> {
        encode(self)
    }
}

pub fn encode<OData, Str>(s: Str) -> TokenTree<OData>
where
    Str: Into<OData>, // FIX :: apparently str doesn't impl Into<&[u8]>???
{
    TokenTree::Literal(Literal::Symbol(s.into()))
}

pub fn decode<'input, 'output, IData, Output>(
    input: &'input TokenTree<IData>,
) -> Result<Output, DecodeError>
where
    IData: BorrowOrShare<'input, 'output, [u8]>,
    &'output str: Into<Output>,
{
    match input {
        TokenTree::Literal(Literal::Symbol(s)) => {
            Ok(std::str::from_utf8(s.borrow_or_share())?.into())
        }
        _ => Err(DecodeError::unexpected(SyrupKind::Symbol(None), input)),
    }
}

pub fn decode_bytes<'i, 'err, OStr>(
    input: &'i [u8],
) -> Result<(&'i [u8], OStr), DecodeBytesError<'i>>
where
    &'i str: Into<OStr>,
{
    let (rem, tree) = TokenTree::<&'i [u8]>::parse::<nom::error::Error<&'i [u8]>>(input)?;
    let decoded = decode::<&'i [u8], &'i str>(&tree)?;
    Ok((rem, decoded.into()))
}
