use ed25519_dalek::VerifyingKey;
use syrup::{symbol::EncodeAsSymbol, Decode, DecodeError, Encode, TokenTree};

mod __impl_vkey {
    use syrup::{Decode, Encode};

    #[derive(Encode, Decode)]
    #[syrup(label = "curve")]
    pub(crate) struct Curve {
        #[syrup(with = syrup::symbol)]
        pub(crate) kind: String,
    }

    #[derive(Encode, Decode)]
    #[syrup(label = "flags")]
    pub(crate) struct Flags {
        #[syrup(with = syrup::symbol)]
        pub(crate) flags: String,
    }
    #[derive(Encode, Decode)]
    #[syrup(label = "q")]
    pub(crate) struct Q {
        #[syrup(with = syrup::bytes::array)]
        pub(crate) q: [u8; 32],
    }
    #[derive(Encode, Decode)]
    #[syrup(label = "ecc")]
    pub(crate) struct Ecc {
        pub(crate) curve: Curve,
        pub(crate) flags: Flags,
        pub(crate) q: Q,
    }
}

pub fn encode(vkey: &VerifyingKey) -> TokenTree {
    __impl_vkey::Ecc {
        curve: __impl_vkey::Curve {
            kind: "Ed25519".into(),
        },
        flags: __impl_vkey::Flags {
            flags: "eddsa".into(),
        },
        q: __impl_vkey::Q {
            q: *vkey.as_bytes(),
        },
    }
    .encode()
}

pub fn decode(input: &TokenTree) -> Result<VerifyingKey, DecodeError<'_>> {
    let ecc = __impl_vkey::Ecc::decode(input)?;
    if &*ecc.curve.kind != "Ed25519" {
        return Err(DecodeError::unexpected(
            "Ed25519".into(),
            ecc.curve.kind.encode_as_symbol(),
        ));
    }
    if &*ecc.flags.flags != "eddsa" {
        return Err(DecodeError::unexpected(
            "eddsa".into(),
            ecc.flags.flags.encode_as_symbol(),
        ));
    }
    Ok(VerifyingKey::from_bytes(&ecc.q.q).unwrap())
}
