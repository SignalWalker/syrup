use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, GenericParam, Ident, LitInt, Type, TypeParam, TypeParamBound,
};

fn gen_tuple_idents(max_arity: usize) -> Vec<Ident> {
    (0..max_arity)
        .into_iter()
        .map(|id_num| Ident::new(&format!("__{id_num}"), Span::call_site()))
        .collect()
}

fn gen_tuple_params(
    param_bound: &TypeParamBound,
    idents: &[Ident],
) -> (Vec<GenericParam>, Vec<Type>) {
    let mut impl_generics: Vec<GenericParam> = Vec::with_capacity(idents.len());
    let mut ty_generics: Vec<Type> = Vec::with_capacity(idents.len());
    for ident in idents {
        impl_generics.push(GenericParam::Type({
            let mut res = TypeParam::from(ident.clone());
            res.bounds.push(param_bound.clone());
            res
        }));
        ty_generics.push(parse_quote!(#ident));
    }
    (impl_generics, ty_generics)
}

#[proc_macro]
pub fn impl_deserialize_for_tuple(max_arity: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let max_arity: usize = parse_macro_input!(max_arity as LitInt)
        .base10_parse()
        .unwrap();
    let idents = gen_tuple_idents(max_arity);
    let param_bound: TypeParamBound = parse_quote!(Deserialize<'input>);
    let mut res = TokenStream::new();
    for idents in (1..=max_arity).into_iter().map(|arity| &idents[0..arity]) {
        let (impl_generics, ty_generics) = gen_tuple_params(&param_bound, idents);
        let arity_str = idents.len().to_string();
        quote! {
            #[automatically_derived]
            impl <'input, #(#impl_generics),* > Deserialize<'input> for ( #(#ty_generics,)* ) {
                fn deserialize<Des: Deserializer<'input>>(de: Des) -> Result<Self, Des::Error> {
                    struct __Visitor<#(#ty_generics),*> {
                        _p: PhantomData<(#(#ty_generics,)*)>
                    }
                    impl<'input, #(#impl_generics),*> Visitor<'input> for __Visitor<#(#ty_generics),*> {
                        type Value = ( #(#ty_generics,)* );

                        fn expecting(&self, f: &mut ::std::fmt::Formatter<'_>) -> std::fmt::Result {
                            f.write_str(::std::concat!(#arity_str, "-tuple"))
                        }

                        fn visit_sequence<Seq: SeqAccess<'input>>(self, mut seq: Seq) -> Result<Self::Value, Seq::Error> {
                            Ok(( #(seq.next_value::<#ty_generics>()?.ok_or_else(|| todo!())?,)* ))
                        }
                    }
                    de.deserialize_sequence(__Visitor { _p: PhantomData })
                }
            }
        }
        .to_tokens(&mut res);
    }
    res.into()
}

#[proc_macro]
pub fn impl_serialize_for_tuple(max_arity: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let max_arity: usize = parse_macro_input!(max_arity as LitInt)
        .base10_parse()
        .unwrap();
    let idents = gen_tuple_idents(max_arity);
    let param_bound: TypeParamBound = parse_quote!(Serialize);
    let mut res = TokenStream::new();
    for (arity, idents) in (1..=max_arity)
        .into_iter()
        .map(|arity| (arity, &idents[0..arity]))
    {
        let (impl_generics, ty_generics) = gen_tuple_params(&param_bound, idents);
        let serializes = (0..arity)
            .into_iter()
            .map(|index| {
                let index = syn::Index::from(index);
                quote! { &self.#index }
            })
            .collect::<Vec<_>>();
        quote! {
            #[automatically_derived]
            impl<#(#impl_generics),*> Serialize for ( #(#ty_generics,)* ) {
                fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
                    let mut seq = s.serialize_sequence(Some(#arity))?;
                    #( seq.serialize_element(#serializes)?; )*
                    seq.end()
                }
            }
        }
        .to_tokens(&mut res);
    }
    res.into()
}
