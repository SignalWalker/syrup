use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens, TokenStreamExt};
use std::collections::HashMap;
use syn::{
    meta::ParseNestedMeta,
    parse::{Parse, ParseBuffer},
    parse_macro_input, parse_quote, parse_quote_spanned,
    punctuated::Punctuated,
    spanned::Spanned,
    token, Arm, Attribute, Expr, ExprAssign, ExprField, ExprMethodCall, FnArg, ImplItemFn,
    ItemImpl, LitBool, Pat, PatType, Path, Receiver, ReturnType, Signature, Token, Type,
    TypeTraitObject,
};

// WARNING :: got way too "clever" with this one

macro_rules! todo_str {
    () => {
        ::std::concat!(
            "not yet implemented (@ line ",
            ::std::line!(),
            " in ",
            ::std::file!(),
            ")"
        )
    };
    ($extra:literal) => {
        ::std::concat!(
            "not yet implemented: ",
            $extra,
            " (@ line ",
            ::std::line!(),
            " in ",
            ::std::file!(),
            ")"
        )
    };
}

macro_rules! format_error {
    ($span:expr => $($arg:tt)+) => {
        ::syn::parse::Error::new_spanned($span, ::std::format!($($arg)+))
    };
    ($($arg:tt)+) => {
        ::syn::parse::Error::new(::proc_macro2::Span::call_site(), ::std::format!($($arg)+))
    };
}

/// Like [std::todo], but it expands to return a [syn::parse::Error] instead of panic.
macro_rules! todo {
    ($span:expr;) => {
        return ::std::result::Result::Err(format_error!($span => todo_str!()))
    };
    () => {
        return ::std::result::Result::Err(format_error!(todo_str!()))
    };
    ($span:expr => $($arg:tt)+) => {
        return ::std::result::Result::Err(
                format_error!($span => todo_str!("{}"), ::std::format!($($arg)+))
            )
    };
    ($($arg:tt)+) => {
        todo!(::proc_macro2::Span::call_site() => $($arg)+)
    };
}

macro_rules! error {
    ($span:expr => $($arg:tt)+) => {
        return Err(format_error!($span => $($arg)+))
    };
    ($($arg:tt)+) => {
        error!(::proc_macro2::Span::call_site() => $($arg)+)
    };
}

macro_rules! tokens_error {
    ($tokens:expr, $span:expr => $($arg:tt)+) => {
        format_error!($span => $($arg)+).into_compile_error().to_tokens($tokens)
    };
    ($tokens:expr => $($arg:tt)+) => {
        format_error!($($arg)+).into_compile_error().to_tokens($tokens)
    };
}

macro_rules! tokens_todo {
    ($tokens:expr, $span:expr => $($arg:tt)+) => {
        tokens_error!($tokens, $span => todo_str!("{}"), ::std::format!($($arg)+))
    };
    ($tokens:expr => $($arg:tt)+) => {
        tokens_error!($tokens => todo_str!("{}"), ::std::format!($($arg)+))
    };
    ($tokens:expr, $span:expr) => {
        tokens_error!($tokens, $span => todo_str!())
    };
    ($tokens:expr) => {
        tokens_error!($tokens => todo_str!())
    };
}

struct Metadata {
    rexa_crate: Path,
    syrup_crate: Path,
    futures_crate: Path,
}

impl Parse for Metadata {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let args = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
        let rexa_crate = None;
        let syrup_crate = None;
        let futures_crate = None;
        #[allow(clippy::never_loop)]
        for arg in args {
            todo!(arg;)
        }
        Ok(Self {
            rexa_crate: rexa_crate.unwrap_or(parse_quote!(::rexa)),
            syrup_crate: syrup_crate.unwrap_or(parse_quote!(::syrup)),
            futures_crate: futures_crate.unwrap_or(parse_quote!(::futures)),
        })
    }
}

enum DeliverInput {
    Session { span: Span },
    Args { span: Span },
    Resolver { span: Span },
    Syrup(PatType),
}

impl DeliverInput {
    fn is_resolver(&self) -> bool {
        match self {
            Self::Resolver { .. } => true,
            _ => false,
        }
    }
}

