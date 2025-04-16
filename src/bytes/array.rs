use std::borrow::Cow;

use crate::de::{DecodeError, Literal, TokenTree};

/// Re-exported here for convenience when using `syrup_derive`
pub use super::encode;

pub fn decode<'error, const LEN: usize>(
    input: &TokenTree,
) -> Result<[u8; LEN], DecodeError<'error>> {
    // TODO :: switch to using [`std::slice::as_array`] once that's out of nightly (this is pretty
    // much just copied from the existing std impl)
    const fn as_array<const LEN: usize>(s: &[u8]) -> Option<&[u8; LEN]> {
        if s.len() == LEN {
            let ptr = s.as_ptr().cast::<[u8; LEN]>();
            #[allow(
                unsafe_code,
                reason = "we just checked that the slice is the right length"
            )]
            Some(unsafe { &*ptr })
        } else {
            None
        }
    }
    match input {
        TokenTree::Literal(Literal::Bytes(b)) => match as_array(b) {
            Some(res) => Ok(*res),
            None => Err(DecodeError::unexpected(
                Cow::Owned(format!("byte string literal with length {LEN}")),
                input.clone(),
            )),
        },
        _ => Err(DecodeError::unexpected(
            Cow::Owned(format!("byte string literal with length {LEN}")),
            input.clone(),
        )),
    }
}
