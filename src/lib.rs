#![cfg_attr(feature = "decode-array", feature(maybe_uninit_array_assume_init))]
// #![feature(int_from_ascii, trait_alias)]

pub use syrup_derive::{Decode, Encode};

pub use de::{Decode, DecodeError, TokenTree};
pub use ser::Encode;

pub use borrow_or_share;
pub use nom;

pub mod de;
pub mod ser;

/// Decode/encode functions for byte string literals.
pub mod bytes;
/// Decode/encode functions for optional collections (where empty collections are encoded as `f`)
pub mod optional_collection;
/// Decode/encode functions for symbol literals.
pub mod symbol;

/// Construct a [`TokenTree::List`] by encoding a sequence of elements.
#[macro_export]
macro_rules! list {
    () => {
        $crate::TokenTree::List($crate::de::List { elements: ::std::vec![] })
    };
    ($($elem:expr),+ $(,)?) => {
        $crate::TokenTree::List($crate::de::List { elements: ::std::vec![$($crate::Encode::encode($elem)),+] })
    };
}

/// Construct a [`TokenTree::Literal`] from a given value.
#[macro_export]
macro_rules! literal {
    [$Ty:ident; $val:expr] => {
        $crate::TokenTree::Literal($crate::de::Literal::$Ty($val))
    };
}
