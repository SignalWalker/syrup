use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::DeriveInput;

use crate::common::Container;

pub(crate) fn generate_serialize(input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let container = Container::from_derive_input(input)?;
    let syrup = &container.syrup_crate;
    let ident = container.ident;

    let (impl_generics, ty_generics, where_clause) = container.ser_generics.split_for_impl();

    let serializer_ty = Ident::new("__Ser", Span::call_site());
    let serializer = Ident::new("__ser", Span::call_site());

    let serialize_expr = container.generate_serialize_expr(&serializer)?;

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #syrup::ser::Serialize for #ident #ty_generics #where_clause {
            fn serialize<#serializer_ty: #syrup::ser::Serializer>(&self, #serializer: #serializer_ty) -> ::std::result::Result<#serializer_ty::Ok, #serializer_ty::Error> {
                #serialize_expr
            }
        }
    })
}
