#![cfg_attr(feature = "decode-array", feature(maybe_uninit_array_assume_init))]
#![feature(int_from_ascii, trait_alias)]

pub use syrup_derive::{Decode, Encode};

pub use de::{Decode, DecodeError, TokenTree};
pub use ser::Encode;

pub mod de;
pub mod ser;

/// Decode/encode functions for byte string literals.
pub mod bytes;
/// Decode/encode functions for optional collections (where empty collections are encoded as `f`)
pub mod optional_collection;
/// Decode/encode functions for symbol literals.
pub mod symbol;