impl DeliverInput {
    fn process(input: &mut PatType) -> syn::Result<Self> {
        let mut obj_attr_index = None;
        let mut res = None;
        for (i, attr) in input.attrs.iter().enumerate() {
            if attr.path().is_ident("arg") {
                obj_attr_index = Some(i);
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("session") {
                        res = Some(Self::Session { span: input.span() });
                        Ok(())
                    } else if meta.path.is_ident("args") {
                        res = Some(Self::Args { span: input.span() });
                        Ok(())
                    } else if meta.path.is_ident("resolver") {
                        res = Some(Self::Resolver { span: input.span() });
                        Ok(())
                    } else if meta.path.is_ident("syrup") {
                        res = Some(Self::Syrup(input.clone()));
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
            None => match &*input.pat {
                Pat::Ident(pat) => {
                    // TODO :: This is convenient, but could probably lead to confusing errors
                    if pat.ident == "session" {
                        Ok(Self::Session { span: input.span() })
                    } else if pat.ident == "args" {
                        Ok(Self::Args { span: input.span() })
                    } else if pat.ident == "resolver" {
                        Ok(Self::Resolver { span: input.span() })
                    } else {
                        Ok(Self::Syrup(input.clone()))
                    }
                }
                _ => Ok(Self::Syrup(input.clone())),
            },
        }
    }
}

impl ToTokens for DeliverInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            DeliverInput::Session { span } => quote_spanned! {*span=> session}.to_tokens(tokens),
            DeliverInput::Args { span } => quote_spanned! {*span=> args}.to_tokens(tokens),
            DeliverInput::Resolver { span } => quote_spanned! {*span=> resolver}.to_tokens(tokens),
            DeliverInput::Syrup(input) => {
                let ty = &input.ty;
                let id = match &*input.pat {
                    syn::Pat::Ident(id) => id.ident.to_string(),
                    _ => "<unnamed>".to_owned(),
                };
                quote_spanned! {input.span()=>
                    match __args.next() {
                        Some(__item) => match <#ty>::from_syrup_item(__item) {
                            Ok(__res) => __res,
                            Err(__item) => ::std::todo!(::std::concat!("expected ", ::std::stringify!(#ty), ", got {:?}"), __item)
                        },
                        None => ::std::todo!(::std::concat!("missing argument: ", #id, ": ", ::std::stringify!(#ty)))
                    }
                }.to_tokens(tokens)
            }
        }
    }
}

fn process_inputs<'arg>(
    inputs: impl IntoIterator<Item = &'arg mut FnArg>,
) -> syn::Result<Vec<DeliverInput>> {
    let mut res = Vec::<DeliverInput>::new();
    for input in inputs.into_iter() {
        match input {
            FnArg::Typed(input) => {
                res.push(DeliverInput::process(input)?);
            }
            _ => {}
        }
    }
    Ok(res)
}

struct DeliverFn {
    attr: DeliverAttr,
    ident: Ident,
    is_async: bool,
    inputs: Vec<DeliverInput>,
    output: ReturnType,
}

impl DeliverFn {
    fn process(attr: DeliverAttr, sig: &mut Signature) -> syn::Result<Self> {
        Ok(Self {
            attr,
            ident: sig.ident.clone(),
            is_async: sig.asyncness.is_some(),
            inputs: process_inputs(&mut sig.inputs)?,
            output: sig.output.clone(),
        })
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

impl ToTokens for DeliverFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // this function does a lot of magic to figure how to handle the returned type

        let ident = &self.ident;
        let args = &self.inputs;

        let mut call: Expr =
            parse_quote_spanned! {self.ident.span()=> Self::#ident(self, #(#args),*)};
        if self.is_async {
            call = parse_quote_spanned! {self.ident.span()=> #call.await};
        }

        let DeliverAttr::Normal { resolution, .. } = &self.attr else {
            return call.to_tokens(tokens);
        };

        match &resolution {
            DeliverResolution::Internal(internal) => {
                call = parse_quote! { #call.map_err(From::from) }
            }
            DeliverResolution::External(external) => {
                let (ok_span, err_span) = match get_result_spans(true, &self.output) {
                    Ok(spans) => spans,
                    Err(e) => return e.into_compile_error().to_tokens(tokens),
                };

                match external {
                    ResolveExternal::Normal { ok_map, err_map } => {
                        let ok_arm: Arm = parse_quote_spanned! {ok_span.span()=> Ok(__ok) => {
                            resolver.fulfill(#ok_map, None, Default::default()).await.map_err(From::from)
                        }};
                        let err_arm: Arm = parse_quote_spanned! {err_span.span()=> Err(__err) => resolver.break_promise(#err_map).await.map_err(From::from)};
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
                            resolver.fulfill(#res_map, None, Default::default()).await.map_err(From::from)
                        }};
                    }
                    ResolveExternal::AlwaysBreak { res_map } => {
                        call = parse_quote_spanned! { self.output.span() => {
                            let __res = #call;
                            resolver.break_promise(#res_map).await.map_err(From::from)
                        }};
                    }
                }
            }
        }

        call.to_tokens(tokens)
    }
}

struct DeliverOnlyFn {
    attr: DeliverOnlyAttr,
    ident: Ident,
    inputs: Vec<DeliverInput>,
    output: ReturnType,
}

impl DeliverOnlyFn {
    fn process(attr: DeliverOnlyAttr, sig: &mut Signature) -> Result<Self, syn::Error> {
        if let Some(token) = sig.asyncness {
            error!(token => "deliver_only object functions must not be async");
        }
        Ok(Self {
            attr,
            ident: sig.ident.clone(),
            inputs: process_inputs(&mut sig.inputs)?,
            output: sig.output.clone(),
        })
    }
}

impl ToTokens for DeliverOnlyFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;
        let args = &self.inputs;

        let mut call: Expr =
            parse_quote_spanned! {self.ident.span()=> Self::#ident(self, #(#args),*)};
        call = parse_quote_spanned! {self.output.span()=> #call.map_err(From::from)};

        call.to_tokens(tokens)
    }
}

mod attr;
use attr::*;

enum ResolveExternal {
    Normal { ok_map: Expr, err_map: Expr },
    AlwaysFulfill { res_map: Expr },
    AlwaysBreak { res_map: Expr },
}

impl ResolveExternal {}

impl From<Span> for ResolveExternal {
    fn from(span: Span) -> Self {
        Self::Normal {
            ok_map: parse_quote! { [&__ok] },
            err_map: parse_quote! { __err },
        }
    }
}

impl TryFrom<AttrOptionSet> for ResolveExternal {
    type Error = syn::Error;

    fn try_from(value: AttrOptionSet) -> Result<Self, Self::Error> {
        todo!(value;)
    }
}

enum DeliverResolution {
    /// Resolution handled by impl
    Internal(LitBool),
    /// Resolution handled by macro
    External(ResolveExternal),
}

impl From<Span> for DeliverResolution {
    fn from(span: Span) -> Self {
        Self::External(ResolveExternal::from(span))
    }
}

impl TryFrom<AttrOption> for DeliverResolution {
    type Error = syn::Error;
    fn try_from(value: AttrOption) -> Result<Self, Self::Error> {
        match value {
            AttrOption::Set(mut set) => {
                let internal = set.remove_flag_or("internal", false)?;

                let res = if internal.value == true {
                    Ok(Self::Internal(internal))
                } else {
                    let external = set.remove_implicit_set("external")?;
                    Ok(Self::External(external))
                };
                set.into_unrecognized_err()?;
                res
            }
            AttrOption::Flag(flag) => Ok(Self::Internal(flag.value)),
        }
    }
}

struct DeliverOutput {
    resolution: DeliverResolution,
}

impl From<Span> for DeliverOutput {
    fn from(span: Span) -> Self {
        Self {
            resolution: From::from(span),
        }
    }
}

impl TryFrom<AttrOptionSet> for DeliverOutput {
    type Error = syn::Error;

    fn try_from(mut set: AttrOptionSet) -> Result<Self, Self::Error> {
        let resolution = set.remove_implicit("resolve")?;
        set.into_unrecognized_err()?;
        Ok(Self { resolution })
    }
}

enum DeliverAttr {
    Verbatim,
    Normal {
        fallback: LitBool,
        resolution: DeliverResolution,
    },
}

impl DeliverAttr {
    fn process(attr: &Attribute) -> syn::Result<Self> {
        let mut is_verbatim = false;
        let mut fallback = None;
        let mut resolution = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("verbatim") {
                is_verbatim = true;
                Ok(())
            } else if meta.path.is_ident("fallback") {
                fallback = Some(LitBool::new(true, meta.path.span()));
                Ok(())
            } else if meta.path.is_ident("always_ok") {
                let res_map = if let Ok(value) = meta.value() {
                    value.parse()?
                } else {
                    parse_quote! { [&__res] }
                };
                resolution = Some(DeliverResolution::External(
                    ResolveExternal::AlwaysFulfill { res_map },
                ));
                Ok(())
            } else {
                Err(meta.error("unrecognized deliver property"))
            }
        })?;

        if is_verbatim {
            Ok(Self::Verbatim)
        } else {
            Ok(Self::Normal {
                fallback: fallback.unwrap_or_else(|| LitBool::new(false, attr.span())),
                resolution: resolution.unwrap_or_else(|| DeliverResolution::from(attr.span())),
            })
        }
    }
}

enum DeliverOnlyAttr {
    Verbatim,
    Normal { fallback: LitBool },
}

impl DeliverOnlyAttr {
    fn process(attr: &Attribute) -> syn::Result<Self> {
        let mut opts = AttrOptionSet::process(attr)?;
        let fallback = opts.remove_flag_or("fallback", false)?;
        let verbatim = opts.remove_flag_or("verbatim", false)?;
        opts.into_unrecognized_err()?;
        if verbatim.value {
            Ok(Self::Verbatim)
        } else {
            Ok(Self::Normal { fallback })
        }
    }
}

enum ObjectFn {
    Deliver(DeliverFn),
    DeliverOnly(DeliverOnlyFn),
}

impl ObjectFn {
    fn process(f: &mut ImplItemFn) -> Result<Option<Self>, syn::Error> {
        let mut res = None;
        let mut attr_index = None;
        for (i, attr) in f.attrs.iter().enumerate() {
            if attr.path().is_ident("deliver") {
                attr_index = Some(i);
                let attr = DeliverAttr::process(attr)?;
                res = Some(Self::Deliver(DeliverFn::process(attr, &mut f.sig)?));
                break;
            } else if attr.path().is_ident("deliver_only") {
                attr_index = Some(i);
                let attr = DeliverOnlyAttr::process(attr)?;
                res = Some(Self::DeliverOnly(DeliverOnlyFn::process(attr, &mut f.sig)?));
                break;
            }
        }

        if let Some(i) = attr_index {
            f.attrs.remove(i);
        }

        Ok(res)
    }
}

struct ObjectDef {
    span: Span,
    self_ty: Type,

    base: ItemImpl,

    deliver_fns: HashMap<String, DeliverFn>,
    deliver_fallback: Option<DeliverFn>,
    deliver_verbatim: Option<DeliverFn>,

    deliver_only_fns: HashMap<String, DeliverOnlyFn>,
    deliver_only_fallback: Option<DeliverOnlyFn>,
    deliver_only_verbatim: Option<DeliverOnlyFn>,
}

impl Parse for ObjectDef {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut base = ItemImpl::parse(input)?;
        let mut deliver_fallback = None;
        let mut deliver_fns = HashMap::new();
        let mut deliver_verbatim = None;
        let mut deliver_only_fallback = None;
        let mut deliver_only_fns = HashMap::new();
        let mut deliver_only_verbatim = None;
        for item in &mut base.items {
            match item {
                syn::ImplItem::Fn(f) => match ObjectFn::process(f)? {
                    Some(ObjectFn::Deliver(del)) => {
                        let DeliverAttr::Normal { fallback, .. } = &del.attr else {
                            deliver_verbatim = Some(del);
                            continue;
                        };
                        if fallback.value {
                            deliver_fallback = Some(del);
                        } else {
                            deliver_fns.insert(del.ident.to_string(), del);
                        }
                    }
                    Some(ObjectFn::DeliverOnly(del)) => {
                        let DeliverOnlyAttr::Normal { fallback, .. } = &del.attr else {
                            deliver_only_verbatim = Some(del);
                            continue;
                        };
                        if fallback.value {
                            deliver_only_fallback = Some(del);
                        } else {
                            deliver_only_fns.insert(del.ident.to_string(), del);
                        }
                    }
                    None => { /* skip */ }
                },
                syn::ImplItem::Verbatim(tt) => {
                    todo!(&tt => "handle verbatim impl item: {tt:?}")
                }
                syn::ImplItem::Macro(m) => {
                    error!(m => "macro items not supported in #[impl_object] blocks")
                }
                syn::ImplItem::Const(_) | syn::ImplItem::Type(_) => { /* ignore */ }
                item => todo!(&item => "unrecognized impl item: {item:?}"),
            }
        }
        Ok(Self {
            span: base.span(),
            self_ty: (*base.self_ty).clone(),
            base,
            deliver_fns,
            deliver_fallback,
            deliver_verbatim,
            deliver_only_fns,
            deliver_only_fallback,
            deliver_only_verbatim,
        })
    }
}

#[proc_macro_attribute]
pub fn impl_object(
    attr_input: proc_macro::TokenStream,
    obj_input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let Metadata {
        rexa_crate,
        syrup_crate,
        futures_crate,
        ..
    } = parse_macro_input!(attr_input as Metadata);
    let ObjectDef {
        span,
        self_ty,
        base,
        deliver_fns,
        deliver_only_fns,
        deliver_fallback,
        deliver_only_fallback,
        deliver_verbatim,
        deliver_only_verbatim,
        ..
    } = parse_macro_input!(obj_input as ObjectDef);

    let object_trait: Path = parse_quote!(#rexa_crate::captp::object::Object);
    let session_type: TypeTraitObject =
        parse_quote!(dyn #rexa_crate::captp::AbstractCapTpSession + ::std::marker::Sync);
    let syrup_item: Path = parse_quote!(#syrup_crate::Item);
    let error_type: Type = parse_quote!(
        ::std::boxed::Box<
            dyn ::std::error::Error + ::std::marker::Send + ::std::marker::Sync + 'static,
        >
    );
    let result_type: Type = parse_quote!(::std::result::Result<(), #error_type>);
    let args_type: Type = parse_quote!(::std::vec::Vec<#syrup_item>);
    let resolver_type: Type = parse_quote!(#rexa_crate::captp::GenericResolver);

    let (impl_generics, _, where_clause) = base.generics.split_for_impl();

    let deliver_only: ImplItemFn = if let Some(verbatim) = deliver_only_verbatim {
        parse_quote! {
            fn deliver_only(&self, session: &(#session_type), args: #args_type) -> #result_type {
                use #syrup_crate::FromSyrupItem;
                let mut __args = args.iter();
                #verbatim
            }
        }
    } else {
        let mut deliver_only_arms = deliver_only_fns
            .into_iter()
            .map(|(ident, del)| {
                parse_quote_spanned! {del.ident.span()=> #ident => #del }
            })
            .collect::<Vec<Arm>>();
        deliver_only_arms.push(match deliver_only_fallback {
            Some(f) => parse_quote_spanned! {f.ident.span()=> id => #f},
            _ => parse_quote! { id => todo!("unrecognized deliver_only function: {id}") },
        });
        parse_quote! {
            fn deliver_only(&self, session: &(#session_type), args: #args_type) -> #result_type {
                use #syrup_crate::FromSyrupItem;
                let mut __args = args.iter();
                let __id = match __args.next() {
                    Some(#syrup_item::Symbol(__id)) => __id,
                    Some(__item) => todo!("first argument to impl_object deliver_only is not symbol: {__item:?}"),
                    None => todo!("no arguments to impl_object deliver_only")
                };
                match __id.as_str() {
                    #(#deliver_only_arms),*
                }
            }
        }
    };

    let deliver: ImplItemFn = if let Some(verbatim) = deliver_verbatim {
        parse_quote! {
            fn deliver_only(&self, session: &(#session_type), args: #args_type) -> #result_type {
                use #syrup_crate::FromSyrupItem;
                let mut __args = args.iter();
                #verbatim
            }
        }
    } else {
        let mut deliver_arms = deliver_fns
            .into_iter()
            .map(|(ident, del)| {
                parse_quote_spanned! {del.ident.span()=> #ident => #del }
            })
            .collect::<Vec<Arm>>();

        deliver_arms.push(match deliver_fallback {
            Some(f) => parse_quote_spanned! {f.ident.span()=> id => #f},
            _ => parse_quote! { id => todo!("unrecognized deliver function: {id}") },
        });
        parse_quote! {
            fn deliver<'result>(&'result self, session: &'result (#session_type), args: #args_type, resolver: #resolver_type) -> #futures_crate::future::BoxFuture<'result, #result_type> {
                use #futures_crate::FutureExt;
                use #syrup_crate::FromSyrupItem;
                async move {
                    let mut __args = args.iter();
                    let __id = match __args.next() {
                        Some(#syrup_item::Symbol(__id)) => __id,
                        Some(__item) => todo!("first argument to impl_object deliver is not symbol: {__item:?}"),
                        None => todo!("no arguments to impl_object deliver")
                    };
                    // `let ...` so that we get more helpful errors
                    let __res: #result_type = match __id.as_str() {
                        #(#deliver_arms),*
                    };
                    __res
                }.boxed()
            }
        }
    };

    quote_spanned! {span=>
        #base

        #[automatically_derived]
        impl #impl_generics #object_trait for #self_ty #where_clause {
            #deliver_only

            #deliver
        }
    }
    .into()
}
