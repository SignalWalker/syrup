use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_quote, parse_quote_spanned, punctuated::Punctuated, spanned::Spanned, token::Comma,
    DeriveInput, Expr, ExprStruct, FieldValue, FieldsNamed, FieldsUnnamed, GenericParam, Ident,
    ImplItemFn, Index, Lifetime, LifetimeParam, LitStr, Member, Signature, Token, TypeParam,
    WhereClause,
};

use crate::{Context, FieldAttr, OuterAttr};

pub(crate) fn generate_decode(input: DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    fn generate_fields<'f>(
        context @ Context {
            outer: OuterAttr { syrup, .. },
            decode_error_ty,
            result_ty,
            ..
        }: &Context,
        _tree_lt: &Lifetime,
        _idata_ty: &Ident,
        fields: impl IntoIterator<Item = &'f syn::Field>,
    ) -> syn::Result<Punctuated<FieldValue, Comma>> {
        let mut res = Punctuated::new();
        for (i, field) in fields.into_iter().enumerate() {
            let (member, expected): (Member, Expr) = match field.ident.as_ref() {
                Some(id) => {
                    let id_str = LitStr::new(&id.to_string(), id.span());
                    (
                        syn::Member::Named(id.clone()),
                        parse_quote_spanned! {field.span()=>#syrup::de::SyrupKind::Unknown(#id_str)},
                    )
                }
                None => {
                    let exp_str = LitStr::new(&format!("{i}th field"), field.span());
                    (
                        syn::Member::Unnamed(Index {
                            index: i as u32,
                            span: field.span(),
                        }),
                        parse_quote_spanned! {field.span()=>#syrup::de::SyrupKind::Unknown(#exp_str)},
                    )
                }
            };
            let attr = FieldAttr::new(context, field)?;

            let mut expr = parse_quote_spanned! {field.span()=>
                match elements.get(#i) {
                    Some(el) => el,
                    None => return #result_ty::Err(#decode_error_ty::Missing(#expected))
                }
            };

            expr = if let Some(decode) = attr.decode {
                let dec = decode(&expr);
                parse_quote_spanned! {dec.span()=>#dec?}
            } else {
                parse_quote_spanned! {field.span()=>#expr.decode()?}
            };

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

    let output_lt = Lifetime::new("'__output", Span::call_site());
    let output_lt_param = LifetimeParam {
        attrs: Vec::with_capacity(0),
        lifetime: output_lt.clone(),
        colon_token: Default::default(),
        bounds: Default::default(),
    };

    let idata_ty = Ident::new("__IData", Span::call_site());
    let idata_param = TypeParam {
        attrs: Default::default(),
        ident: idata_ty.clone(),
        colon_token: None,
        bounds: Default::default(),
        eq_token: None,
        default: None,
    };

    let id = input.ident;

    let context = Context::new(OuterAttr::new(&id, &input.attrs)?);

    let Context {
        outer:
            OuterAttr {
                syrup,
                label,
                decode_where,
                ..
            },
        token_tree_ty,
        decode_error_ty,
        result_ty,
        de_result_ty,
        literal_ty,
    } = &context;

    let mut impl_params = input.generics.params.clone();
    impl_params.push(GenericParam::Type(idata_param.clone()));
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut where_clause = where_clause.cloned().unwrap_or(WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });

    where_clause.predicates.push(parse_quote! {
        #idata_ty: #syrup::borrow_or_share::BorrowOrShare<#input_lt, #output_lt, [u8]>
    });

    if decode_where.is_empty() {
        match &input.data {
            syn::Data::Enum(_) => todo!("derive(Decode) for enums"),
            syn::Data::Union(_) => todo!("derive(Decode) for unions"),
            syn::Data::Struct(data) => match &data.fields {
                syn::Fields::Unit => {}
                syn::Fields::Named(FieldsNamed { named: fields, .. })
                | syn::Fields::Unnamed(FieldsUnnamed {
                    unnamed: fields, ..
                }) => {
                    for field in fields {
                        let ty = &field.ty;
                        where_clause
                            .predicates
                            .push(parse_quote_spanned! {ty.span()=>
                                #ty: #syrup::Decode<#input_lt, #idata_ty>
                            });
                    }
                }
            },
        }
    }

    for pred in decode_where {
        where_clause.predicates.push(pred.clone());
    }

    let decode_sig: Signature = parse_quote! {fn decode(input: &#input_lt #syrup::de::TokenTree<#idata_ty>) -> #de_result_ty };

    let decode_fn: ImplItemFn = match input.data {
        syn::Data::Union(u) => {
            return Err(syn::Error::new_spanned(
                u.union_token,
                "not yet implemented: union decode derivation",
            ))
        }
        syn::Data::Enum(data) => {
            return Err(syn::Error::new_spanned(
                data.enum_token,
                "not yet implemented: enum decode derivation",
            ))
        }
        syn::Data::Struct(data) => {
            let label_expr: Expr = parse_quote_spanned! {label.span()=> {
                #[expect(clippy::string_lit_as_bytes)]
                match label {
                    #token_tree_ty::Literal(#literal_ty::Symbol(label_sym)) => if label_sym.borrow_or_share() != #label.as_bytes() {
                        return #result_ty::Err(#decode_error_ty::unexpected(
                            #syrup::de::SyrupKind::Symbol(::std::option::Option::Some(#label)),
                            label
                        ))
                    },
                    _ => return #result_ty::Err(#decode_error_ty::unexpected(
                            #syrup::de::SyrupKind::Symbol(::std::option::Option::Some(#label)),
                            label,
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
                        fields: generate_fields(&context, &input_lt, &idata_ty, fields)?,
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
                        #token_tree_ty::Record(record) => {
                            let (label, elements) = (&record.label, &record.elements);
                            #label_expr;
                            #res_expr
                        },
                        _ => #result_ty::Err(#decode_error_ty::unexpected(
                            #syrup::de::SyrupKind::Record { label: ::std::option::Option::Some(#label) },
                            input,
                        ))
                    }
                }
            }
        }
    };

    Ok(quote! {
        #[automatically_derived]
        impl<#input_lt_param, #output_lt_param, #impl_params> #syrup::Decode<#input_lt, #idata_ty> for #id #ty_generics #where_clause {
            #decode_fn
        }
    }.into())
}
