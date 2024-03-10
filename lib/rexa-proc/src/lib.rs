use std::collections::HashMap;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{
    parse::Parse, parse_macro_input, parse_quote, parse_quote_spanned, punctuated::Punctuated,
    spanned::Spanned, Arm, Attribute, Expr, ExprField, ExprMethodCall, FnArg, ImplItemFn, ItemImpl,
    LitBool, PatType, Path, Receiver, ReturnType, Signature, Token, Type, TypeTraitObject,
};

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

/// Like [std::todo], but it expands to return a [syn::parse::Error] instead of panic.
macro_rules! todo {
    ($span:expr;) => {
        return ::std::result::Result::Err(::syn::parse::Error::new($span, todo_str!()))
    };
    () => {
        todo!(::proc_macro2::Span::call_site();)
    };
    ($span:expr => $($arg:tt)+) => {
        return ::std::result::Result::Err(
            ::syn::parse::Error::new(
                $span,
                ::std::format!(todo_str!("{}"), ::std::format!($($arg)+))
                )
            )
    };
    ($($arg:tt)+) => {
        todo!(::proc_macro2::Span::call_site() => $($arg)+)
    };
}

macro_rules! error {
    ($span:expr => $($arg:tt)+) => {
        return ::std::result::Result::Err(::syn::parse::Error::new($span, ::std::format!($($arg)+)))
    };
    ($($arg:tt)+) => {
        error!(::proc_macro2::Span::call_site() => $($arg)+)
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
            todo!(arg.span();)
        }
        Ok(Self {
            rexa_crate: rexa_crate.unwrap_or(parse_quote!(::rexa)),
            syrup_crate: syrup_crate.unwrap_or(parse_quote!(::syrup)),
            futures_crate: futures_crate.unwrap_or(parse_quote!(::futures)),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeliverInputKind {
    Unknown,
    Session,
    Args,
    Resolver,
}

impl Parse for DeliverInputKind {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let kind_id = Ident::parse(input)?;
        match kind_id.to_string().as_str() {
            "session" => Ok(Self::Session),
            "args" => Ok(Self::Args),
            "resolver" => Ok(Self::Resolver),
            _ => error!(kind_id.span() => "unrecognized object fn kind"),
        }
    }
}

enum DeliverInput {
    Session { span: Span },
    Args { span: Span },
    Resolver { span: Span },
    Other(PatType),
}

impl DeliverInput {
    fn process(input: &mut PatType) -> syn::Result<Self> {
        let mut kind = DeliverInputKind::Unknown;
        let mut obj_attr_index = None;
        for (i, attr) in input.attrs.iter().enumerate() {
            if let Some(id) = attr.path().get_ident() {
                if id == "object" {
                    kind = attr.parse_args::<DeliverInputKind>()?;
                    obj_attr_index = Some(i);
                    break;
                }
            }
        }
        if let Some(i) = obj_attr_index {
            input.attrs.remove(i);
        }
        Ok(match kind {
            DeliverInputKind::Unknown => Self::Other(input.clone()),
            DeliverInputKind::Session => Self::Session { span: input.span() },
            DeliverInputKind::Args => Self::Args { span: input.span() },
            DeliverInputKind::Resolver => Self::Resolver { span: input.span() },
        })
    }
}

impl ToTokens for DeliverInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            DeliverInput::Session { span } => quote_spanned! {*span=> session}.to_tokens(tokens),
            DeliverInput::Args { span } => quote_spanned! {*span=> args}.to_tokens(tokens),
            DeliverInput::Resolver { span } => quote_spanned! {*span=> resolver}.to_tokens(tokens),
            DeliverInput::Other(input) => {
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

struct DeliverArgs {
    inputs: Vec<DeliverInput>,
}

impl DeliverArgs {
    fn from_inputs<'arg>(args: impl IntoIterator<Item = &'arg mut FnArg>) -> syn::Result<Self> {
        let mut inputs = Vec::new();
        for input in args.into_iter() {
            match input {
                FnArg::Typed(input) => {
                    inputs.push(DeliverInput::process(input)?);
                }
                _ => {}
            }
        }

        Ok(Self { inputs })
    }
}

impl ToTokens for DeliverArgs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut args = Punctuated::<&DeliverInput, Token![,]>::new();
        for input in self.inputs.iter() {
            args.push(input)
        }
        args.to_tokens(tokens)
    }
}

struct DeliverFn {
    ident: Ident,
    is_async: bool,
    inputs: DeliverArgs,
    output: ReturnType,
}

impl DeliverFn {
    fn process(sig: &mut Signature) -> syn::Result<Self> {
        Ok(Self {
            ident: sig.ident.clone(),
            is_async: sig.asyncness.is_some(),
            inputs: DeliverArgs::from_inputs(&mut sig.inputs)?,
            output: sig.output.clone(),
        })
    }
}

