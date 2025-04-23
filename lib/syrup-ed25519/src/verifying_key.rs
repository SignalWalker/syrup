use ed25519_dalek::VerifyingKey;
use syrup::{
    borrow_or_share::Bos, de::SyrupKind, symbol::Symbol, Decode, DecodeError, Encode, TokenTree,
};

mod __impl_vkey {

    use syrup::{symbol::Symbol, Decode, Encode};

    #[derive(Encode, Decode)]
    #[syrup(label = "curve")]
    pub(crate) struct Curve<Str> {
        pub(crate) kind: Symbol<Str>,
    }

    #[derive(Encode, Decode)]
    #[syrup(label = "flags")]
    pub(crate) struct Flags<Str> {
        pub(crate) flags: Symbol<Str>,
    }
    #[derive(Encode, Decode)]
    #[syrup(label = "q", encode_where = { &'bytes [u8]: Into<__OData> }, decode_where = { '__output: 'bytes })]
    pub(crate) struct Q<'bytes> {
        #[syrup(encode = syrup::bytes::array::encode(self.q.as_slice()), decode = syrup::bytes::array::decode)]
        pub(crate) q: &'bytes [u8; 32],
    }

    #[derive(Encode, Decode)]
    #[syrup(label = "ecc")]
    pub(crate) struct Ecc<'bytes, Str> {
        pub(crate) curve: Curve<Str>,
        pub(crate) flags: Flags<Str>,
        pub(crate) q: Q<'bytes>,
    }
}

pub fn encode<'input, OData>(vkey: &'input VerifyingKey) -> TokenTree<OData>
where
    &'input [u8]: Into<OData>,
{
    __impl_vkey::Ecc {
        curve: __impl_vkey::Curve {
            kind: Symbol("Ed25519"),
        },
        flags: __impl_vkey::Flags {
            flags: Symbol("eddsa"),
        },
        q: __impl_vkey::Q { q: vkey.as_bytes() },
    }
    .encode()
}

pub fn decode<'tree, IData>(input: &'tree TokenTree<IData>) -> Result<VerifyingKey, DecodeError>
where
    IData: Bos<[u8]>,
{
    let ecc = __impl_vkey::Ecc::<'tree, &'tree str>::decode(input)?;
    if ecc.curve.kind.0 != "Ed25519" {
        return Err(DecodeError::Unexpected {
            expected: SyrupKind::Symbol(Some("Ed25519")),
            found: SyrupKind::Symbol(None),
        });
    }
    if ecc.flags.flags.0 != "eddsa" {
        return Err(DecodeError::Unexpected {
            expected: SyrupKind::Symbol(Some("eddsa")),
            found: SyrupKind::Symbol(None),
        });
    }
    Ok(VerifyingKey::from_bytes(ecc.q.q).unwrap())
}
