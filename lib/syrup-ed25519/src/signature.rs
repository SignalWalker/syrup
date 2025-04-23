use ed25519_dalek::Signature;
use syrup::{borrow_or_share::Bos, Decode, DecodeError, Encode, TokenTree};

mod __impl_sig {
    use syrup::{Decode, Encode};

    #[derive(Encode, Decode)]
    #[syrup(label = "r", encode_where = { &'bytes [u8]: Into<__OData> }, decode_where = { '__output: 'bytes })]
    pub(crate) struct R<'bytes>(
        #[syrup(encode = syrup::bytes::array::encode(self.0.as_slice()), decode = syrup::bytes::array::decode)]
        pub(crate) &'bytes [u8; 32],
    );

    #[derive(Encode, Decode)]
    #[syrup(label = "s", encode_where = { &'bytes [u8]: Into<__OData> }, decode_where = { '__output: 'bytes })]
    pub(crate) struct S<'bytes>(
        #[syrup(encode = syrup::bytes::array::encode(self.0.as_slice()), decode = syrup::bytes::array::decode)]
        pub(crate) &'bytes [u8; 32],
    );

    #[derive(Encode, Decode)]
    #[syrup(label = "eddsa")]
    pub(crate) struct Eddsa<'bytes>(pub(crate) R<'bytes>, pub(crate) S<'bytes>);
}

/// Encode as `<eddsa <r [u8; 32]> <s [u8; 32]>>`
pub fn encode<'input, OData>(sig: &'input Signature) -> TokenTree<OData>
where
    &'input [u8]: Into<OData>,
{
    use __impl_sig::*;
    Eddsa::<'input>(R(sig.r_bytes()), S(sig.s_bytes())).encode()
}

/// Decode from `<eddsa <r [u8; 32]> <s [u8; 32]>>`
pub fn decode<IData>(input: &TokenTree<IData>) -> Result<Signature, DecodeError>
where
    IData: Bos<[u8]>,
{
    __impl_sig::Eddsa::decode(input)
        .map(|__impl_sig::Eddsa(r, s)| Signature::from_components(*r.0, *s.0))
}
