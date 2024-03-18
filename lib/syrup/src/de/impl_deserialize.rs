use super::{Deserialize, Deserializer};
use crate::{
    de::{DeserializeError, DictAccess, SeqAccess, SetAccess, Visitor},
    Symbol,
};
use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

mod for_tuple {
    use crate::de::{Deserialize, Deserializer, SeqAccess, Visitor};
    use std::marker::PhantomData;

    syrup_proc::impl_deserialize_for_tuple!(32);
}

impl<'input> Deserialize<'input> for bool {
    fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        struct BoolVisitor;
        impl<'i> Visitor<'i> for BoolVisitor {
            type Value = bool;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("bool")
            }
            fn visit_bool<E: DeserializeError>(self, v: bool) -> Result<Self::Value, E> {
                Ok(v)
            }
        }
        de.deserialize_bool(BoolVisitor)
    }
}

impl<'input> Deserialize<'input> for &'input [u8] {
    fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        struct ByteVisitor;
        impl<'input> Visitor<'input> for ByteVisitor {
            type Value = &'input [u8];

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("borrowed bytes")
            }

            fn visit_bytes<E: DeserializeError>(self, v: &'input [u8]) -> Result<Self::Value, E> {
                Ok(v)
            }
        }
        de.deserialize_bytes(ByteVisitor)
    }
}

impl<'input> Deserialize<'input> for Symbol<&'input str> {
    fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        struct SymVisitor;
        impl<'input> Visitor<'input> for SymVisitor {
            type Value = Symbol<&'input str>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("borrowed symbol")
            }

            fn visit_sym<E: DeserializeError>(self, v: &'input str) -> Result<Self::Value, E> {
                Ok(Symbol(v))
            }
        }
        de.deserialize_sym(SymVisitor)
    }
}

impl<'input> Deserialize<'input> for Symbol<String> {
    fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        struct SymbolVisitor;
        impl<'input> Visitor<'input> for SymbolVisitor {
            type Value = Symbol<String>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("owned or borrowed symbol")
            }

            fn visit_sym<E: DeserializeError>(self, v: &'input str) -> Result<Self::Value, E> {
                Ok(Symbol(v.to_owned()))
            }
            fn visit_symbol<E: DeserializeError>(self, v: String) -> Result<Self::Value, E> {
                Ok(Symbol(v))
            }
        }
        de.deserialize_symbol(SymbolVisitor)
    }
}

impl<'input> Deserialize<'input> for &'input str {
    fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        struct StrVisitor;
        impl<'input> Visitor<'input> for StrVisitor {
            type Value = &'input str;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("borrowed string")
            }

            fn visit_str<E: DeserializeError>(self, v: &'input str) -> Result<Self::Value, E> {
                Ok(v)
            }
        }
        de.deserialize_str(StrVisitor)
    }
}

impl<'input> Deserialize<'input> for String {
    fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        struct StringVisitor;
        impl<'input> Visitor<'input> for StringVisitor {
            type Value = String;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("owned or borrowed string")
            }

            fn visit_str<E: DeserializeError>(self, v: &'input str) -> Result<Self::Value, E> {
                Ok(v.to_owned())
            }

            fn visit_string<E: DeserializeError>(self, v: String) -> Result<Self::Value, E> {
                Ok(v)
            }
        }
        de.deserialize_string(StringVisitor)
    }
}

impl<
        'input,
        K: Deserialize<'input> + PartialEq + Eq + std::hash::Hash,
        V: Deserialize<'input>,
        State: std::hash::BuildHasher + Default,
    > Deserialize<'input> for HashMap<K, V, State>
{
    fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        struct MapVisitor<K, V, State> {
            _k: PhantomData<K>,
            _v: PhantomData<V>,
            _state: PhantomData<State>,
        }
        impl<
                'input,
                K: Deserialize<'input> + PartialEq + Eq + std::hash::Hash,
                V: Deserialize<'input>,
                State: std::hash::BuildHasher + Default,
            > Visitor<'input> for MapVisitor<K, V, State>
        {
            type Value = HashMap<K, V, State>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("dictionary")
            }

            fn visit_dictionary<D: DictAccess<'input>>(
                self,
                mut dict: D,
            ) -> Result<Self::Value, D::Error> {
                let mut res = HashMap::<K, V, State>::with_capacity_and_hasher(
                    dict.size_hint().unwrap_or(0),
                    State::default(),
                );
                while let Some((k, v)) = dict.next_entry()? {
                    res.insert(k, v);
                }
                Ok(res)
            }
        }
        de.deserialize_dictionary(MapVisitor {
            _k: PhantomData,
            _v: PhantomData,
            _state: PhantomData,
        })
    }
}

