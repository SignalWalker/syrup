use syn::{
    braced, parse_macro_input, parse_quote, parse_quote_spanned, punctuated::Punctuated,
    spanned::Spanned, Attribute, DeriveInput, Expr, Field, Ident, LitStr, Path, Token, Type,
    TypePath, WherePredicate,
};

mod decode;
pub(crate) use decode::*;

mod encode;
pub(crate) use encode::*;

struct OuterAttr {
    syrup: Path,
    label: LitStr,
    // with: Option<Path>,
    decode_where: Punctuated<WherePredicate, Token![,]>,
    encode_where: Punctuated<WherePredicate, Token![,]>,
}

struct Context {
    outer: OuterAttr,
    token_tree_ty: Type,
    decode_error_ty: Type,
    result_ty: Type,
    de_result_ty: Type,
    literal_ty: Type,
}

impl Context {
    fn new(outer: OuterAttr) -> Self {
        let syrup = &outer.syrup;
        let result_ty = parse_quote! { ::std::result::Result };
        let decode_error_ty = parse_quote! { #syrup::de::DecodeError };
        Self {
            // record_ty: parse_quote! { #syrup::de::Record },
            de_result_ty: parse_quote! { #result_ty<Self, #decode_error_ty> },
            result_ty,
            decode_error_ty,
            token_tree_ty: parse_quote! { #syrup::TokenTree },
            // decode_ty: parse_quote! { #syrup::Decode },
            literal_ty: parse_quote! { #syrup::de::Literal },
            outer,
        }
    }
}

impl OuterAttr {
    fn new(ident: &Ident, attrs: &[Attribute]) -> syn::Result<Self> {
        let mut label: Option<LitStr> = None;
        let mut syrup: Option<Path> = None;
        let mut with: Option<Path> = None;
        let mut decode_where: Option<Punctuated<WherePredicate, Token![,]>> = None;
        let mut encode_where: Option<Punctuated<WherePredicate, Token![,]>> = None;
        let mut transparent = false;
        for attr in attrs.iter() {
            if attr.path().is_ident("syrup") {
                attr.parse_nested_meta(|meta| {
                    // the string to use as the record label
                    if meta.path.is_ident("label") {
                        label = Some(meta.value()?.parse()?);
                        Ok(())
                    // use the given module or type's encode/decode functions instead of generating
                    // our own
                    } else if meta.path.is_ident("with") {
                        with = Some(meta.value()?.parse()?);
                        Ok(())
                    // path to the base syrup module (mostly only useful inside the syrup crate
                    // itself)
                    } else if meta.path.is_ident("syrup") {
                        syrup = Some(meta.value()?.parse::<Path>()?);
                        Ok(())
                    // when applied to a struct with a single field, encode/decode it as if it were
                    // that field
                    } else if meta.path.is_ident("transparent") {
                        transparent = true;
                        Ok(())
                    } else if meta.path.is_ident("decode_where") {
                        let content;
                        braced!(content in &meta.value()?);
                        decode_where = Some(Punctuated::parse_terminated(&content)?);
                        Ok(())
                    } else if meta.path.is_ident("encode_where") {
                        let content;
                        braced!(content in &meta.value()?);
                        encode_where = Some(Punctuated::parse_terminated(&content)?);
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized syrup attribute"))
                    }
                })?;
            }
        }
        Ok(Self {
            label: label.unwrap_or_else(|| LitStr::new(ident.to_string().as_str(), ident.span())),
            syrup: syrup.unwrap_or_else(|| parse_quote! { ::syrup }),
            decode_where: decode_where.unwrap_or_default(),
            encode_where: encode_where.unwrap_or_default(),
        })
    }
}

pub(crate) trait FieldEncodeTransform: Fn(&Expr, &Type) -> Expr {}
impl<F> FieldEncodeTransform for F where F: Fn(&Expr, &Type) -> Expr {}

pub(crate) trait FieldDecodeTransform: Fn(&Expr) -> Expr {}
impl<F> FieldDecodeTransform for F where F: Fn(&Expr) -> Expr {}

struct FieldAttr {
    decode: Option<Box<dyn FieldDecodeTransform>>,
    encode: Option<Box<dyn FieldEncodeTransform>>,
}

impl FieldAttr {
    fn new(
        Context {
            outer: OuterAttr { syrup, .. },
            ..
        }: &Context,
        field: &Field,
    ) -> syn::Result<Self> {
        fn expr_decode_from(
            syrup: Path,
            base_ty: Type,
            as_ty: TypePath,
        ) -> impl FieldDecodeTransform {
            move |tokens| {
                parse_quote_spanned! {as_ty.span()=> <#as_ty as ::std::convert::Into<#base_ty>>::into(<#as_ty as #syrup::Decode<'_, _>>::decode(#tokens)) }
            }
        }
        fn expr_encode_into(as_ty: TypePath) -> impl FieldEncodeTransform {
            move |access, accessed_ty| parse_quote_spanned! {as_ty.span()=> <#accessed_ty as ::std::convert::Into<#as_ty>>::into(#access).encode() }
        }
        let attrs = &field.attrs;
        // path to overriding decode function
        let mut decode: Option<Box<dyn FieldDecodeTransform>> = None;
        // path to overriding encode function
        let mut encode: Option<Box<dyn FieldEncodeTransform>> = None;
        for attr in attrs.iter() {
            if attr.path().is_ident("syrup") {
                attr.parse_nested_meta(|meta| {
                    // shorthand for `decode = M::decode, encode = M::encode`
                    if meta.path.is_ident("with") {
                        let m_dec: Path = meta.value()?.parse()?;
                        let m_enc: Path = m_dec.clone();
                        decode = Some(Box::new(
                            move |tokens| parse_quote_spanned! {m_dec.span()=> #m_dec::decode(#tokens) },
                        ));
                        encode = Some(Box::new(
                            move |access, _| parse_quote_spanned! {m_enc.span()=> #m_enc::encode(#access) },
                        ));
                        Ok(())
                    // shorthand for `from = T, into = T`
                    } else if meta.path.is_ident("as") {
                        let as_ty: TypePath = meta.value()?.parse()?;
                        decode = Some(Box::new(
                            expr_decode_from(syrup.clone(), field.ty.clone(), as_ty.clone()),
                        ));
                        encode = Some(Box::new(
                            expr_encode_into(as_ty.clone()),
                        ));
                        Ok(())
                    // decode this field as if it were the given type
                    } else if meta.path.is_ident("from") {
                        decode = Some(Box::new(expr_decode_from(syrup.clone(), field.ty.clone(), meta.value()?.parse()?)));
                        Ok(())
                    // encode this field as if it were the given type
                    } else if meta.path.is_ident("into") {
                        encode = Some(Box::new(expr_encode_into(meta.value()?.parse()?)));
                        Ok(())
                    // decode this field with the given function
                    } else if meta.path.is_ident("decode") {
                        let decode_fn: Path = meta.value()?.parse()?;
                        decode = Some(Box::new(move |tokens| {
                            parse_quote_spanned! {decode_fn.span() => #decode_fn(#tokens)}
                        }));
                        Ok(())
                    // encode this field with the given expression
                    } else if meta.path.is_ident("encode") {
                        let encode_expr: Expr = meta.value()?.parse()?;
                        encode = Some(Box::new(move |_, _| {
                            parse_quote_spanned! {encode_expr.span() => #encode_expr}
                        }));
                        Ok(())
                    // } else if meta.path.is_ident("as_ref") {
                    //     let val: LitBool = meta.value()?.parse()?;
                    //     decode_access_by_ref = val.value;
                    //     encode_access_by_ref = val.value;
                    //     Ok(())
                    // } else if meta.path.is_ident("encode_from_ref") {
                    //     let val: LitBool = meta.value()?.parse()?;
                    //     encode_access_by_ref = val.value;
                    //     Ok(())
                    } else {
                        Err(meta.error("unrecognized syrup attribute"))
                    }
                })?;
            }
        }
        Ok(Self { decode, encode })
    }
}

#[proc_macro_derive(Decode, attributes(syrup))]
pub fn derive_decode(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match generate_decode(input) {
        Ok(res) => res,
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(Encode, attributes(syrup))]
pub fn derive_encode(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate_encode(input) {
        Ok(res) => res,
        Err(e) => e.to_compile_error().into(),
    }
}
