use ed25519_dalek::Signature;
use syrup::{de::DecodeError, Decode, Encode, Span, TokenStream, TokenTree};

mod __impl_sig {
    use syrup::{ByteArray, Decode, Encode};

    #[derive(Encode, Decode)]
    #[syrup(label = "r")]
    pub(super) struct R<'b>(pub(super) ByteArray<'b, 32>);

    #[derive(Encode, Decode)]
    #[syrup(label = "s")]
    pub(super) struct S<'b>(pub(super) ByteArray<'b, 32>);

    #[derive(Encode, Decode)]
    #[syrup(label = "eddsa")]
    pub(super) struct Eddsa<'i>(pub(super) R<'i>, pub(super) S<'i>);
}

/// Encode as `<eddsa <r [u8]> <s [u8]>>`
pub fn to_tokens_spanned(sig: &Signature, span: Span) -> TokenTree<'_> {
    use __impl_sig::*;
    Eddsa(R(sig.r_bytes().into()), S(sig.s_bytes().into())).to_tokens_spanned(span)
}

/// Decode from `<eddsa <r [u8]> <s [u8]>>`
pub fn decode(input: TokenTree<'_>) -> Result<Signature, DecodeError<'_>> {
    __impl_sig::Eddsa::decode(input)
        .map(|__impl_sig::Eddsa(r, s)| Signature::from_components(*r.0, *s.0))
}
