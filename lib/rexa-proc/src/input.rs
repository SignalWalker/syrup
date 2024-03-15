use proc_macro2::{Span, TokenStream};
use quote::{quote_spanned, ToTokens};
use syn::{parse_quote_spanned, spanned::Spanned, Expr, FnArg, LitBool, PatType, Receiver, Type};

use crate::{attr::ParseNestedMetaExt, Metadata};

pub(crate) enum DeliverInput<'cx> {
    Receiver(Receiver),
    Session {
        map: Expr,
    },
    Args {
        map: Expr,
    },
    Resolver {
        map: Expr,
    },
    // Iter { map: Expr },
    Syrup {
        ty: Type,
        map: Expr,
        context: &'cx Metadata,
    },
}

impl<'cx> DeliverInput<'cx> {
    fn is_resolver(&self) -> bool {
        matches!(self, Self::Resolver { .. })
    }

    fn process(context: &'cx Metadata, input: &mut PatType) -> syn::Result<Self> {
        let mut obj_attr_index = None;
        let mut res = None;
        for (i, attr) in input.attrs.iter().enumerate() {
            if attr.path().is_ident("arg") {
                obj_attr_index = Some(i);
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("session") {
                        res = Some(Self::Session {
                            map: meta.value_or_else(
                                || parse_quote_spanned! {meta.path.span()=> session},
                            )?,
                        });
                        Ok(())
                    } else if meta.path.is_ident("args") {
                        res = Some(Self::Args {
                            map: meta
                                .value_or_else(|| parse_quote_spanned! {meta.path.span()=> args})?,
                        });
                        Ok(())
                    } else if meta.path.is_ident("resolver") {
                        res = Some(Self::Resolver {
                            map: meta.value_or_else(
                                || parse_quote_spanned! {meta.path.span()=> resolver},
                            )?,
                        });
                        Ok(())
                    } else if meta.path.is_ident("syrup") {
                        res = Some(Self::Syrup {
                            ty: (*input.ty).clone(),
                            map: meta
                                .value_or_else(|| parse_quote_spanned! {meta.path.span()=> arg })?,
                            context,
                        });
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized arg attribute"))
                    }
                })?;
                break;
            }
        }
        if let Some(i) = obj_attr_index {
            input.attrs.remove(i);
        }

        match res {
            Some(arg) => Ok(arg),
            None => Ok(Self::Syrup {
                ty: (*input.ty).clone(),
                map: {
                    let error_t = &context.error_t;
                    let syrup = &context.syrup;
                    let ty = &input.ty;
                    parse_quote_spanned! {input.span()=>
                        <#ty as #syrup::FromSyrupItem>::from_syrup_item(&arg).ok_or_else(|| #error_t::unexpected(::std::stringify!(#ty), 0, arg))?
                    }
                },
                context,
            }),
        }
    }
}

impl<'cx> ToTokens for DeliverInput<'cx> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            DeliverInput::Receiver(receiver) => {
                let expr: Expr = parse_quote_spanned! {receiver.span()=> self};
                expr.to_tokens(tokens)
            }
            DeliverInput::Session { map } => map.to_tokens(tokens),
            DeliverInput::Args { map } => map.to_tokens(tokens),
            DeliverInput::Resolver { map } => map.to_tokens(tokens),
            // DeliverInput::Iter { map } => map.to_tokens(tokens),
            DeliverInput::Syrup {
                ty,
                map,
                context: Metadata { error_t, .. },
            } => {
                quote_spanned! {ty.span()=>
                    match args.pop() {
                        Some(arg) => #map,
                        None => return ::std::result::Result::Err(#error_t::missing(0, ::std::stringify!(#ty))), // ::std::todo!(::std::concat!("missing argument: ", #id, ": ", ::std::stringify!(#ty)))
                    }
                }
                .to_tokens(tokens)
            }
        }
    }
}

pub(crate) fn process_inputs<'cx, 'arg>(
    context: &'cx Metadata,
    inputs: impl IntoIterator<Item = &'arg mut FnArg>,
) -> syn::Result<(LitBool, Vec<DeliverInput<'cx>>)> {
    let mut takes_resolver = LitBool::new(false, Span::call_site());
    let mut res = Vec::<DeliverInput<'cx>>::new();
    for input in inputs.into_iter() {
        match input {
            FnArg::Typed(input) => {
                let input = DeliverInput::process(context, input)?;
                if input.is_resolver() {
                    takes_resolver = LitBool::new(true, input.span());
                }
                res.push(input);
            }
            FnArg::Receiver(receiver) => {
                res.push(DeliverInput::Receiver(receiver.clone()));
            }
        }
    }
    Ok((takes_resolver, res))
}
