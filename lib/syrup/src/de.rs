use super::Error;
use std::marker::PhantomData;

mod impl_deserialize;

mod parse;
pub use parse::*;

mod byte_deserializer;
pub use byte_deserializer::*;

mod visitor;
pub use visitor::*;

// mod stream;
// pub use stream::*;

pub use nom::Needed;

pub trait DeserializeError {
    fn needed(&self) -> Option<Needed>;
}

pub trait DeserializeSeed<'input> {
    type Value;

    fn deserialize<D: Deserializer<'input>>(self, de: D) -> Result<Self::Value, D::Error>;
}

impl<'input, D: Deserialize<'input>> DeserializeSeed<'input> for PhantomData<D> {
    type Value = D;

    fn deserialize<De: Deserializer<'input>>(self, de: De) -> Result<Self::Value, De::Error> {
        D::deserialize(de)
    }
}

pub trait DictAccess<'input> {
    type Error: DeserializeError;

    fn next_key_seed<K: DeserializeSeed<'input>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error>;
    fn next_value_seed<V: DeserializeSeed<'input>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error>;

    fn next_entry_seed<K: DeserializeSeed<'input>, V: DeserializeSeed<'input>>(
        &mut self,
        key: K,
        val: V,
    ) -> Result<Option<(K::Value, V::Value)>, Self::Error> {
        match self.next_key_seed(key)? {
            Some(k) => Ok(Some((k, self.next_value_seed(val)?))),
            None => Ok(None),
        }
    }

    fn next_key<K: Deserialize<'input>>(&mut self) -> Result<Option<K>, Self::Error> {
        self.next_key_seed(PhantomData)
    }
    fn next_value<V: Deserialize<'input>>(&mut self) -> Result<V, Self::Error> {
        self.next_value_seed(PhantomData)
    }
    fn next_entry<K: Deserialize<'input>, V: Deserialize<'input>>(
        &mut self,
    ) -> Result<Option<(K, V)>, Self::Error> {
        self.next_entry_seed(PhantomData, PhantomData)
    }

    fn size_hint(&self) -> Option<usize> {
        None
    }
}

pub trait SetAccess<'input> {
    type Error: DeserializeError;

    fn next_key_seed<K: DeserializeSeed<'input>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error>;

    fn next_key<K: Deserialize<'input>>(&mut self) -> Result<Option<K>, Self::Error> {
        self.next_key_seed(PhantomData)
    }

    fn size_hint(&self) -> Option<usize> {
        None
    }
}

pub trait SeqAccess<'input> {
    type Error: DeserializeError;

    fn next_value_seed<V: DeserializeSeed<'input>>(
        &mut self,
        seed: V,
    ) -> Result<Option<V::Value>, Self::Error>;

    fn next_value<V: Deserialize<'input>>(&mut self) -> Result<Option<V>, Self::Error> {
        self.next_value_seed(PhantomData)
    }

    fn size_hint(&self) -> Option<usize> {
        None
    }
}

pub trait RecordAccess<'input>: Sized {
    type Error: DeserializeError;
    type FieldAccess: RecordFieldAccess<'input, Error = Self::Error>;

    fn label_seed<L: DeserializeSeed<'input>>(
        self,
        seed: L,
    ) -> Result<(Self::FieldAccess, L::Value), Self::Error>;

    fn label<L: Deserialize<'input>>(self) -> Result<(Self::FieldAccess, L), Self::Error> {
        self.label_seed(PhantomData)
    }
}

pub trait RecordFieldAccess<'input> {
    type Error: DeserializeError;

    fn next_field_seed<F: DeserializeSeed<'input>>(
        &mut self,
        seed: F,
    ) -> Result<Option<F::Value>, Self::Error>;

    fn next_field<F: Deserialize<'input>>(&mut self) -> Result<Option<F>, Self::Error> {
        self.next_field_seed(PhantomData)
    }

    fn size_hint(&self) -> Option<usize> {
        None
    }
}

pub trait Deserializer<'input> {
    type Error: DeserializeError;

    fn deserialize_any<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;

    fn deserialize_bool<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;

    fn deserialize_i8<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_i16<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_i32<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_i64<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_i128<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_isize<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;

    fn deserialize_u8<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_u16<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_u32<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_u64<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_u128<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_usize<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;

    fn deserialize_f32<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_f64<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;

    fn deserialize_str<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_string<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;

    fn deserialize_sym<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_symbol<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;

    fn deserialize_bytes<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_byte_buf<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;

    fn deserialize_option<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;

    fn deserialize_dictionary<V: Visitor<'input>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error>;
    fn deserialize_sequence<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_record<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_set<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error>;
}

pub trait Deserialize<'input>: Sized {
    fn deserialize<D: Deserializer<'input>>(de: D) -> Result<Self, D::Error>;
}

