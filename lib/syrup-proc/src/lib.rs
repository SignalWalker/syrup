use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, GenericParam, Ident, Lifetime, LifetimeParam, LitInt, LitStr,
    Type, TypeParam, TypeParamBound,
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

    let input_lt: Lifetime = Lifetime::new("'__input", Span::call_site());
    let input_lt_param: LifetimeParam = LifetimeParam::new(input_lt.clone());

    let output_lt: Lifetime = Lifetime::new("'__output", Span::call_site());
    let output_lt_param: LifetimeParam = LifetimeParam::new(output_lt.clone());

    let idata_id: Ident = Ident::new("__IData", Span::call_site());
    let idata_param: GenericParam = GenericParam::Type(TypeParam {
        attrs: Default::default(),
        ident: idata_id.clone(),
        colon_token: None,
        bounds: Default::default(),
        eq_token: None,
        default: None,
    });

    let idents = gen_tuple_idents(max_arity);
    let param_bound: TypeParamBound = parse_quote!(syrup::de::Decode<#input_lt, #idata_id>);
    let mut res = TokenStream::new();

    let decodes = idents
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let exp_str = LitStr::new(&format!("{i}th element"), Span::call_site());
            quote! {
                match elements.get(#i) {
                    Some(el) => el.decode()?,
                    None => return Err(syrup::de::DecodeError::Missing(syrup::de::SyrupKind::Unknown(#exp_str)))
                }
            }
        })
        .collect::<Vec<_>>();

    for (idents, decodes) in (1..=max_arity).map(|arity| (&idents[..arity], &decodes[..arity])) {
        let (impl_generics, ty_generics) = gen_tuple_params(&param_bound, idents);

        let expected_len = idents.len();

        quote! {
            #[automatically_derived]
            impl <#input_lt_param, #output_lt_param, #idata_param, #(#impl_generics),*> syrup::de::Decode<#input_lt, #idata_id> for ( #(#ty_generics,)* )
            where
                #idata_id: borrow_or_share::BorrowOrShare<#input_lt, #output_lt, [u8]>,
            {
                fn decode(input: &#input_lt syrup::de::TokenTree<#idata_id>) -> ::std::result::Result<Self, syrup::de::DecodeError> {
                    match input {
                        syrup::de::TokenTree::List(syrup::de::List {
                            elements,
                        }) => {
                            Ok(( #(#decodes,)* ))
                        },
                        tree => Err(syrup::de::DecodeError::unexpected(syrup::de::SyrupKind::List { length: Some(#expected_len) }, tree))
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

    let input_lt: Lifetime = Lifetime::new("'__input", Span::call_site());
    let input_lt_param: LifetimeParam = LifetimeParam::new(input_lt.clone());

    let odata_id: Ident = Ident::new("__OData", Span::call_site());
    let odata_param: GenericParam = GenericParam::Type(TypeParam {
        attrs: Default::default(),
        ident: odata_id.clone(),
        colon_token: None,
        bounds: Default::default(),
        eq_token: None,
        default: None,
    });

    let param_bound: TypeParamBound = parse_quote!(syrup::ser::Encode<#input_lt, #odata_id>);
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
            impl<#input_lt_param, #odata_param, #(#impl_generics),*> syrup::ser::Encode<#input_lt, #odata_id> for ( #(#ty_generics,)* ) {
                fn encode(&#input_lt self) -> syrup::de::TokenTree<#odata_id> {
                    use syrup::ser::Encode;
                    syrup::de::TokenTree::List(syrup::de::List::new(vec![#(#encodes),*]))
                }
            }
        }
        .to_tokens(&mut res);
    }
    res.into()
}
