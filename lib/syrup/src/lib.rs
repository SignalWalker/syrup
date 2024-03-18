pub use syrup_derive::{Deserialize, Serialize};

pub mod de;
pub use de::Deserialize;
pub mod ser;
pub use ser::Serialize;

// #[cfg(feature = "serde")]
// pub mod serde;

pub mod extra;

mod error;
pub use error::*;

mod syrup_types;
pub use syrup_types::*;

// #[cfg(feature = "async-stream")]
// pub mod async_stream;

#[macro_export]
macro_rules! record_struct {
    ($Rec:ident, $syrup_name:expr => $($field:ident: $FieldTy:path),+) => {
        #[derive($crate::Serialize, $crate::Deserialize)]
        #[syrup(crate = $crate, name = $syrup_name)]
        struct $Rec {
            $(
                $field: $FieldTy
            ),+
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    #[macro_export]
    macro_rules! test_deserialize {
        ($n:ident; $($arg:ident in $generator:expr),+ => $input:ident, $mk_input:expr => $f:expr => $exp_res:expr) => {
            proptest::proptest! {
                #[test]
                fn $n($($arg in $generator),+) {
                    let $input = $mk_input;
                    assert_eq!($f, Ok($exp_res))
                }
            }
        };
        ($n:ident, $f:expr; $($arg:ident in $generator:expr),+ => $mk_input:expr => $exp_res:expr) => {
            test_deserialize!{$n; $($arg in $generator),+ => __input, $mk_input => $f(&__input) => $exp_res}
        };
        ($f:ident; $($arg:ident in $generator:expr),+ => $mk_input:expr => $exp_left:expr; $exp_res:expr) => {
            test_deserialize!($f, $crate::de::parse::$f::<nom::error::VerboseError<_>>; $($arg in $generator),+ => $mk_input => $exp_left; $exp_res)
        };
    }
}

pub mod optional_map {
    use super::{
        de::{Deserialize, Deserializer},
        ser::{Serialize, SerializeDict, Serializer},
    };
    use std::collections::HashMap;

    pub fn serialize<S: Serializer, K: Serialize, V: Serialize, State>(
        m: &HashMap<K, V, State>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        if m.is_empty() {
            ser.serialize_bool(false)
        } else {
            let mut map = ser.serialize_dictionary(Some(m.len()))?;
            for (k, v) in m {
                map.serialize_entry(k, v)?;
            }
            map.end()
        }
    }

    pub fn deserialize<
        'de,
        D: Deserializer<'de>,
        K: Deserialize<'de> + Eq + PartialEq + std::hash::Hash,
        V: Deserialize<'de>,
        S: Default + std::hash::BuildHasher,
    >(
        de: D,
    ) -> Result<HashMap<K, V, S>, D::Error> {
        Option::<HashMap<K, V, S>>::deserialize(de).map(Option::unwrap_or_default)
    }
}

pub mod bytes {
    pub mod vec {
        use crate::{
            de::{Deserializer, Visitor},
            ser::Serializer,
        };

        pub fn serialize<S: Serializer>(bytes: &[u8], ser: S) -> Result<S::Ok, S::Error> {
            ser.serialize_bytes(bytes)
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Vec<u8>, D::Error> {
            struct __Visitor;
            impl<'de> Visitor<'de> for __Visitor {
                type Value = Vec<u8>;

                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("raw bytes")
                }

                fn visit_byte_buf<E: crate::de::DeserializeError>(
                    self,
                    v: Vec<u8>,
                ) -> Result<Self::Value, E> {
                    Ok(v)
                }

                fn visit_bytes<E: crate::de::DeserializeError>(
                    self,
                    v: &'de [u8],
                ) -> Result<Self::Value, E> {
                    Ok(v.to_owned())
                }
            }
            de.deserialize_byte_buf(__Visitor)
        }
    }
    pub mod slice {
        use crate::{
            de::{Deserializer, Visitor},
            ser::Serializer,
        };

        pub fn serialize<S: Serializer>(bytes: &[u8], ser: S) -> Result<S::Ok, S::Error> {
            ser.serialize_bytes(bytes)
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<&'de [u8], D::Error> {
            struct __Visitor;
            impl<'de> Visitor<'de> for __Visitor {
                type Value = &'de [u8];

                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("raw byte slice")
                }

                fn visit_bytes<E: crate::de::DeserializeError>(
                    self,
                    v: &'de [u8],
                ) -> Result<Self::Value, E> {
                    Ok(v)
                }
            }
            de.deserialize_bytes(__Visitor)
        }
    }
    pub mod array {
        use std::marker::PhantomData;

        use crate::{de::Deserializer, ser::Serializer};

        #[derive(Default)]
        pub struct Visitor<const LEN: usize>(PhantomData<[u8; LEN]>);
        impl<'de, const LEN: usize> crate::de::Visitor<'de> for Visitor<LEN> {
            type Value = [u8; LEN];

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "exactly {LEN} raw bytes")
            }

            fn visit_byte_buf<E: crate::de::DeserializeError>(
                self,
                v: Vec<u8>,
            ) -> Result<Self::Value, E> {
                match v.try_into() {
                    Ok(v) => Ok(v),
                    Err(_) => todo!("deserialize incorrectly sized byte arrays"),
                }
            }

            fn visit_bytes<E: crate::de::DeserializeError>(
                self,
                v: &'de [u8],
            ) -> Result<Self::Value, E> {
                match v.try_into() {
                    Ok(v) => Ok(v),
                    Err(_) => todo!("deserialize incorrectly sized byte arrays"),
                }
            }
        }

        pub fn serialize<S: Serializer, const LEN: usize>(
            bytes: &[u8; LEN],
            ser: S,
        ) -> Result<S::Ok, S::Error> {
            ser.serialize_bytes(bytes)
        }

        pub fn deserialize<'de, D: Deserializer<'de>, const LEN: usize>(
            de: D,
        ) -> Result<[u8; LEN], D::Error> {
            de.deserialize_bytes(Visitor(PhantomData))
        }
    }
}
