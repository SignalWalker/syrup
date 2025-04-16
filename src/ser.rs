use crate::de::TokenTree;

mod impl_encode;

pub trait Encode {
    /// Converts the given value to syrup tokens
    fn encode(&self) -> TokenTree;
}

mod private {
    /// Exists only to prevent external implementation of certain traits
    pub trait EncodeSealed {}
}

impl<T: Encode + ?Sized> private::EncodeSealed for T {}

pub trait EncodeExt: Encode + private::EncodeSealed {
    /// Converts the given value to syrup data
    fn encode_bytes(&self) -> Vec<u8>;
}

impl<T: ?Sized> EncodeExt for T
where
    T: Encode,
{
    #[inline]
    fn encode_bytes(&self) -> Vec<u8> {
        self.encode().to_bytes()
    }
}
