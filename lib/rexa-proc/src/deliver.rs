use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{
    parse_quote, parse_quote_spanned, spanned::Spanned, Arm, Attribute, Expr, LitBool, LitStr,
    ReturnType, Signature, Type,
};

use crate::{attr::ParseNestedMetaExt, process_inputs, DeliverInput, Metadata};

pub(crate) struct DeliverFn<'cx> {
    context: &'cx Metadata,
    pub(crate) attr: DeliverAttr,
    ident: Ident,
    is_async: bool,
    inputs: Vec<DeliverInput<'cx>>,
    output: ReturnType,
}

impl<'cx> DeliverFn<'cx> {
    pub(crate) fn process(
        context: &'cx Metadata,
        attr: DeliverAttr,
        sig: &Signature,
        inputs: Vec<DeliverInput<'cx>>,
    ) -> syn::Result<Self> {
        Ok(Self {
            context,
            attr,
            ident: sig.ident.clone(),
            is_async: sig.asyncness.is_some(),
            inputs,
            output: sig.output.clone(),
        })
    }

    pub(crate) fn symbol(&self) -> LitStr {
        match &self.attr {
            DeliverAttr::Normal { symbol, .. } => match symbol {
                Some(symbol) => symbol.clone(),
                None => LitStr::new(&self.ident.to_string(), self.ident.span()),
            },
            _ => LitStr::new(&self.ident.to_string(), self.ident.span()),
        }
    }
}

fn get_result_spans<'r>(
    expecting_result: bool,
    output: &'r ReturnType,
) -> syn::Result<(&'r dyn Spanned, &'r dyn Spanned)> {
    match output {
        ReturnType::Default => Ok((output, output)),
        ReturnType::Type(_, ty) => match &**ty {
            Type::Path(tpath) => {
                let Some(final_segment) = tpath.path.segments.last() else {
                    return Ok((tpath, tpath));
                };
                if !expecting_result && final_segment.ident != "Result" {
                    return Ok((final_segment, final_segment));
                }
                match &final_segment.arguments {
                    syn::PathArguments::AngleBracketed(args) => {
                        let mut arg_iter = args.args.iter();
                        let Some(ok) = arg_iter.next() else {
                            return Ok((args, args));
                        };
                        Ok((
                            ok as &dyn Spanned,
                            arg_iter
                                .next()
                                .map(|arg| arg as &dyn Spanned)
                                .unwrap_or_else(|| &args.args as &dyn Spanned),
                        ))
                    }
                    _ => Ok((tpath, tpath)),
                }
            }
            _ => Ok((ty, ty)),
        },
    }
}

