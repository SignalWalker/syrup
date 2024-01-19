use super::{Container, Field};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, Expr, Generics, Ident, Lifetime, Path};

pub enum Inner<'input> {
    Variants(()),
    Fields(Vec<Field<'input>>),
}

impl<'input> Inner<'input> {
    pub fn from_fields(
        syrup: &Path,
        fields: impl IntoIterator<Item = &'input syn::Field>,
        c_ser_generics: &Generics,
        c_des_generics: &Generics,
        c_des_lifetime: &Lifetime,
    ) -> Result<Self, syn::Error> {
        let fields = fields.into_iter();
        let mut res = Vec::new();
        for field in fields {
            res.push(Field::from_field(
                syrup,
                field,
                c_ser_generics,
                c_des_generics,
                c_des_lifetime,
            )?);
        }
        Ok(Self::Fields(res))
    }

    pub fn generate_deserialize_expr(
        &self,
        container: &Container<'_>,
        deserializer: &Ident,
        visitor: &Ident,
    ) -> Result<(TokenStream, Expr), syn::Error> {
        match &self {
            Inner::Variants(_) => errtodo!("deserialize enums"),
            Inner::Fields(fields) => {
                let syrup = &container.syrup_crate;
                let rec = Ident::new("__rec", Span::call_site());
                let record_label = &container.name;
                let lifetime = &container.des_lifetime;

                let next_field = parse_quote! { #rec.next_field };

                let mut field_ids = Vec::with_capacity(fields.len());
                let mut field_exprs = Vec::with_capacity(fields.len());
                for field in fields {
                    field_ids.push(field.ident.unwrap());
                    field_exprs.push(field.generate_deserialize(container, &next_field)?);
                }
                Ok((
                    quote! {
                        fn visit_record<Rec: #syrup::de::RecordAccess<#lifetime>>(self, #rec: Rec) -> ::std::result::Result<Self::Value, Rec::Error> {
                            use #syrup::de::RecordFieldAccess;

                            let (mut #rec, label) = #rec.label::<#syrup::Symbol<&#lifetime str>>()?;
                            if label.0 != #record_label {
                                todo!("handle mismatched record labels in deserialize derive (expected {:?}, got {:?})", #record_label, label.0)
                            }
                            Ok(Self::Value {
                                #(#field_ids: #field_exprs),*
                            })
                        }
                    },
                    parse_quote! {
                        #deserializer.deserialize_record(#visitor)
                    },
                ))
            }
        }
    }

    pub fn generate_serialize_expr(
        &self,
        container: &Container<'_>,
        serializer: &Ident,
    ) -> Result<Expr, syn::Error> {
        match &self {
            Inner::Variants(_) => errtodo!("serialize enums"),
            Inner::Fields(fields) => {
                let syrup = &container.syrup_crate;

                let record_label = &container.name;
                let field_len = fields.len();
                let rec = Ident::new("__rec", Span::call_site());

                let driver: Expr = parse_quote! {
                    #rec.serialize_field
                };

                let mut field_exprs: Vec<Expr> = Vec::with_capacity(fields.len());
                for (index, field) in fields.iter().enumerate() {
                    field_exprs.push(field.generate_serialize_expr(
                        container,
                        &driver,
                        u32::try_from(index).unwrap(),
                    )?);
                }

                Ok(parse_quote! {{
                    use #syrup::ser::SerializeRecord;
                    let mut #rec = #serializer.serialize_record(#record_label, Some(#field_len))?;
                    #(#field_exprs;)*
                    #rec.end()
                }})
            }
        }
    }
}
