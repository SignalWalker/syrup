use crate::de::TokenTree;

mod impl_encode;

pub trait Encode<'input, OData> {
    /// Converts the given value to syrup tokens
    fn encode(&'input self) -> TokenTree<OData>;
}

pub trait EncodeInto<'input> {
    /// Write the given value, as syrup data, to the given writer.
    ///
    /// Should be equivalent to `self.encode().write_bytes(w)`.
    fn encode_into(&'input self, w: &mut impl std::io::Write) -> std::io::Result<usize>;
}

mod private {
    /// Exists only to prevent external implementation of certain traits
    pub trait EncodeSealed<'input, OData> {}

    pub trait EncodeIntoSealed<'input> {}
}

impl<'input, OData, T: Encode<'input, OData> + ?Sized> private::EncodeSealed<'input, OData> for T {}

impl<'input, T: EncodeInto<'input> + ?Sized> private::EncodeIntoSealed<'input> for T {}

pub trait EncodeIntoExt<'input>: EncodeInto<'input> + private::EncodeIntoSealed<'input> {
    /// Converts the given value to syrup data.
    fn encode_bytes(&'input self) -> Vec<u8>;
}

impl<'input, T: ?Sized> EncodeIntoExt<'input> for T
where
    T: EncodeInto<'input>,
{
    fn encode_bytes(&'input self) -> Vec<u8> {
        let mut res = Vec::new();
        drop(self.encode_into(&mut res));
        res
    }
}
