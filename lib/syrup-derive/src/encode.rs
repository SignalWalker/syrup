use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_quote, parse_quote_spanned, spanned::Spanned, DeriveInput, Expr, Field, GenericParam,
    Ident, Index, Lifetime, LifetimeParam, PredicateType, Token, Type, TypeParam, TypeReference,
    WhereClause,
};

use crate::{Context, FieldAttr, OuterAttr};

pub(crate) fn generate_encode(input: DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    fn generate_fields<'f, Fields>(
        input_lt: &'f Lifetime,
        context @ Context { .. }: &'f Context,
        fields: Fields,
    ) -> impl Iterator<Item = syn::Result<Expr>> + 'f + use<'f, Fields>
    where
        Fields: IntoIterator<Item = &'f Field>,
        <Fields as IntoIterator>::IntoIter: 'f,
    {
        fields.into_iter().enumerate().map(|(i, field)| {
            let Field { ident, .. } = field;
            let attr = FieldAttr::new(context, field)?;
            let member = match ident.as_ref() {
                Some(id) => syn::Member::Named(id.clone()),
                None => syn::Member::Unnamed(Index {
                    index: i as u32,
                    span: field.span(),
                }),
            };
            let (access, accessed_ty): (Expr, Type) = {
                (
                    parse_quote_spanned! {field.span()=>&self.#member},
                    Type::Reference(TypeReference {
                        and_token: Token![&](field.span()),
                        lifetime: Some(input_lt.clone()),
                        mutability: None,
                        elem: Box::new(field.ty.clone()),
                    }),
                )
            };
            if let Some(encode) = attr.encode {
                Ok(encode(&access, &accessed_ty))
            } else {
                Ok(parse_quote_spanned! {field.span()=>self.#member.encode()})
            }
        })
    }

    let context = Context::new(OuterAttr::new(&input.ident, &input.attrs)?);

    let Context {
        outer:
            OuterAttr {
                syrup,
                label,
                encode_where,
                ..
            },
        ..
    } = &context;

    let label_lt = Lifetime::new("'__label", Span::call_site());
    let label_lt_param = LifetimeParam {
        attrs: Vec::with_capacity(0),
        lifetime: label_lt.clone(),
        colon_token: Default::default(),
        bounds: Default::default(),
    };

    let input_lt = Lifetime::new("'__input", Span::call_site());
    let input_lt_param = LifetimeParam {
        attrs: Vec::with_capacity(0),
        lifetime: input_lt.clone(),
        colon_token: Default::default(),
        bounds: Default::default(),
    };

    let output_lt = Lifetime::new("'__output", Span::call_site());
    let output_lt_param = LifetimeParam {
        attrs: Vec::with_capacity(0),
        lifetime: output_lt.clone(),
        colon_token: Default::default(),
        bounds: Default::default(),
    };

    let id = &input.ident;

    let odata_ty: Ident = Ident::new("__OData", Span::call_site());
    let odata_param: TypeParam = TypeParam {
        attrs: Default::default(),
        ident: odata_ty.clone(),
        colon_token: None,
        bounds: Default::default(),
        eq_token: None,
        default: None,
    };

    let mut impl_params = input.generics.params.clone();
    impl_params.push(GenericParam::Type(odata_param));
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut where_clause = where_clause.cloned().unwrap_or(WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });

    // default where predicates (for each field in self, `Field: Encode`)
    if encode_where.is_empty() {
        // ensure label can be converted into odata
        where_clause
            .predicates
            .push(syn::WherePredicate::Type(PredicateType {
                lifetimes: None,
                bounded_ty: parse_quote! { &#label_lt [u8] },
                colon_token: Token![:](Span::call_site()),
                bounds: parse_quote! { ::std::convert::Into<#odata_ty> },
            }));

        match &input.data {
            syn::Data::Enum(_) => todo!("derive(Encode) for enums"),
            syn::Data::Union(_) => todo!("derive(Encode) for unions"),
            syn::Data::Struct(data) => match &data.fields {
                syn::Fields::Unit => Default::default(),
                syn::Fields::Named(fields) => {
                    for field in &fields.named {
                        let ty = &field.ty;
                        where_clause
                            .predicates
                            .push(parse_quote_spanned! {ty.span()=>
                                #ty: #syrup::Encode<#input_lt, #odata_ty>
                            });
                    }
                }
                syn::Fields::Unnamed(fields) => {
                    for field in &fields.unnamed {
                        let ty = &field.ty;
                        where_clause
                            .predicates
                            .push(parse_quote_spanned! {ty.span()=>
                                #ty: #syrup::Encode<#input_lt, #odata_ty>
                            });
                    }
                }
            },
        }
    }

    for pred in encode_where {
        where_clause.predicates.push(pred.clone());
    }

    match input.data {
        syn::Data::Union(u) => Err(syn::Error::new_spanned(
            u.union_token,
            "not yet implemented: union encode derivation",
        )),
        syn::Data::Enum(data) => Err(syn::Error::new_spanned(
            data.enum_token,
            "not yet implemented: enum encode derivation",
        )),
        syn::Data::Struct(data) => {
            let fields: Vec<Expr> = match &data.fields {
                syn::Fields::Named(fields) => {
                    let mut res = Vec::with_capacity(fields.named.len());
                    for field in generate_fields(&input_lt, &context, &fields.named) {
                        res.push(field?);
                    }
                    res
                }
                syn::Fields::Unnamed(fields) => {
                    let mut res = Vec::with_capacity(fields.unnamed.len());
                    for field in generate_fields(&input_lt, &context, &fields.unnamed) {
                        res.push(field?);
                    }
                    res
                }
                syn::Fields::Unit => Vec::<Expr>::with_capacity(0),
            };
            let res = quote! {
                #[automatically_derived]
                impl<#label_lt_param, #input_lt_param, #output_lt_param, #impl_params> #syrup::Encode<#input_lt, #odata_ty> for #id #ty_generics #where_clause {
                    fn encode(&#input_lt self) -> #syrup::TokenTree<#odata_ty> {
                        #syrup::TokenTree::Record(::std::boxed::Box::new(#syrup::de::Record {
                            label: #syrup::TokenTree::Literal(#syrup::de::Literal::Symbol(#label.as_bytes().into())),
                            elements: ::std::vec![
                                #(#fields),*
                            ]
                        }))
                    }
                }
            };
            Ok(res.into())
        }
    }
}
