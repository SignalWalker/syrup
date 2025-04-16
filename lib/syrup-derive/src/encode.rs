use quote::quote;
use syn::{parse_quote_spanned, spanned::Spanned, DeriveInput, Expr, Field, Index};

use crate::{Context, FieldAttr, OuterAttr};

pub(crate) fn generate_encode(input: DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    fn generate_fields<'f, Fields>(
        Context { .. }: &Context,
        fields: Fields,
    ) -> impl Iterator<Item = syn::Result<Expr>> + 'f + use<'f, Fields>
    where
        Fields: IntoIterator<Item = &'f Field>,
        <Fields as IntoIterator>::IntoIter: 'f,
    {
        fields.into_iter().enumerate().map(|(i, field)| {
            let Field { ident, attrs, .. } = field;
            let attr = FieldAttr::new(attrs)?;
            let member = match ident.as_ref() {
                Some(id) => syn::Member::Named(id.clone()),
                None => syn::Member::Unnamed(Index {
                    index: i as u32,
                    span: field.span(),
                }),
            };
            match attr.encode {
                Some(encode) => Ok(parse_quote_spanned! {encode.span()=>
                    #encode(&self.#member)
                }),
                None => Ok(parse_quote_spanned! {field.span()=>
                    self.#member.encode()
                }),
            }
        })
    }

    let context = Context::new(OuterAttr::new(&input.ident, &input.attrs)?);

    let Context {
        outer: OuterAttr { syrup, label },
        ..
    } = &context;

    // let output_lt = Lifetime::new("'__output", Span::call_site());
    // let output_lt_param = LifetimeParam {
    //     attrs: Vec::with_capacity(0),
    //     lifetime: output_lt.clone(),
    //     colon_token: Default::default(),
    //     bounds: input
    //         .generics
    //         .lifetimes()
    //         .map(|param| param.lifetime.clone())
    //         .collect(),
    // };
    let id = &input.ident;

    let impl_params = &input.generics.params;
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

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
                    for field in generate_fields(&context, &fields.named) {
                        res.push(field?);
                    }
                    res
                }
                syn::Fields::Unnamed(fields) => {
                    let mut res = Vec::with_capacity(fields.unnamed.len());
                    for field in generate_fields(&context, &fields.unnamed) {
                        res.push(field?);
                    }
                    res
                }
                syn::Fields::Unit => Vec::<Expr>::with_capacity(0),
            };
            let res = quote! {
                #[automatically_derived]
                impl<#impl_params> #syrup::Encode for #id #ty_generics #where_clause {
                    fn encode(&self) -> #syrup::TokenTree {
                        #syrup::TokenTree::Record(::std::boxed::Box::new(#syrup::de::Record {
                            label: #syrup::TokenTree::Literal(#syrup::de::Literal::Symbol(#label.as_bytes().to_vec())),
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
