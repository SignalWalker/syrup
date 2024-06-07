use ed25519_dalek::VerifyingKey;
use syrup::{de::DecodeError, Decode, Encode, Span, TokenTree};

mod __impl_vkey {
    use syrup::{ByteArray, Decode, Encode, Symbol};

    #[derive(Encode, Decode)]
    #[syrup(label = "curve")]
    pub(super) struct Curve<'i> {
        pub(super) kind: Symbol<'i>,
    }
    #[derive(Encode, Decode)]
    #[syrup(label = "flags")]
    pub(super) struct Flags<'i> {
        pub(super) flags: Symbol<'i>,
    }
    #[derive(Encode, Decode)]
    #[syrup(label = "q")]
    pub(super) struct Q<'b> {
        pub(super) q: ByteArray<'b, 32>,
    }
    #[derive(Encode, Decode)]
    #[syrup(label = "ecc")]
    pub(super) struct Ecc<'i> {
        pub(super) curve: Curve<'i>,
        pub(super) flags: Flags<'i>,
        pub(super) q: Q<'i>,
    }
}

pub fn to_tokens_spanned(vkey: &VerifyingKey, span: Span) -> TokenTree<'_> {
    __impl_vkey::Ecc {
        curve: __impl_vkey::Curve {
            kind: "Ed25519".into(),
        },
        flags: __impl_vkey::Flags {
            flags: "eddsa".into(),
        },
        q: __impl_vkey::Q {
            q: vkey.as_bytes().into(),
        },
    }
    .to_tokens_spanned(span)
}

pub fn decode(input: TokenTree<'_>) -> Result<VerifyingKey, DecodeError<'_>> {
    let ecc = __impl_vkey::Ecc::decode(input)?;
    if &*ecc.curve.kind != "Ed25519" {
        return Err(DecodeError::unexpected(
            "Ed25519".into(),
            ecc.curve.kind.to_tokens(),
        ));
    }
    if &*ecc.flags.flags != "eddsa" {
        return Err(DecodeError::unexpected(
            "eddsa".into(),
            ecc.flags.flags.to_tokens(),
        ));
    }
    Ok(VerifyingKey::from_bytes(&ecc.q.q).unwrap())
}
