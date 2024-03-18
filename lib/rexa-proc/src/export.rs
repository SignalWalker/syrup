use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{parse_quote_spanned, spanned::Spanned, Expr, FnArg, PatType, Receiver, Signature};

pub(crate) struct ExportFn {
    ident: Ident,
    inputs: Vec<ExportInput>,
}

impl ToTokens for ExportFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;
        // TODO :: custom inputs
        // let args = &self.inputs;
        // let call: Expr = parse_quote_spanned! {ident.span()=> Self::#ident(#(#args),*) };
        let call: Expr =
            parse_quote_spanned! {ident.span()=> Self::#ident(self, remote_key, position) };
        call.to_tokens(tokens);
    }
}

impl ExportFn {
    pub(crate) fn process(sig: &mut Signature) -> syn::Result<Self> {
        Ok(Self {
            ident: sig.ident.clone(),
            inputs: sig
                .inputs
                .iter()
                .map(|input| match input {
                    FnArg::Receiver(rec) => ExportInput::Receiver(rec.clone()),
                    FnArg::Typed(pat) => ExportInput::Arg(pat.clone()),
                })
                .collect(),
        })
    }
}

enum ExportInput {
    Receiver(Receiver),
    Arg(PatType),
}

impl ToTokens for ExportInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            ExportInput::Receiver(receiver) => {
                let expr: Expr = parse_quote_spanned! {receiver.span()=> self };
                expr
            }
            ExportInput::Arg(arg) => {
                let id = match &*arg.pat {
                    syn::Pat::Ident(id) => &id.ident,
                    _ => return tokens_todo!(tokens, arg),
                };
                parse_quote_spanned! {id.span()=> #id }
            }
        }
        .to_tokens(tokens);
    }
}
