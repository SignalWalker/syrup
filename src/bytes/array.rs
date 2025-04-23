use borrow_or_share::BorrowOrShare;

use crate::de::{DecodeError, Literal, SyrupKind, TokenTree};

/// Re-exported here for convenience when using `syrup_derive`
pub use super::encode;

pub fn decode<'tree, 'output, IData, OData, const LEN: usize>(
    input: &'tree TokenTree<IData>,
) -> Result<OData, DecodeError>
where
    IData: BorrowOrShare<'tree, 'output, [u8]>,
    &'output [u8; LEN]: Into<OData>,
{
    // TODO :: switch to using [`std::slice::as_array`] once that's out of nightly (this is pretty
    // much just copied from the existing std impl)
    const fn as_array<const LEN: usize>(s: &[u8]) -> Option<&[u8; LEN]> {
        if s.len() == LEN {
            let ptr = s.as_ptr().cast::<[u8; LEN]>();
            #[expect(
                unsafe_code,
                reason = "we just checked that the slice is the right length"
            )]
            Some(unsafe { &*ptr })
        } else {
            None
        }
    }
    match input {
        TokenTree::Literal(Literal::Bytes(b)) => match as_array::<LEN>(b.borrow_or_share()) {
            Some(res) => Ok(res.into()),
            None => Err(DecodeError::unexpected(
                SyrupKind::Bytes { length: Some(LEN) },
                input,
            )),
        },
        _ => Err(DecodeError::unexpected(
            SyrupKind::Bytes { length: Some(LEN) },
            input,
        )),
    }
}
