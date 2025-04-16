use ed25519_dalek::Signature;
use syrup::{Decode, DecodeError, Encode, TokenTree};

mod __impl_sig {
    use syrup::{Decode, Encode};

    #[derive(Encode, Decode)]
    #[cfg_attr(test, derive(std::fmt::Debug, proptest_derive::Arbitrary))]
    #[syrup(label = "r")]
    pub(crate) struct R(#[syrup(with = syrup::bytes::array)] pub(crate) [u8; 32]);

    #[derive(Encode, Decode)]
    #[cfg_attr(test, derive(std::fmt::Debug, proptest_derive::Arbitrary))]
    #[syrup(label = "s")]
    pub(crate) struct S(#[syrup(with = syrup::bytes::array)] pub(crate) [u8; 32]);

    #[derive(Encode, Decode)]
    #[cfg_attr(test, derive(std::fmt::Debug, proptest_derive::Arbitrary))]
    #[syrup(label = "eddsa")]
    pub(crate) struct Eddsa(pub(crate) R, pub(crate) S);
}

#[cfg(test)]
pub(crate) use __impl_sig::*;

/// Encode as `<eddsa <r [u8; 32]> <s [u8; 32]>>`
pub fn encode(sig: &Signature) -> TokenTree {
    use __impl_sig::*;
    Eddsa(R(*sig.r_bytes()), S(*sig.s_bytes())).encode()
}

/// Decode from `<eddsa <r [u8; 32]> <s [u8; 32]>>`
pub fn decode(input: &TokenTree) -> Result<Signature, DecodeError<'_>> {
    __impl_sig::Eddsa::decode(input)
        .map(|__impl_sig::Eddsa(r, s)| Signature::from_components(r.0, s.0))
}