impl<
        'input,
        K: Deserialize<'input> + PartialEq + Eq + std::hash::Hash,
        State: std::hash::BuildHasher + Default,
    > Deserialize<'input> for HashSet<K, State>
{
    fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        struct SetVisitor<K, State> {
            _k: PhantomData<K>,
            _state: PhantomData<State>,
        }
        impl<
                'input,
                K: Deserialize<'input> + PartialEq + Eq + std::hash::Hash,
                State: std::hash::BuildHasher + Default,
            > Visitor<'input> for SetVisitor<K, State>
        {
            type Value = HashSet<K, State>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("set")
            }

            fn visit_set<S: SetAccess<'input>>(self, mut set: S) -> Result<Self::Value, S::Error> {
                let mut res = HashSet::<K, State>::with_capacity_and_hasher(
                    set.size_hint().unwrap_or(0),
                    State::default(),
                );
                while let Some(k) = set.next_key()? {
                    res.insert(k);
                }
                Ok(res)
            }
        }
        de.deserialize_set(SetVisitor {
            _k: PhantomData,
            _state: PhantomData,
        })
    }
}

impl<'input, T: Deserialize<'input>> Deserialize<'input> for Vec<T> {
    fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        struct SeqVisitor<T> {
            _t: PhantomData<T>,
        }
        impl<'input, T: Deserialize<'input>> Visitor<'input> for SeqVisitor<T> {
            type Value = Vec<T>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("sequence")
            }

            fn visit_sequence<S: SeqAccess<'input>>(
                self,
                mut seq: S,
            ) -> Result<Self::Value, S::Error> {
                let mut res = Vec::<T>::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(v) = seq.next_value()? {
                    res.push(v);
                }
                Ok(res)
            }
        }
        de.deserialize_sequence(SeqVisitor { _t: PhantomData })
    }
}

impl<'input, T: Deserialize<'input>, const LEN: usize> Deserialize<'input> for [T; LEN] {
    fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        struct ArrayVisitor<T, const LEN: usize> {
            _t: PhantomData<[T; LEN]>,
        }
        impl<'input, T: Deserialize<'input>, const LEN: usize> Visitor<'input> for ArrayVisitor<T, LEN> {
            type Value = [T; LEN];

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "sequence of length {}", LEN)
            }

            fn visit_sequence<S: SeqAccess<'input>>(
                self,
                mut seq: S,
            ) -> Result<Self::Value, S::Error> {
                let mut res = Vec::<T>::with_capacity(LEN);
                while let Some(v) = seq.next_value()? {
                    res.push(v);
                }
                Ok(match res.try_into() {
                    Ok(r) => r,
                    Err(_) => todo!(),
                })
            }
        }
        de.deserialize_sequence(ArrayVisitor { _t: PhantomData })
    }
}

impl<'input, T: Deserialize<'input>> Deserialize<'input> for Option<T> {
    fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        struct OptionVisitor<T> {
            _t: PhantomData<T>,
        }
        impl<'input, T: Deserialize<'input>> Visitor<'input> for OptionVisitor<T> {
            type Value = Option<T>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "either false or an object of type T")
            }

            fn visit_none<E: DeserializeError>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_some<D: Deserializer<'input>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                T::deserialize(deserializer).map(Some)
            }
        }
        de.deserialize_option(OptionVisitor { _t: PhantomData })
    }
}

macro_rules! deserialize_int {
    ($Int:ty, $de_fn:ident => $($visit_fn:ident, $From:ty, $v:ident, $from:expr);+$(;)?) => {
        impl<'input> Deserialize<'input> for $Int {
            fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
                struct __Visitor;
                impl<'input> Visitor<'input> for __Visitor {
                    type Value = $Int;

                    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(f, "integer within ({}..{})", <$Int>::MIN, <$Int>::MAX)
                    }

                    $(
                    fn $visit_fn<E: DeserializeError>(self, $v: $From) -> Result<Self::Value, E> {
                        Ok($from)
                    }
                    )+
                }
                de.$de_fn(__Visitor)
            }
        }
    };
    ($Int:ty, $de_fn:ident => $($visit_fn:ident, $From:ty);+$(;)?) => {
        deserialize_int!{$Int, $de_fn => $($visit_fn, $From, v, <$Int>::from(v);)+}
    }
}

