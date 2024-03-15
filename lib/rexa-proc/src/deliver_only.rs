use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{
    parse_quote_spanned, spanned::Spanned, Attribute, Expr, LitBool, LitStr, ReturnType, Signature,
};

use crate::{process_inputs, DeliverInput, Metadata};

pub(crate) struct DeliverOnlyFn<'context> {
    context: &'context Metadata,
    pub(crate) attr: DeliverOnlyAttr,
    ident: Ident,
    inputs: Vec<DeliverInput<'context>>,
    output: ReturnType,
}

impl<'cx> DeliverOnlyFn<'cx> {
    pub(crate) fn process(
        context: &'cx Metadata,
        attr: DeliverOnlyAttr,
        sig: &Signature,
        inputs: Vec<DeliverInput<'cx>>,
    ) -> Result<Self, syn::Error> {
        if let Some(token) = sig.asyncness {
            error!(token => "deliver_only object functions must not be async");
        }
        Ok(Self {
            context,
            attr,
            ident: sig.ident.clone(),
            inputs,
            output: sig.output.clone(),
        })
    }

    pub(crate) fn symbol(&self) -> LitStr {
        match &self.attr {
            DeliverOnlyAttr::Normal { symbol, .. } => match symbol {
                Some(symbol) => symbol.clone(),
                None => LitStr::new(&self.ident.to_string(), self.ident.span()),
            },
            _ => LitStr::new(&self.ident.to_string(), self.ident.span()),
        }
    }
}

impl<'cx> ToTokens for DeliverOnlyFn<'cx> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;
        let args = &self.inputs;
        let from_fn = &self.context.from_fn;

        let mut call: Expr = parse_quote_spanned! {self.ident.span()=> Self::#ident(#(#args),*)};
        call = parse_quote_spanned! {self.output.span()=> #call.map_err(#from_fn)};

        call.to_tokens(tokens)
    }
}

pub(crate) enum DeliverOnlyAttr {
    Verbatim,
    Normal {
        fallback: LitBool,
        symbol: Option<LitStr>,
    },
}

impl DeliverOnlyAttr {
    pub(crate) fn process<'cx>(
        context: &'cx Metadata,
        attr: &Attribute,
        sig: &mut Signature,
    ) -> syn::Result<(Self, Vec<DeliverInput<'cx>>)> {
        let mut verbatim = None;
        let mut fallback = None;
        let mut symbol = None;

        let (takes_resolver, inputs) = process_inputs(context, &mut sig.inputs)?;
        if takes_resolver.value {
            error!(takes_resolver => "deliver_only object functions cannot take a resolver parameter");
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("verbatim") {
                verbatim = Some(meta.path.clone());
                Ok(())
            } else if meta.path.is_ident("fallback") {
                fallback = Some(if let Ok(value) = meta.value() {
                    value.parse()?
                } else {
                    LitBool::new(true, meta.path.span())
                });
                Ok(())
            } else if meta.path.is_ident("symbol") {
                symbol = Some(meta.value()?.parse()?);
                Ok(())
            } else {
                Err(meta.error("unrecognized deliver_only property"))
            }
        })?;
        if verbatim.is_some() {
            Ok((Self::Verbatim, inputs))
        } else {
            Ok((
                Self::Normal {
                    fallback: fallback.unwrap_or_else(|| LitBool::new(false, attr.span())),
                    symbol,
                },
                inputs,
            ))
        }
    }
}
