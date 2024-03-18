use super::{Inner, With};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse_quote, parse_quote_spanned, punctuated::Punctuated, spanned::Spanned, DeriveInput, Expr,
    Generics, Ident, Lifetime, LitStr, Path, Token, WherePredicate,
};

pub(crate) struct Container<'input> {
    pub(crate) ident: &'input Ident,

    /// path to syrup crate
    pub(crate) syrup_crate: Path,
    /// custom `expecting` message
    pub(crate) expecting: Option<Expr>,

    /// name with which to serialize/deserialize records of this type
    pub(crate) name: String,

    pub(crate) from: Option<With>,
    pub(crate) into: Option<With>,

    pub(crate) ser_generics: Generics,
    pub(crate) des_generics: Generics,
    pub(crate) des_lifetime: Lifetime,

    pub(crate) inner: Option<Inner<'input>>,
}

impl<'input> Container<'input> {
    pub(crate) fn from_derive_input(input: &'input DeriveInput) -> Result<Self, syn::Error> {
        let mut syrup_crate: Path = parse_quote! { ::syrup };
        let mut expecting = None;
        let mut name = input.ident.to_string();
        let mut from = None;
        let mut into = None;

        let mut des_bounds = None;
        let mut ser_bounds = None;

        for attr in input.attrs.iter().filter(|&a| a.path().is_ident("syrup")) {
            attr.parse_nested_meta(|meta| {
                let attr_id = meta.path.require_ident()?.to_string();
                match attr_id.as_str() {
                    "crate" => syrup_crate = meta.value()?.parse()?,
                    "expecting" => expecting = Some(meta.value()?.parse()?),
                    "name" => name = meta.value()?.parse::<LitStr>()?.value(),
                    // conversion
                    "from" => from = Some(With::infallible(meta.value()?.parse()?)),
                    "into" => into = Some(With::infallible(meta.value()?.parse()?)),
                    "try_from" => from = Some(With::fallible(meta.value()?.parse()?)),
                    "try_into" => into = Some(With::fallible(meta.value()?.parse()?)),
                    "as" => {
                        from = Some(With::infallible(meta.value()?.parse()?));
                        into = Some(With::infallible(meta.value()?.parse()?));
                    }
                    "try_as" => {
                        from = Some(With::fallible(meta.value()?.parse()?));
                        into = Some(With::fallible(meta.value()?.parse()?));
                    }
                    "transparent" => {
                        from = Some(With::Verbatim(parse_quote! { self.0 }));
                        into = Some(With::Verbatim(parse_quote! { self.0 }));
                    }
                    // bounds
                    "deserialize_bound" => {
                        des_bounds =
                            Some(Punctuated::<WherePredicate, Token![;]>::parse_terminated(
                                meta.value()?,
                            )?);
                    }
                    "serialize_bound" => {
                        ser_bounds =
                            Some(Punctuated::<WherePredicate, Token![;]>::parse_terminated(
                                meta.value()?,
                            )?);
                    }
                    _ => return Err(meta.error(format!("unrecognized syrup attribute: {attr_id}"))),
                }
                Ok(())
            })?;
        }

        let (des_lifetime, des_generics, ser_generics) =
            Self::generics(&syrup_crate, &input.generics, des_bounds, ser_bounds)?;

        Ok(Self {
            ident: &input.ident,
            expecting,
            name,
            from,
            into,
            inner: match &input.data {
                syn::Data::Struct(s) => match &s.fields {
                    syn::Fields::Named(named) => Some(Inner::from_fields(
                        &syrup_crate,
                        &named.named,
                        &ser_generics,
                        &des_generics,
                        &des_lifetime,
                    )?),
                    syn::Fields::Unnamed(unnamed) => Some(Inner::from_fields(
                        &syrup_crate,
                        &unnamed.unnamed,
                        &ser_generics,
                        &des_generics,
                        &des_lifetime,
                    )?),
                    syn::Fields::Unit => None,
                },
                syn::Data::Enum(_) => errtodo!("enums"),
                syn::Data::Union(_) => errtodo!("unions"),
            },
            syrup_crate,
            ser_generics,
            des_generics,
            des_lifetime,
        })
    }

    pub(crate) fn expecting(&self) -> Result<Expr, syn::Error> {
        let name = &self.name;
        match &self.expecting {
            Some(msg) => Ok(parse_quote_spanned! { msg.span() => f.write_str(#msg) }),
            None => Ok(parse_quote! { f.write_str(::std::concat!(#name, " object")) }),
        }
    }

    fn generics(
        syrup_crate: &Path,
        generics: &'input Generics,
        des_bounds: Option<impl IntoIterator<Item = WherePredicate>>,
        ser_bounds: Option<impl IntoIterator<Item = WherePredicate>>,
    ) -> Result<(Lifetime, Generics, Generics), syn::Error> {
        let mut ser_generics = generics.clone();
        match ser_bounds {
            Some(b) => ser_generics.make_where_clause().predicates.extend(b),
            None => {
                for param in ser_generics.type_params_mut() {
                    param
                        .bounds
                        .push(parse_quote! { #syrup_crate::ser::Serialize });
                }
            }
        }

        let des_lifetime = Lifetime::new("'__de", Span::call_site());
        let mut des_generics = generics.clone();
        // des_generics.params.insert(
        //     0,
        //     GenericParam::Lifetime(LifetimeParam::new(des_lifetime.clone())),
        // );
        match des_bounds {
            Some(b) => des_generics.make_where_clause().predicates.extend(b),
            None => {
                for param in des_generics.type_params_mut() {
                    param
                        .bounds
                        .push(parse_quote! { #syrup_crate::de::Deserialize<#des_lifetime> });
                }
            }
        }

        Ok((des_lifetime, des_generics, ser_generics))
    }

    pub(crate) fn generate_deserialize_expr(
        &self,
        deserializer: &Ident,
        visitor: &Ident,
    ) -> Result<(TokenStream, Expr), syn::Error> {
        match &self.from {
            Some(w) => match w {
                With::Verbatim(from_expr) => Ok((
                    quote! {},
                    parse_quote! {
                        #from_expr.deserialize(#deserializer).map(Self)
                    },
                )),
                _ => todo!("deserialize_with"),
            },
            None => match &self.inner {
                Some(inner) => inner.generate_deserialize_expr(self, deserializer, visitor),
                None => {
                    let syrup = &self.syrup_crate;
                    let record_label = &self.name;
                    let self_ty = self.ident;
                    let lifetime = &self.des_lifetime;
                    Ok((
                        quote! {
                            fn visit_sym<E: #syrup::de::DeserializeError>(self, sym: &#lifetime str) -> Result<Self::Value, E> {
                                match sym {
                                    #record_label => Ok(#self_ty),
                                    _ => Err(todo!())
                                }
                            }
                        },
                        parse_quote! {
                            #deserializer.deserialize_sym(#visitor)
                        },
                    ))
                }
            },
        }
    }

    pub(crate) fn generate_serialize_expr(&self, serializer: &Ident) -> Result<Expr, syn::Error> {
        match &self.into {
            Some(_) => errtodo!("serialize_with"),
            None => match &self.inner {
                Some(inner) => inner.generate_serialize_expr(self, serializer),
                None => {
                    let record_label = &self.name;
                    Ok(parse_quote! {
                        #serializer.serialize_sym(#record_label)
                    })
                }
            },
        }
    }
}
