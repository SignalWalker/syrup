use std::collections::{HashMap, HashSet};

use super::{Container, Conversion, With};
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{
    parse_quote, punctuated::Punctuated, spanned::Spanned, ConstParam, Expr, GenericParam,
    Generics, Ident, Lifetime, LifetimeParam, Path, PathArguments, Token, Type, TypeParam,
    WherePredicate,
};

pub struct Field<'input> {
    pub ident: Option<&'input Ident>,
    pub ty: &'input Type,

    pub ser_generics: Generics,
    pub des_generics: Generics,

    // /// if not present during deserialization, generate with this function
    // default: Option<Path>,
    pub from: Option<With>,
    pub into: Option<With>,
}

impl<'input> Field<'input> {
    pub fn from_field(
        syrup: &Path,
        field: &'input syn::Field,
        c_ser_generics: &Generics,
        c_des_generics: &Generics,
        c_des_lifetime: &Lifetime,
    ) -> Result<Self, syn::Error> {
        // let mut default: Option<Path> = None;
        let mut from = None;
        let mut into = None;

        for attr in field.attrs.iter().filter(|&a| a.path().is_ident("syrup")) {
            attr.parse_nested_meta(|meta| {
                let attr_id = meta.path.require_ident()?.to_string();
                match attr_id.as_str() {
                    // "default" => {
                    //     default = Some(match meta.input.is_empty() {
                    //         true => {
                    //             parse_quote_spanned! { meta.path.span() => ::std::default::Default::default }
                    //         }
                    //         false => meta.value()?.parse()?,
                    //     })
                    // }
                    // conversion
                    "from" => from = Some(With::infallible(meta.value()?.parse()?)),
                    "into" => into = Some(With::infallible(meta.value()?.parse()?)),
                    "try_from" => from = Some(With::fallible(meta.value()?.parse()?)),
                    "try_into" => into = Some(With::fallible(meta.value()?.parse()?)),
                    "as" => {
                        from = Some(With::infallible(meta.value()?.parse()?));
                        into = from.clone();
                    }
                    "as_symbol" => {
                        from = Some(With::infallible(parse_quote! { #syrup::Symbol<String> }));
                        into = Some(With::infallible(parse_quote! { #syrup::Symbol<&str> }));
                    }
                    "try_as" => {
                        from = Some(With::fallible(meta.value()?.parse()?));
                        into = from.clone();
                    }
                    "deserialize_with" => {
                        from = Some(With::Custom(meta.value()?.parse()?));
                    }
                    "serialize_with" => {
                        into = Some(With::Custom(meta.value()?.parse()?));
                    }
                    "with" => {
                        let mut module: Path = meta.value()?.parse()?;
                        let from_fn = {
                            let mut m = module.clone();
                            m.segments.push(parse_quote! { deserialize });
                            m
                        };
                        let into_fn = {
                            module.segments.push(parse_quote! { serialize });
                            module
                        };
                        from = Some(With::Custom(from_fn));
                        into = Some(With::Custom(into_fn));
                    }
                    _ => return Err(meta.error(format!("unrecognized syrup attribute: {attr_id}"))),
                }
                Ok(())
            })?;
        }

        let (ser_generics, des_generics) = {
            struct ParamData<'p> {
                generics: Generics,
                lifetimes: HashMap<&'p Ident, &'p GenericParam>,
                types: HashMap<&'p Ident, &'p GenericParam>,
                consts: HashMap<&'p Ident, &'p GenericParam>,
                predicates: HashMap<&'p Ident, &'p WherePredicate>,
            }
            impl<'p> ParamData<'p> {
                fn from_params(
                    params: impl IntoIterator<Item = &'p GenericParam>,
                    clauses: Option<impl IntoIterator<Item = &'p WherePredicate>>,
                ) -> Self {
                    let mut lts = HashMap::new();
                    let mut tys = HashMap::new();
                    let mut cnsts = HashMap::new();
                    for p in params {
                        match p {
                            GenericParam::Lifetime(lt) => {
                                lts.insert(&lt.lifetime.ident, p);
                            }
                            GenericParam::Type(t) => {
                                tys.insert(&t.ident, p);
                            }
                            GenericParam::Const(c) => {
                                cnsts.insert(&c.ident, p);
                            }
                        };
                    }
                    let mut preds = HashMap::new();
                    if let Some(clauses) = clauses {
                        for clause in clauses {
                            match &clause {
                                WherePredicate::Lifetime(lt) => {
                                    preds.insert(&lt.lifetime.ident, clause);
                                }
                                WherePredicate::Type(t) => match &t.bounded_ty {
                                    Type::Path(p) => match p.path.get_ident() {
                                        Some(id) => {
                                            preds.insert(id, clause);
                                        }
                                        None => todo!(),
                                    },
                                    _ => todo!(),
                                },
                                _ => todo!("get field generic where predicates for {clause:?}"),
                            }
                        }
                    }
                    Self {
                        generics: Generics {
                            lt_token: Some(Token![<](Span::call_site())),
                            params: Punctuated::new(),
                            gt_token: Some(Token![>](Span::call_site())),
                            where_clause: None,
                        },
                        lifetimes: lts,
                        types: tys,
                        consts: cnsts,
                        predicates: preds,
                    }
                }
                fn insert_predicate(&mut self, id: &Ident) {
                    if let Some(&w) = self.predicates.get(&id) {
                        self.generics.make_where_clause().predicates.push(w.clone());
                    }
                }
                fn insert_lt(&mut self, lt: &Lifetime) {
                    if let Some(&p) = self.lifetimes.get(&lt.ident) {
                        self.generics.params.push(p.clone());
                    }
                    self.insert_predicate(&lt.ident);
                }
                fn insert_type_ident(&mut self, id: &Ident) {
                    if let Some(&p) = self.types.get(id) {
                        self.generics.params.push(p.clone());
                    }
                    self.insert_predicate(id);
                }
            }
            let mut ser_data = ParamData::from_params(
                &c_ser_generics.params,
                c_ser_generics.where_clause.as_ref().map(|w| &w.predicates),
            );
            let mut des_data = ParamData::from_params(
                &c_des_generics.params,
                c_des_generics.where_clause.as_ref().map(|w| &w.predicates),
            );

            let mut type_stack = vec![&field.ty];
            while let Some(ty) = type_stack.pop() {
                match ty {
                    Type::Path(p) => match p.qself.as_ref() {
                        None => match p.path.get_ident() {
                            Some(i) => {
                                ser_data.insert_type_ident(i);
                                des_data.insert_type_ident(i);
                            }
                            None => {
                                for segment in &p.path.segments {
                                    if let PathArguments::AngleBracketed(args) = &segment.arguments
                                    {
                                        for arg in &args.args {
                                            match arg {
                                                syn::GenericArgument::Lifetime(lt) => {
                                                    ser_data.insert_lt(lt);
                                                    des_data.insert_lt(lt);
                                                }
                                                syn::GenericArgument::Type(t) => {
                                                    type_stack.push(t);
                                                }
                                                _ => todo!(
                                                    "extract generics from path argument {arg:?}"
                                                ),
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Some(q) => todo!("extract generics from type path qualifier {q:?}"),
                    },
                    Type::Array(arr) => todo!("extract generics from array type {arr:?}"),
                    Type::BareFn(f) => todo!("extract generics from bare fn {f:?}"),
                    Type::Group(_) => todo!("extract generics from type group"),
                    Type::ImplTrait(_) => todo!("extract generics from impl trait"),
                    Type::Infer(_) => {
                        unreachable!("inferred types aren't allowed in type definitions")
                    }
                    Type::Macro(_) => todo!("extract generics from macro"),
                    Type::Never(_) => {}
                    Type::Paren(_) => todo!("extract generics from parenthesized type"),
                    Type::Ptr(_) => todo!("extract generics from type ptr"),
                    Type::Reference(_) => todo!("extract generics from reference type"),
                    Type::Slice(_) => todo!("extract generics from slice type"),
                    Type::TraitObject(_) => todo!("extract generics from trait object"),
                    Type::Tuple(_) => todo!("extract generics from tuple"),
                    Type::Verbatim(_) => todo!("extract generics from verbatim"),
                    _ => todo!("extract generics from type {:?}", ty),
                }
            }

            (ser_data.generics, des_data.generics)
        };

        Ok(Self {
            ident: field.ident.as_ref(),
            ty: &field.ty,
            ser_generics,
            des_generics,
            // default,
            from,
            into,
        })
    }

    pub fn generate_deserialize(
        &self,
        container: &Container,
        driver: &Expr,
    ) -> Result<Expr, syn::Error> {
        let parse_to = self.ty;
        match &self.from {
            Some(f) => match &f {
                With::Conversion(c) => match &c {
                    Conversion::Infallible(from_ty) => {
                        let self_ty = self.ty;
                        Ok(parse_quote! {
                            #driver::<#from_ty>()?.map(<#self_ty as ::std::convert::From<#from_ty>>::from).unwrap()
                        })
                    }
                    Conversion::Fallible(_from_ty) => {
                        // let self_ty = self.ty;
                        // Ok(parse_quote! {
                        //     #driver::<#from_ty>()?.ok_or_else(|| todo!()).and_then()?
                        // })
                        errtodo!(self.ident.span(), "fallible conversion")
                    }
                },
                With::Custom(des_fn) => {
                    let syrup = &container.syrup_crate;

                    let lifetime = &container.des_lifetime;
                    let (_, ty_generics, where_clause) = self.des_generics.split_for_impl();
                    let impl_generics = {
                        let mut gen = self.des_generics.clone();
                        gen.params.insert(
                            0,
                            GenericParam::Lifetime(LifetimeParam::new(lifetime.clone())),
                        );
                        let (res, _, _) = gen.split_for_impl();
                        res.to_token_stream()
                    };

                    let turbo = ty_generics.as_turbofish();

                    let wrapper_ty = Ident::new("__Wrapper", Span::call_site());
                    let res_ty = self.ty;
                    let des_ty = Ident::new("__Des", Span::call_site());
                    let des = Ident::new("__des", Span::call_site());
                    Ok(parse_quote! {{
                        struct #wrapper_ty #ty_generics (#res_ty);
                        impl #impl_generics #syrup::de::Deserialize<#lifetime> for #wrapper_ty #ty_generics #where_clause {
                            fn deserialize<#des_ty: #syrup::de::Deserializer<#lifetime>>(#des: #des_ty) -> ::std::result::Result<Self, #des_ty::Error> {
                                #des_fn(#des).map(#wrapper_ty #turbo)
                            }
                        }
                        #driver::<#wrapper_ty #ty_generics>()?.unwrap().0
                    }})
                }
            },
            None => Ok(parse_quote! {
                #driver::<#parse_to>()?.unwrap()
            }),
        }
    }

    pub fn generate_serialize_expr(
        &self,
        container: &Container,
        driver: &Expr,
        index: u32,
    ) -> Result<Expr, syn::Error> {
        let field_access = match self.ident {
            Some(id) => quote! { &self.#id },
            None => quote! { &self.#index },
        };
        match &self.into {
            Some(i) => match i {
                With::Conversion(c) => match c {
                    Conversion::Infallible(into_ty) => {
                        let self_ty = self.ty;
                        Ok(parse_quote! {
                            #driver::<#into_ty>(&<&#self_ty as ::std::convert::Into<#into_ty>>::into(#field_access))?
                        })
                    }
                    Conversion::Fallible(_fal) => {
                        errtodo!(self.ident.span(), "fallible conversion")
                    }
                },
                With::Custom(into_fn) => {
                    let syrup = &container.syrup_crate;

                    let wrapper_ty = Ident::new("__Wrapper", Span::call_site());
                    let wrapper_lt = Lifetime::new("'__inner", Span::call_site());

                    let generics = {
                        let mut res = self.ser_generics.clone();
                        res.params.insert(
                            0,
                            GenericParam::Lifetime(LifetimeParam::new(wrapper_lt.clone())),
                        );
                        res
                    };

                    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

                    let res_ty = self.ty;
                    let ser_ty = Ident::new("_Ser", Span::call_site());
                    let ser = Ident::new("__ser", Span::call_site());

                    Ok(parse_quote! {{
                        struct #wrapper_ty #ty_generics (&#wrapper_lt #res_ty);
                        impl #impl_generics #syrup::ser::Serialize for #wrapper_ty #ty_generics #where_clause {
                            #[inline]
                            fn serialize<#ser_ty: #syrup::ser::Serializer>(&self, #ser: #ser_ty) -> ::std::result::Result<#ser_ty::Ok, #ser_ty::Error> {
                                #into_fn(self.0, #ser)
                            }
                        }
                        #driver(&#wrapper_ty(#field_access))?
                    }})
                }
            },
            None => Ok(parse_quote! {
                #driver(#field_access)?
            }),
        }
    }
}