pub fn from_bytes<'i, T: Deserialize<'i>>(input: &'i [u8]) -> Result<T, Error<'i>> {
    let mut de = ByteDeserializer::from_bytes(input);
    let res = T::deserialize(&mut de)?;
    if de.input.is_empty() {
        Ok(res)
    } else {
        todo!("trailing characters")
        // Err(Error::TrailingCharacters)
    }
}

pub fn nom_bytes<'i, T: Deserialize<'i>>(input: &'i [u8]) -> Result<(&'i [u8], T), Error<'i>> {
    let mut de = ByteDeserializer::from_bytes(input);
    let res = T::deserialize(&mut de)?;
    Ok((de.input, res))
}

#[cfg(test)]
mod test {
    use crate::{de::from_bytes, ser::to_bytes, test_deserialize, Symbol};
    use ibig::UBig;
    use proptest::num;
    use std::collections::HashMap;

    test_deserialize!(deserialize_str;
        value in proptest::string::string_regex(".+").unwrap() => i, format!("{}\"{value}", value.len()) => from_bytes::<&str>(i.as_bytes()) => value.as_str()
    );

    test_deserialize!(deserialize_string;
        value in proptest::string::string_regex(".+").unwrap() => i, format!("{}\"{value}", value.len()) => from_bytes::<String>(i.as_bytes()) => value
    );

    test_deserialize!(deserialize_sym;
        value in proptest::string::string_regex(".+").unwrap() => i, format!("{}'{value}", value.len()) => from_bytes::<Symbol<&str>>(i.as_bytes()) => Symbol(value.as_str())
    );

    test_deserialize!(deserialize_symbol;
        value in proptest::string::string_regex(".+").unwrap() => i, format!("{}'{value}", value.len()) => from_bytes::<Symbol<String>>(i.as_bytes()) => Symbol(value)
    );

    test_deserialize!(deserialize_bytes;
        value in proptest::string::bytes_regex("(?s-u:.+)").unwrap() => i, {
            let mut res = format!("{}:", value.len()).as_bytes().to_owned();
            res.extend_from_slice(&value);
            res
        } => from_bytes::<&[u8]>(&i) => value.as_slice()
    );

    test_deserialize!(deserialize_dictionary;
        value in proptest::collection::hash_map(proptest::num::u8::ANY, proptest::num::u8::ANY, 0..32) => i, {
            to_bytes(&value).unwrap()
        } => from_bytes::<HashMap<u8, u8>>(&i) => value
    );

    // test_deserialize!(deserialize_byte_buf;
    //     value in proptest::string::bytes_regex("(?s-u:.+)").unwrap() => i, {
    //         let mut res = format!("{}:", value.len()).as_bytes().to_owned();
    //         res.extend_from_slice(&value);
    //         res
    //     } => from_bytes::<Vec<u8>>(&i) => value
    // );

    macro_rules! int_to_str {
        ($value:ident, $Int:ty, $UInt:ty) => {
            $value
                .checked_abs()
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("{}", <$UInt>::try_from(<$Int>::MAX).unwrap() + 1))
        };
    }

    macro_rules! format_int {
        ($value:ident, $Int:ty, $UInt:ty) => {
            format!(
                "{}{}",
                int_to_str!($value, $Int, $UInt),
                if $value > 0 { '+' } else { '-' }
            )
        };
    }

    macro_rules! test_int {
        ($test_fn:ident, $Num:ident, $Int:ty, $UInt:ty) => {
            test_deserialize!($test_fn; value in proptest::num::$Num::ANY => i, format_int!(value, $Int, $UInt).into_bytes() => from_bytes::<$Int>(&i) => value);
        };
    }

    test_int!(deserialize_i8, i8, i8, u16);
    test_int!(deserialize_i16, i16, i16, u32);
    test_int!(deserialize_i32, i32, i32, u64);
    test_int!(deserialize_i64, i64, i64, u128);
    test_int!(deserialize_i128, i128, i128, UBig);
    test_int!(deserialize_isize, isize, isize, u128);

    test_deserialize!(deserialize_u8, from_bytes::<u8>;
        value in num::u8::ANY => format!("{value}+").into_bytes() => value
    );
    test_deserialize!(deserialize_u16, from_bytes::<u16>;
        value in num::u16::ANY => format!("{value}+").into_bytes() => value
    );
    test_deserialize!(deserialize_u32, from_bytes::<u32>;
        value in num::u32::ANY => format!("{value}+").into_bytes() => value
    );
    test_deserialize!(deserialize_u64, from_bytes::<u64>;
        value in num::u64::ANY => format!("{value}+").into_bytes() => value
    );
    test_deserialize!(deserialize_u128, from_bytes::<u128>;
        value in num::u128::ANY => format!("{value}+").into_bytes() => value
    );
    test_deserialize!(deserialize_usize, from_bytes::<usize>;
        value in num::usize::ANY => format!("{value}+").into_bytes() => value
    );
}