deserialize_int!(i8, deserialize_i8 =>
    visit_i8, i8;
);
deserialize_int!(i16, deserialize_i16 =>
    visit_i8, i8;
    visit_i16, i16;
);
deserialize_int!(i32, deserialize_i32 =>
    visit_i8, i8;
    visit_i16, i16;
    visit_i32, i32;
);
deserialize_int!(i64, deserialize_i64 =>
    visit_i8, i8;
    visit_i16, i16;
    visit_i32, i32;
    visit_i64, i64;
);
deserialize_int!(i128, deserialize_i128 =>
    visit_i8, i8;
    visit_i16, i16;
    visit_i32, i32;
    visit_i64, i64;
    visit_i128, i128;
);
#[cfg(target_pointer_width = "16")]
deserialize_int!(isize, deserialize_isize =>
    visit_i8, i8;
    visit_i16, i16;
    visit_isize, isize;
);
#[cfg(target_pointer_width = "32")]
deserialize_int!(isize, deserialize_isize =>
    visit_i8, i8, v, isize::from(v);
    visit_i16, i16, v, isize::from(v);
    visit_i32, i32, v, isize::try_from(v).unwrap();
    visit_isize, isize, v, v
);
#[cfg(target_pointer_width = "64")]
deserialize_int!(isize, deserialize_isize =>
    visit_i8, i8, v, isize::from(v);
    visit_i16, i16, v, isize::from(v);
    visit_i32, i32, v, isize::try_from(v).unwrap();
    visit_i64, i64, v, isize::try_from(v).unwrap();
    visit_isize, isize, v, v
);

deserialize_int!(u8, deserialize_u8 =>
    visit_u8, u8;
);
deserialize_int!(u16, deserialize_u16 =>
    visit_u8, u8;
    visit_u16, u16;
);
deserialize_int!(u32, deserialize_u32 =>
    visit_u8, u8;
    visit_u16, u16;
    visit_u32, u32;
);
deserialize_int!(u64, deserialize_u64 =>
    visit_u8, u8;
    visit_u16, u16;
    visit_u32, u32;
    visit_u64, u64;
);
deserialize_int!(u128, deserialize_u128 =>
    visit_u8, u8;
    visit_u16, u16;
    visit_u32, u32;
    visit_u64, u64;
    visit_u128, u128;
);
#[cfg(target_pointer_width = "16")]
deserialize_int!(usize, deserialize_usize =>
    visit_u8, u8;
    visit_u16, u16;
    visit_usize, usize;
);
#[cfg(target_pointer_width = "32")]
deserialize_int!(usize, deserialize_usize =>
    visit_u8, u8, v, usize::from(v);
    visit_u16, u16, v, usize::from(v);
    visit_u32, u32, v, usize::try_from(v).unwrap();
    visit_usize, usize, v, v
);
#[cfg(target_pointer_width = "64")]
deserialize_int!(usize, deserialize_usize =>
    visit_u8, u8, v, usize::from(v);
    visit_u16, u16, v, usize::from(v);
    visit_u32, u32, v, usize::try_from(v).unwrap();
    visit_u64, u64, v, usize::try_from(v).unwrap();
    visit_usize, usize, v, v
);

macro_rules! deserialize_float {
    ($Float:ty, $de_fn:ident => $($visit_fn:ident, $From:ty);+$(;)?) => {
        impl<'input> Deserialize<'input> for $Float {
            fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
                struct __Visitor;
                impl<'input> Visitor<'input> for __Visitor {
                    type Value = $Float;

                    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(f, "float within ({}..{})", <$Float>::MIN, <$Float>::MAX)
                    }

                    $(
                    fn $visit_fn<E: DeserializeError>(self, v: $From) -> Result<Self::Value, E> {
                        Ok(<$Float>::from(v))
                    }
                    )+
                }
                de.$de_fn(__Visitor)
            }
        }
    };
}

deserialize_float!(f32, deserialize_f32 => visit_f32, f32);
deserialize_float!(f64, deserialize_f64 => visit_f32, f32; visit_f64, f64);
