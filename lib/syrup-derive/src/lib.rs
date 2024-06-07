use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, parse_quote_spanned, punctuated::Punctuated, spanned::Spanned,
    token::Comma, Attribute, DeriveInput, Expr, ExprStruct, FieldValue, FieldsNamed, FieldsUnnamed,
    Ident, ImplItemFn, Index, Lifetime, LifetimeParam, LitStr, Path, Signature, Token, Type,
};

struct OuterAttr {
    syrup: Path,
    label: LitStr,
    with: Option<Path>,
}

struct Context {
    outer: OuterAttr,
    cow_ty: Type,
    token_tree_ty: Type,
    decode_error_ty: Type,
    result_ty: Type,
    de_result_ty: Type,
    record_ty: Type,
    decode_ty: Type,
}

impl Context {
    fn new(outer: OuterAttr, input_lt: &Lifetime) -> Self {
        let syrup = &outer.syrup;
        let result_ty = parse_quote! { ::std::result::Result };
        let decode_error_ty = parse_quote! { #syrup::de::DecodeError };
        Self {
            record_ty: parse_quote! { #syrup::de::Record },
            cow_ty: parse_quote! { ::std::borrow::Cow },
            de_result_ty: parse_quote! { #result_ty<Self, #decode_error_ty<#input_lt>> },
            result_ty,
            decode_error_ty,
            token_tree_ty: parse_quote! { #syrup::TokenTree },
            decode_ty: parse_quote! { #syrup::Decode },
            outer,
        }
    }
}

impl OuterAttr {
    fn new(ident: &Ident, attrs: &[Attribute]) -> syn::Result<Self> {
        let mut label: Option<LitStr> = None;
        let mut syrup: Option<Path> = None;
        let mut with: Option<Path> = None;
        let mut transparent = false;
        for attr in attrs.iter() {
            if attr.path().is_ident("syrup") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("label") {
                        label = Some(meta.value()?.parse()?);
                        Ok(())
                    } else if meta.path.is_ident("with") {
                        with = Some(meta.value()?.parse()?);
                        Ok(())
                    } else if meta.path.is_ident("syrup") {
                        syrup = Some(meta.value()?.parse::<Path>()?);
                        Ok(())
                    } else if meta.path.is_ident("transparent") {
                        transparent = true;
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized syrup attribute"))
                    }
                })?;
            }
        }
        Ok(Self {
            label: label.unwrap_or_else(|| LitStr::new(ident.to_string().as_str(), ident.span())),
            with,
            syrup: syrup.unwrap_or_else(|| parse_quote! { ::syrup }),
        })
    }
}

struct FieldAttr {
    with: Option<Path>,
    from: Option<Path>,
    into: Option<Path>,
}

impl FieldAttr {
    fn new(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut with: Option<Path> = None;
        let mut from: Option<Path> = None;
        let mut into: Option<Path> = None;
        for attr in attrs.iter() {
            if attr.path().is_ident("syrup") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("with") {
                        with = Some(meta.value()?.parse()?);
                        Ok(())
                    } else if meta.path.is_ident("as") {
                        let ty = Some(meta.value()?.parse()?);
                        from.clone_from(&ty);
                        into = ty;
                        Ok(())
                    } else if meta.path.is_ident("from") {
                        from = Some(meta.value()?.parse()?);
                        Ok(())
                    } else if meta.path.is_ident("into") {
                        into = Some(meta.value()?.parse()?);
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized syrup attribute"))
                    }
                })?;
            }
        }
        Ok(Self { with, from, into })
    }
}

