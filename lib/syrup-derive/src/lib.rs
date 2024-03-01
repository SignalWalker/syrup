use de::generate_deserialize;
use ser::generate_serialize;
use syn::{parse_macro_input, DeriveInput};

pub(crate) mod common;

mod de;
mod ser;

#[proc_macro_derive(Deserialize, attributes(syrup))]
pub fn derive_deserialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate_deserialize(&input) {
        Ok(res) => res.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(Serialize, attributes(syrup))]
pub fn derive_serialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate_serialize(&input) {
        Ok(res) => res.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