impl ToTokens for DeliverFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;
        let args = &self.inputs;

        let mut call: Expr = parse_quote_spanned! {self.ident.span()=> Self::#ident(self, #args)};
        if self.is_async {
            call = parse_quote_spanned! {self.ident.span()=> #call.await};
        }
        if let ReturnType::Type(_, ty) = &self.output {
            call = parse_quote_spanned! {ty.span()=> #call.map_err(From::from)};
        }

        call.to_tokens(tokens)
    }
}

struct DeliverOnlyFn {
    ident: Ident,
    inputs: DeliverArgs,
    output: ReturnType,
}

impl DeliverOnlyFn {
    fn process(sig: &mut Signature) -> Result<Self, syn::Error> {
        if let Some(token) = sig.asyncness {
            error!(token.span() => "deliver_only object functions must not be async");
        }
        Ok(Self {
            ident: sig.ident.clone(),
            inputs: DeliverArgs::from_inputs(&mut sig.inputs)?,
            output: sig.output.clone(),
        })
    }
}

impl ToTokens for DeliverOnlyFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;
        let args = &self.inputs;

        let mut call: Expr = parse_quote_spanned! {self.ident.span()=> Self::#ident(self, #args)};
        call = parse_quote_spanned! {self.output.span()=> #call.map_err(From::from)};

        call.to_tokens(tokens)
    }
}

enum ObjectFn {
    Deliver { f: DeliverFn, fallback: bool },
    DeliverOnly { f: DeliverOnlyFn, fallback: bool },
}

impl ObjectFn {
    fn process(f: &mut ImplItemFn) -> Result<Option<Self>, syn::Error> {
        let mut res = None;
        let mut attr_index = None;
        for (i, attr) in f.attrs.iter().enumerate() {
            if attr.path().is_ident("deliver") {
                attr_index = Some(i);
                let obj_fn = DeliverFn::process(&mut f.sig)?;
                let mut fallback = false;
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("fallback") {
                        fallback = true;
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized deliver property"))
                    }
                })?;
                res = Some(ObjectFn::Deliver {
                    f: obj_fn,
                    fallback,
                });
                break;
            } else if attr.path().is_ident("deliver_only") {
                attr_index = Some(i);
                let obj_fn = DeliverOnlyFn::process(&mut f.sig)?;
                let mut fallback = false;
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("fallback") {
                        fallback = true;
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized deliver_only property"))
                    }
                })?;
                res = Some(ObjectFn::DeliverOnly {
                    f: obj_fn,
                    fallback,
                });
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
    deliver_only_fns: HashMap<String, DeliverOnlyFn>,
    deliver_only_fallback: Option<DeliverOnlyFn>,
}

impl Parse for ObjectDef {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut base = ItemImpl::parse(input)?;
        let mut deliver_fallback = None;
        let mut deliver_fns = HashMap::new();
        let mut deliver_only_fallback = None;
        let mut deliver_only_fns = HashMap::new();
        for item in &mut base.items {
            match item {
                syn::ImplItem::Fn(f) => match ObjectFn::process(f)? {
                    Some(ObjectFn::Deliver { f: del, fallback }) => {
                        if fallback {
                            deliver_fallback = Some(del);
                        } else {
                            deliver_fns.insert(del.ident.to_string(), del);
                        }
                    }
                    Some(ObjectFn::DeliverOnly { f: del, fallback }) => {
                        if fallback {
                            deliver_only_fallback = Some(del);
                        } else {
                            deliver_only_fns.insert(del.ident.to_string(), del);
                        }
                    }
                    None => { /* skip */ }
                },
                syn::ImplItem::Verbatim(tt) => {
                    todo!(tt.span() => "handle verbatim impl item: {tt:?}")
                }
                syn::ImplItem::Macro(m) => error!(m.span() => "macro items not supported"),
                syn::ImplItem::Const(_) | syn::ImplItem::Type(_) => { /* ignore */ }
                item => todo!(item.span() => "unrecognized impl item: {item:?}"),
            }
        }
        Ok(Self {
            span: base.span(),
            self_ty: (*base.self_ty).clone(),
            base,
            deliver_fns,
            deliver_fallback,
            deliver_only_fns,
            deliver_only_fallback,
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

    let mut deliver_only_arms = deliver_only_fns
        .into_iter()
        .map(|(ident, del)| {
            parse_quote_spanned! {del.ident.span()=> #ident => #del }
        })
        .collect::<Vec<Arm>>();

    let mut deliver_arms = deliver_fns
        .into_iter()
        .map(|(ident, del)| {
            parse_quote_spanned! {del.ident.span()=> #ident => #del }
        })
        .collect::<Vec<Arm>>();

    deliver_only_arms.push(match deliver_only_fallback {
        Some(f) => parse_quote_spanned! {f.ident.span()=> id => #f},
        _ => parse_quote! { id => todo!("unrecognized deliver_only function: {id}") },
    });
    deliver_arms.push(match deliver_fallback {
        Some(f) => parse_quote_spanned! {f.ident.span()=> id => #f},
        _ => parse_quote! { id => todo!("unrecognized deliver function: {id}") },
    });

    quote_spanned! {span=>
        #base

        #[automatically_derived]
        impl #impl_generics #object_trait for #self_ty #where_clause {
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
                    match __id.as_str() {
                        #(#deliver_arms),*
                    }
                }.boxed()
            }
        }
    }
    .into()
}