impl<'cx> ToTokens for DeliverFn<'cx> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // this function does a lot of magic to figure how to handle the returned type

        let ident = &self.ident;
        let args = &self.inputs;
        let from_fn = &self.context.from_fn;

        let mut call: Expr = parse_quote_spanned! {self.ident.span()=> Self::#ident(#(#args),*)};
        if self.is_async {
            call = parse_quote_spanned! {self.ident.span()=> #call.await};
        }

        let DeliverAttr::Normal { resolution, .. } = &self.attr else {
            return call.to_tokens(tokens);
        };

        match &resolution {
            DeliverResolution::Internal(internal) => {
                call = parse_quote_spanned! {internal.span()=> #call.map_err(#from_fn) }
            }
            DeliverResolution::External(external) => {
                let (ok_span, err_span) = match get_result_spans(true, &self.output) {
                    Ok(spans) => spans,
                    Err(e) => return e.into_compile_error().to_tokens(tokens),
                };

                match external {
                    ResolveExternal::Normal { ok_map, err_map } => {
                        let ok_arm: Arm = parse_quote_spanned! {ok_span.span()=> Ok(__ok) => {
                            resolver.fulfill(#ok_map, None, Default::default()).await.map_err(#from_fn)
                        }};
                        let err_arm: Arm = parse_quote_spanned! {err_span.span()=> Err(__err) => resolver.break_promise(#err_map).await.map_err(#from_fn)};
                        call = parse_quote_spanned! {self.output.span() => {
                            match #call {
                                #ok_arm,
                                #err_arm
                            }
                        }};
                    }
                    ResolveExternal::AlwaysFulfill { res_map } => {
                        call = parse_quote_spanned! { self.output.span() => {
                            let __res = #call;
                            resolver.fulfill(#res_map, None, Default::default()).await.map_err(#from_fn)
                        }};
                    }
                    ResolveExternal::AlwaysBreak { res_map } => {
                        call = parse_quote_spanned! { self.output.span() => {
                            let __res = #call;
                            resolver.break_promise(#res_map).await.map_err(#from_fn)
                        }};
                    }
                }
            }
        }

        call.to_tokens(tokens)
    }
}

pub(crate) enum ResolveExternal {
    Normal { ok_map: Expr, err_map: Expr },
    AlwaysFulfill { res_map: Expr },
    AlwaysBreak { res_map: Expr },
}

impl ResolveExternal {
    fn normal(ok_map: Expr, err_map: Expr) -> Self {
        Self::Normal { ok_map, err_map }
    }

    fn always_fulfill(res_map: Expr) -> Self {
        Self::AlwaysFulfill { res_map }
    }

    fn always_break(res_map: Expr) -> Self {
        Self::AlwaysBreak { res_map }
    }
}

pub(crate) enum DeliverResolution {
    /// Resolution handled by impl
    Internal(LitBool),
    /// Resolution handled by macro
    External(ResolveExternal),
}

impl DeliverResolution {
    fn normal(ok_map: Expr, err_map: Expr) -> Self {
        Self::External(ResolveExternal::normal(ok_map, err_map))
    }

    fn always_fulfill(res_map: Expr) -> Self {
        Self::External(ResolveExternal::always_fulfill(res_map))
    }

    fn always_break(res_map: Expr) -> Self {
        Self::External(ResolveExternal::always_break(res_map))
    }
}

pub(crate) enum DeliverAttr {
    Verbatim,
    Normal {
        fallback: LitBool,
        resolution: DeliverResolution,
        symbol: Option<LitStr>,
    },
}

impl DeliverAttr {
    pub(crate) fn process<'cx>(
        context: &'cx Metadata,
        attr: &Attribute,
        sig: &mut Signature,
    ) -> syn::Result<(Self, Vec<DeliverInput<'cx>>)> {
        const RESOLVER_CONFLICT: &'static str = "this function takes a resolver parameter, and, therefore, resolution must be handled internally";

        let mut is_verbatim = false;
        let mut fallback = None;
        let mut resolution = None;
        let mut symbol = None;

        let mut ok_map: Expr = parse_quote! { [&__ok] };
        let mut err_map: Expr = parse_quote! { &__err };

        let (takes_resolver, inputs) = process_inputs(context, &mut sig.inputs)?;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("verbatim") {
                is_verbatim = true;
                Ok(())
            } else if meta.path.is_ident("fallback") {
                fallback = Some(LitBool::new(true, meta.path.span()));
                Ok(())
            } else if meta.path.is_ident("ok") {
                if takes_resolver.value {
                    return Err(meta.error(RESOLVER_CONFLICT));
                }
                ok_map = meta.value()?.parse()?;
                Ok(())
            } else if meta.path.is_ident("err") {
                if takes_resolver.value {
                    return Err(meta.error(RESOLVER_CONFLICT));
                }
                err_map = meta.value()?.parse()?;
                Ok(())
            } else if meta.path.is_ident("always_fulfill") {
                if takes_resolver.value {
                    return Err(meta.error(RESOLVER_CONFLICT));
                }
                resolution = Some(DeliverResolution::always_fulfill(
                    meta.value_or_else(|| parse_quote! { [&__res] })?,
                ));
                Ok(())
            } else if meta.path.is_ident("always_break") {
                if takes_resolver.value {
                    return Err(meta.error(RESOLVER_CONFLICT));
                }
                resolution = Some(DeliverResolution::always_break(
                    meta.value_or_else(|| parse_quote! { &__res })?,
                ));
                Ok(())
            } else if meta.path.is_ident("symbol") {
                symbol = Some(meta.value()?.parse::<LitStr>()?);
                Ok(())
            } else {
                Err(meta.error("unrecognized deliver property"))
            }
        })?;

        if is_verbatim {
            Ok((Self::Verbatim, inputs))
        } else {
            Ok((
                Self::Normal {
                    fallback: fallback.unwrap_or_else(|| LitBool::new(false, attr.span())),
                    resolution: resolution.unwrap_or_else(|| {
                        if takes_resolver.value {
                            DeliverResolution::Internal(takes_resolver)
                        } else {
                            DeliverResolution::normal(ok_map, err_map)
                        }
                    }),
                    symbol,
                },
                inputs,
            ))
        }
    }
}
