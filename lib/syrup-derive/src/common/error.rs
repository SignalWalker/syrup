use proc_macro2::TokenStream;
use quote::quote;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not yet implemented: {0}")]
    Todo(&'static str),
    #[error("unrecognized syrup attribute: {0:?}")]
    UnrecognizedAttribute(syn::Meta),
    #[error(transparent)]
    Syn(#[from] syn::Error),
}

impl quote::ToTokens for Error {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        fn extend_with_error(tokens: &mut TokenStream, error: impl ToString) {
            let message = error.to_string();
            tokens.extend(quote! { ::core::compile_error!(#message) })
        }
        match self {
            Self::Syn(s) => tokens.extend(s.to_compile_error()),
            _ => extend_with_error(tokens, self),
        }
    }
}

macro_rules! errtodo {
    ($span:expr, $feature:expr) => {
        return Err(syn::Error::new(
            $span,
            ::std::concat!("not yet implemented: ", $feature),
        ))
    };
    ($feature:expr) => {
        errtodo!(Span::call_site(), $feature)
    };
    () => {
        errtodo!(Span::call_site(), "<unspecified>")
    };
}
