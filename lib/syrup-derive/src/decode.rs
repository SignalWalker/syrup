use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_quote, parse_quote_spanned, punctuated::Punctuated, spanned::Spanned, token::Comma,
    DeriveInput, Expr, ExprStruct, FieldValue, FieldsNamed, FieldsUnnamed, ImplItemFn, Index,
    Lifetime, LifetimeParam, LitStr, Signature, Token,
};

use crate::{Context, FieldAttr, OuterAttr};

pub(crate) fn generate_decode(input: DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    fn generate_fields<'f>(
        Context {
            cow_ty,
            decode_error_ty,
            result_ty,
            ..
        }: &Context,
        fields: impl IntoIterator<Item = &'f syn::Field>,
    ) -> syn::Result<Punctuated<FieldValue, Comma>> {
        let mut res = Punctuated::new();
        for (i, field) in fields.into_iter().enumerate() {
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
                match elements.get(#i) {
                    Some(el) => el,
                    None => return #result_ty::Err(#decode_error_ty::missing(#cow_ty::Borrowed(#expected)))
                }
            };

            expr = match attr.decode {
                Some(decode) => parse_quote_spanned! {decode.span()=>
                    #decode(#expr)?
                },
                None => parse_quote_spanned! {field.span()=>
                    #expr.decode()?
                },
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

    let id = input.ident;

    let context = Context::new(OuterAttr::new(&id, &input.attrs)?);

    let Context {
        outer: OuterAttr { syrup, label },
        cow_ty,
        token_tree_ty,
        decode_error_ty,
        result_ty,
        de_result_ty,
        literal_ty,
        de_error_lt,
    } = &context;

    let error_lt_param = LifetimeParam {
        attrs: Vec::with_capacity(0),
        lifetime: de_error_lt.clone(),
        colon_token: Default::default(),
        bounds: Default::default(),
    };

    let impl_params = &input.generics.params;
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

    let decode_sig: Signature = parse_quote! {fn decode<#error_lt_param>(input: &#input_lt #syrup::de::TokenTree) -> #de_result_ty };

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
                #[allow(clippy::string_lit_as_bytes)]
                match label {
                    #token_tree_ty::Literal(#literal_ty::Symbol(label_sym)) => if label_sym != #label.as_bytes() {
                        return #result_ty::Err(#decode_error_ty::unexpected(
                            #cow_ty::Borrowed(#label),
                            label.clone(),
                        ))
                    },
                    _ => return #result_ty::Err(#decode_error_ty::unexpected(
                            #cow_ty::Borrowed(#label),
                            label.clone(),
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
                        #token_tree_ty::Record(record) => {
                            let (label, elements) = (&record.label, &record.elements);
                            #label_expr;
                            #res_expr
                        },
                        _ => #result_ty::Err(#decode_error_ty::unexpected(
                            #cow_ty::Borrowed(#label),
                            input.clone(),
                        ))
                    }
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