fn generate_decode(input: DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    fn generate_fields<'f>(
        Context {
            cow_ty,
            token_tree_ty,
            decode_error_ty,
            result_ty,
            ..
        }: &Context,
        fields: impl IntoIterator<Item = &'f syn::Field>,
    ) -> syn::Result<Punctuated<FieldValue, Comma>> {
        let mut res = Punctuated::new();
        for (i, field) in fields.into_iter().enumerate() {
            let ty = &field.ty;
            let (member, expected) = match field.ident.as_ref() {
                Some(id) => (
                    syn::Member::Named(id.clone()),
                    LitStr::new(id.to_string().as_ref(), id.span()),
                ),
                None => (
                    syn::Member::Unnamed(Index {
                        index: i as u32,
                        span: field.span(),
                    }),
                    LitStr::new(&format!("{i}th field"), field.span()),
                ),
            };
            let attr = FieldAttr::new(&field.attrs)?;

            let mut expr = parse_quote_spanned! {field.span()=>
                    stream.require(#cow_ty::Borrowed(#expected))?
            };

            expr = match attr.with {
                Some(with) => parse_quote_spanned! {with.span()=>
                    #with::decode(#expr)?
                },
                None => parse_quote_spanned! {field.span()=>
                    #expr.decode()?
                },
            };

            //if let Some(from) = attr.from {
            //    expr = parse_quote_spanned! {from.span()=>
            //        #ty::from::<#from>(#expr)
            //    };
            //}

            res.push(FieldValue {
                attrs: Vec::with_capacity(0),
                member,
                colon_token: Some(Token![:](field.span())),
                expr,
            });
        }
        Ok(res)
    }

    let input_lt = Lifetime::new("'__input", Span::call_site());
    let input_lt_param = LifetimeParam {
        attrs: Vec::with_capacity(0),
        lifetime: input_lt.clone(),
        colon_token: Default::default(),
        bounds: input
            .generics
            .lifetimes()
            .map(|param| param.lifetime.clone())
            .collect(),
    };
    let id = input.ident;

    let context = Context::new(OuterAttr::new(&id, &input.attrs)?, &input_lt);

    let Context {
        outer: OuterAttr { syrup, label, .. },
        cow_ty,
        token_tree_ty,
        decode_error_ty,
        result_ty,
        de_result_ty,
        record_ty,
        decode_ty,
        ..
    } = &context;

    let impl_params = &input.generics.params;
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

    let decode_sig: Signature =
        parse_quote! {fn decode(input: #syrup::de::TokenTree<#input_lt>) -> #de_result_ty};

    let decode_fn: ImplItemFn = match input.data {
        syn::Data::Union(u) => {
            return Err(syn::Error::new_spanned(
                u.union_token,
                "not yet implemented: union decode derivation",
            ))
        }
        syn::Data::Struct(data) => {
            let label_expr: Expr = parse_quote_spanned! {label.span()=> {
                let label = stream.require(#cow_ty::Borrowed("record label"))?.decode::<#syrup::Symbol<#input_lt>>()?;
                if &*label != #label {
                    return #result_ty::Err(#decode_error_ty::unexpected(
                        #cow_ty::Borrowed(#label),
                        #token_tree_ty::record(stream, span),
                    ))
                }
            }};
            let res_expr: Expr = match &data.fields {
                syn::Fields::Named(FieldsNamed { named: fields, .. })
                | syn::Fields::Unnamed(FieldsUnnamed {
                    unnamed: fields, ..
                }) => {
                    let res = ExprStruct {
                        attrs: Vec::with_capacity(0),
                        qself: None,
                        path: parse_quote! { Self },
                        brace_token: Default::default(),
                        fields: generate_fields(&context, fields)?,
                        dot2_token: None,
                        rest: None,
                    };
                    parse_quote! { #result_ty::Ok(#res) }
                }
                syn::Fields::Unit => parse_quote! {
                    #result_ty::Ok(Self)
                },
            };

            parse_quote! {
                #decode_sig {
                    match input {
                        #token_tree_ty::Record(#record_ty {
                            mut stream,
                            span
                        }) => {
                            #label_expr;
                            #res_expr
                        },
                        tree => #result_ty::Err(#decode_error_ty::unexpected(
                            #cow_ty::Borrowed(#label),
                            tree,
                        ))
                    }
                }
            }
        }
        syn::Data::Enum(data) => {
            parse_quote! {
                #decode_sig {
                    ::std::todo!(::std::concat!("decode derivation for ", #label))
                }
            }
        }
    };

    Ok(quote! {
        #[automatically_derived]
        impl<#input_lt_param, #impl_params> #syrup::Decode<#input_lt> for #id #ty_generics #where_clause {
            #decode_fn
        }
    }.into())
}

#[proc_macro_derive(Decode, attributes(syrup))]
pub fn derive_decode(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match generate_decode(input) {
        Ok(res) => res,
        Err(e) => e.to_compile_error().into(),
    }
}

fn generate_encode(input: &DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    let outer = OuterAttr::new(&input.ident, &input.attrs)?;

    let syrup = &outer.syrup;

    let id = &input.ident;

    let impl_params = &input.generics.params;
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

    let res = quote! {
        #[automatically_derived]
        impl<'o, #impl_params> #syrup::Encode<'o> for #id #ty_generics #where_clause {
            fn to_tokens_spanned(self, span: #syrup::Span) -> #syrup::TokenTree<'o> {
                todo!()
            }
        }

        #[automatically_derived]
        impl<'o, #impl_params> #syrup::Encode<'o> for &#id #ty_generics #where_clause {
            fn to_tokens_spanned(self, span: #syrup::Span) -> #syrup::TokenTree<'o> {
                todo!()
            }
        }
    };

    Ok(res.into())
}

#[proc_macro_derive(Encode, attributes(syrup))]
pub fn derive_encode(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate_encode(&input) {
        Ok(res) => res,
        Err(e) => e.to_compile_error().into(),
    }
}
