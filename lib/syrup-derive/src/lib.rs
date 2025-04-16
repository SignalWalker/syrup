use proc_macro2::Span;
use syn::{
    parse_macro_input, parse_quote, parse_quote_spanned, spanned::Spanned, Attribute, DeriveInput,
    Ident, Lifetime, LitStr, Path, Type,
};

mod decode;
pub(crate) use decode::*;

mod encode;
pub(crate) use encode::*;

struct OuterAttr {
    syrup: Path,
    label: LitStr,
    // with: Option<Path>,
}

struct Context {
    outer: OuterAttr,
    cow_ty: Type,
    token_tree_ty: Type,
    decode_error_ty: Type,
    result_ty: Type,
    de_result_ty: Type,
    // record_ty: Type,
    // decode_ty: Type,
    literal_ty: Type,
    de_error_lt: Lifetime,
}

impl Context {
    fn new(outer: OuterAttr) -> Self {
        let syrup = &outer.syrup;
        let result_ty = parse_quote! { ::std::result::Result };
        let decode_error_ty = parse_quote! { #syrup::de::DecodeError };
        let de_error_lt = Lifetime::new("'__error", Span::call_site());
        Self {
            // record_ty: parse_quote! { #syrup::de::Record },
            cow_ty: parse_quote! { ::std::borrow::Cow },
            de_result_ty: parse_quote! { #result_ty<Self, #decode_error_ty<#de_error_lt>> },
            result_ty,
            decode_error_ty,
            de_error_lt,
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
                    } else {
                        Err(meta.error("unrecognized syrup attribute"))
                    }
                })?;
            }
        }
        Ok(Self {
            label: label.unwrap_or_else(|| LitStr::new(ident.to_string().as_str(), ident.span())),
            // with,
            syrup: syrup.unwrap_or_else(|| parse_quote! { ::syrup }),
        })
    }
}

struct FieldAttr {
    decode: Option<Path>,
    encode: Option<Path>,
}

impl FieldAttr {
    fn new(attrs: &[Attribute]) -> syn::Result<Self> {
        // path to overriding decode function
        let mut decode: Option<Path> = None;
        // path to overriding encode function
        let mut encode: Option<Path> = None;
        for attr in attrs.iter() {
            if attr.path().is_ident("syrup") {
                attr.parse_nested_meta(|meta| {
                    // shorthand for `decode = M::decode, encode = M::encode`
                    if meta.path.is_ident("with") {
                        let m: Path = meta.value()?.parse()?;
                        decode = Some(parse_quote_spanned! {m.span()=> #m::decode});
                        encode = Some(parse_quote_spanned! {m.span()=> #m::encode});
                        Ok(())
                    // shorthand for `from = T, into = T`
                    } else if meta.path.is_ident("as") {
                        Err(meta.error("todo: field as"))
                    // decode this field as if it were the given type
                    } else if meta.path.is_ident("from") {
                        Err(meta.error("todo: field from"))
                    // encode this field as if it were the given type
                    } else if meta.path.is_ident("into") {
                        Err(meta.error("todo: field into"))
                    // decode this field with the given function
                    } else if meta.path.is_ident("decode") {
                        decode = Some(meta.value()?.parse()?);
                        Ok(())
                    // encode this field with the given function
                    } else if meta.path.is_ident("encode") {
                        encode = Some(meta.value()?.parse()?);
                        Ok(())
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
