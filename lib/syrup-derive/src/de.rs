use crate::common::Container;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{parse_quote, DeriveInput, GenericParam, LifetimeParam, TypeTuple};

pub(crate) fn generate_deserialize(input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let container = Container::from_derive_input(input)?;
    let syrup = &container.syrup_crate;
    let ident = container.ident;
    let expecting = container.expecting()?;

    let de_lifetime = &container.des_lifetime;
    let (_, ty_generics, where_clause) = container.des_generics.split_for_impl();
    let impl_generics = {
        let mut gen = container.des_generics.clone();
        gen.params.insert(
            0,
            GenericParam::Lifetime(LifetimeParam::new(de_lifetime.clone())),
        );
        let (res, _, _) = gen.split_for_impl();
        res.to_token_stream()
    };

    let generic_idents = input.generics.params.iter().filter_map(|p| match p {
        syn::GenericParam::Type(t) => Some(&t.ident),
        _ => None,
    });

    let phantom_tuple: TypeTuple = parse_quote! { (#(#generic_idents,)*) };

    let deserializer_ty = Ident::new("__De", Span::call_site());
    let deserializer = Ident::new("__de", Span::call_site());
    let visitor_ty = Ident::new("__Visitor", Span::call_site());
    let visitor = Ident::new("__visitor", Span::call_site());

    let (visit_impl, deserialize_expr) =
        container.generate_deserialize_expr(&deserializer, &visitor)?;

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #syrup::de::Deserialize<#de_lifetime> for #ident #ty_generics #where_clause {
            fn deserialize<#deserializer_ty: #syrup::de::Deserializer<#de_lifetime>>(#deserializer: #deserializer_ty) -> ::std::result::Result<Self, #deserializer_ty::Error> {
                struct #visitor_ty #ty_generics {
                    _p: ::std::marker::PhantomData<#phantom_tuple>
                }
                impl #impl_generics #syrup::de::Visitor<#de_lifetime> for #visitor_ty #ty_generics #where_clause {
                    type Value = #ident #ty_generics;

                    fn expecting(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                        #expecting
                    }

                    #visit_impl
                }

                let #visitor = #visitor_ty { _p: ::std::marker::PhantomData };
                #deserialize_expr
            }
        }
    })
}
