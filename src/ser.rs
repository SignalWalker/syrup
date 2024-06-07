use crate::de::{Span, TokenTree};

mod impl_encode;

// #[cfg(feature = "serde")]
// mod serde;
// #[cfg(feature = "serde")]
// pub use serde::*;

pub trait Encode<'output>: Sized {
    fn to_tokens_spanned(self, span: Span) -> TokenTree<'output>;
    fn to_tokens(self) -> TokenTree<'output> {
        self.to_tokens_spanned(Span::new(0, 0))
    }
}
