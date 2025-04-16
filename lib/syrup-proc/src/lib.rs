use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, GenericParam, Ident, Lifetime, LifetimeParam, LitInt, Type,
    TypeParam, TypeParamBound,
};

fn gen_tuple_idents(max_arity: usize) -> Vec<Ident> {
    (0..max_arity)
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
pub fn impl_decode_for_tuple(max_arity: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let max_arity: usize = parse_macro_input!(max_arity as LitInt)
        .base10_parse()
        .unwrap();

    let lifetime: Lifetime = Lifetime::new("'__input", Span::call_site());
    let lifetime_param: LifetimeParam = LifetimeParam::new(lifetime.clone());

    let idents = gen_tuple_idents(max_arity);
    let param_bound: TypeParamBound = parse_quote!(syrup::de::Decode<#lifetime>);
    let mut res = TokenStream::new();

    let decodes = idents
        .iter()
        .enumerate()
        .map(|(i, _)| {
            // TODO :: include expected type
            let require = format!("{i}th tuple element");
            quote! {
                match elements.get(#i) {
                    Some(el) => el.decode()?,
                    None => return Err(syrup::de::DecodeError::missing(::std::borrow::Cow::Borrowed(#require)))
                }
            }
        })
        .collect::<Vec<_>>();

    for (idents, decodes) in (1..=max_arity).map(|arity| (&idents[..arity], &decodes[..arity])) {
        let (impl_generics, ty_generics) = gen_tuple_params(&param_bound, idents);

        let expected = format!("List with {} elements", idents.len());

        quote! {
            #[automatically_derived]
            impl <#lifetime_param, #(#impl_generics),*> syrup::de::Decode<#lifetime> for ( #(#ty_generics,)* ) {
                fn decode<'__error>(input: &#lifetime syrup::de::TokenTree) -> ::std::result::Result<Self, syrup::de::DecodeError<'__error>> {
                    match input {
                        syrup::de::TokenTree::List(syrup::de::List {
                            elements,
                        }) => {
                            Ok(( #(#decodes,)* ))
                        },
                        tree => Err(syrup::de::DecodeError::unexpected(::std::borrow::Cow::Borrowed(#expected), tree.clone()))
                    }
                }
            }
        }
        .to_tokens(&mut res);
    }
    res.into()
}

#[proc_macro]
pub fn impl_encode_for_tuple(max_arity: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let max_arity: usize = parse_macro_input!(max_arity as LitInt)
        .base10_parse()
        .unwrap();
    let idents = gen_tuple_idents(max_arity);
    let lifetime: Lifetime = Lifetime::new("'output", Span::call_site());
    let lifetime_param: LifetimeParam = LifetimeParam::new(lifetime.clone());
    let param_bound: TypeParamBound = parse_quote!(syrup::ser::Encode);
    let mut res = TokenStream::new();
    let encodes = idents
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let index = syn::Index::from(i);
            quote! { self.#index.encode() }
        })
        .collect::<Vec<_>>();
    for (idents, encodes) in (1..=max_arity).map(|arity| (&idents[0..arity], &encodes[0..arity])) {
        let (impl_generics, ty_generics) = gen_tuple_params(&param_bound, idents);
        quote! {
            #[automatically_derived]
            impl<#lifetime_param, #(#impl_generics),*> syrup::ser::Encode for ( #(#ty_generics,)* ) {
                fn encode(&self) -> syrup::de::TokenTree {
                    use syrup::ser::Encode;
                    syrup::de::TokenTree::List(syrup::de::List::new(vec![#(#encodes),*]))
                }
            }
        }
        .to_tokens(&mut res);
    }
    res.into()
}
