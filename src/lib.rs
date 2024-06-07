#![cfg_attr(
    feature = "decode-array",
    feature(maybe_uninit_uninit_array, maybe_uninit_array_assume_init)
)]

pub use syrup_derive::{Decode, Encode};

pub use de::{
    ByteArray, Bytes, Decode, Dictionary, Record, Sequence, Set, Span, Symbol, TokenStream,
    TokenTree,
};
pub use ser::Encode;

pub mod de;
pub mod ser;

/// Decode/encode functions for an optionally-present map, where empty maps are decoded from and
/// encoded to `false`.
pub mod optional_map;

/// Construct a [`Symbol`] from a string literal.
#[macro_export]
macro_rules! symbol {
    [$symbol:literal] => {
        $crate::Symbol(::std::borrow::Cow::Borrowed($symbol))
    };
}

/// Construct a [`TokenTree`] from a literal value.
#[macro_export]
macro_rules! literal {
    [bool; $val:literal] => {
        $crate::literal![$val, $crate::de::Span::new(0, 0) => Bool]
    };
    [f32; $val:literal] => {
        $crate::literal![$val, $crate::de::Span::new(0, 0) => F32]
    };
    [f64; $val:literal] => {
        $crate::literal![$val, $crate::de::Span::new(0, 0) => F64]
    };
    [Symbol; $symbol:literal] => {
        $crate::literal![::std::borrow::Cow::Borrowed($symbol), $crate::de::Span::new(0, 0) => Symbol]
    };
    [String; $string:literal] => {
        $crate::literal![::std::borrow::Cow::Borrowed($string), $crate::de::Span::new(0, 0) => String]
    };
    [Bytes; $bytes:literal] => {
        $crate::literal![::std::borrow::Cow::Borrowed($bytes), $crate::de::Span::new(0, 0) => Bytes]
    };
    [$val:expr, $span:expr => $Ty:ident] => {
        $crate::TokenTree::Literal($crate::de::Literal::new($crate::de::LiteralValue::$Ty($val), $span))
    };
}
