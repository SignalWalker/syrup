use nom::{character::streaming as nchar, combinator::verify, error::ParseError, Parser};
use serde::{
    de::{EnumAccess, MapAccess, SeqAccess, VariantAccess},
    Deserialize,
};

use crate::syrup::{
    de::{
        parse_bool, parse_byte_obj, parse_char, parse_f32, parse_f64, parse_i16, parse_i32,
        parse_i64, parse_i8, parse_int, parse_str, parse_symbol, parse_u16, parse_u32, parse_u64,
        parse_u8, parse_unit, parse_unit_record,
    },
    Error,
};

pub struct Deserializer<'input> {
    input: &'input [u8],
}

impl<'i> Deserializer<'i> {
    pub fn from_bytes(input: &'i [u8]) -> Self {
        Self { input }
    }

    fn nom<O, E: ParseError<&'i [u8]>>(
        &mut self,
        mut p: impl Parser<&'i [u8], O, E>,
    ) -> Result<O, E> {
        let (rem, res) = match p.parse(self.input) {
            Ok(o) => o,
            Err(nom::Err::Incomplete(n)) => todo!(),
            Err(nom::Err::Error(e)) => return Err(e),
            Err(nom::Err::Failure(e)) => return Err(e),
        };
        self.input = rem;
        Ok(res)
    }
}

pub fn from_bytes<'i, T: Deserialize<'i>>(input: &'i [u8]) -> Result<T, Error> {
    let mut de = Deserializer::from_bytes(input);
    let res = T::deserialize(&mut de)?;
    if de.input.is_empty() {
        Ok(res)
    } else {
        todo!()
        // Err(Error::TrailingCharacters)
    }
}

macro_rules! deserialize_t {
    ($de:ident, $visitor:ident, $parse_fn:ident, $visit_fn:path) => {{
        $visit_fn($visitor, $de.nom::<_, Self::Error>($parse_fn)?)
    }};
}

macro_rules! deserialize_any {
    ($de:ident, $visitor:ident => $($parse_fn:expr, $visit_fn:path);+) => {
        $(if let Ok(res) = $de.nom::<_, Self::Error>($parse_fn) {
            return $visit_fn($visitor, res);
        })+
    };
}

macro_rules! try_ibig_to_std {
    ($visitor:ident, $ibig:expr => $($Int:ty, $visit_fn:path);+) => {
        $(
        if let Ok(r) = <$Int>::try_from($ibig) {
            return $visit_fn($visitor, r);
        }
        )+
    }
}

impl<'de> serde::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error<'de>;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_any!(self, visitor =>
            parse_bool, V::visit_bool;
            parse_f32, V::visit_f32;
            parse_f64, V::visit_f64
        );
        if let Ok(i) = self.nom::<_, Self::Error>(parse_int) {
            if i >= 0.into() {
                try_ibig_to_std!(visitor, &i =>
                    u8, V::visit_u8;
                    u16, V::visit_u16;
                    u32, V::visit_u32;
                    u64, V::visit_u64
                );
            } else {
                try_ibig_to_std!(visitor, &i =>
                    i8, V::visit_i8;
                    i16, V::visit_i16;
                    i32, V::visit_i32;
                    i64, V::visit_i64
                );
            }
            return Err(todo!());
        }
        deserialize_any!(self, visitor =>
            parse_char, V::visit_char;
            parse_str, V::visit_borrowed_str;
            parse_byte_obj, V::visit_borrowed_bytes
        );
        todo!();
        return Err(todo!());

        // if let Ok(r) = self.nom::<_, Self::Error>(parse_bool) {
        //     visitor.visit_bool(r)
        // } else if let Ok(r) = self.nom::<_, Self::Error>(parse_f32) {
        //     visitor.visit_f32(r)
        // } else {
        //     todo!()
        // }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_bool, V::visit_bool)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_i8, V::visit_i8)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_i16, V::visit_i16)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_i32, V::visit_i32)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_i64, V::visit_i64)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_u8, V::visit_u8)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_u16, V::visit_u16)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_u32, V::visit_u32)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_u64, V::visit_u64)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_f32, V::visit_f32)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_f64, V::visit_f64)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_char, V::visit_char)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_str, V::visit_borrowed_str)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_str, V::visit_borrowed_str)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_byte_obj, V::visit_borrowed_bytes)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        deserialize_t!(self, visitor, parse_byte_obj, V::visit_borrowed_bytes)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.nom::<(), Self::Error>(parse_unit)?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.nom::<_, Self::Error>(parse_unit_record(verify(parse_symbol, |s| s.0 == name)))?;
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.nom::<_, Self::Error>(nchar::char('['))?;
        let res = visitor.visit_seq(DeSequencer::<b']'>::new(&mut *self))?;
        self.nom::<_, Self::Error>(nchar::char(']'))?;
        Ok(res)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.nom::<_, Self::Error>(nchar::char('<'))?;
        self.nom::<_, Self::Error>(verify(parse_symbol, |s| s.0 == name))?;
        let res = visitor.visit_seq(DeSequencer::<b'>'>::new(&mut *self))?;
        self.nom::<_, Self::Error>(nchar::char('>'))?;
        Ok(res)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.nom::<_, Self::Error>(nchar::char('{'))?;
        let res = visitor.visit_map(DeMapper::new(&mut *self))?;
        self.nom::<_, Self::Error>(nchar::char('}'))?;
        Ok(res)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.nom::<_, Self::Error>(nchar::char('<'))?;
        self.nom::<_, Self::Error>(verify(parse_symbol, |s| s.0 == name))?;
        let res = visitor.visit_seq(DeSequencer::<b'>'>::new(&mut *self))?;
        self.nom::<_, Self::Error>(nchar::char('>'))?;
        Ok(res)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.nom::<_, Self::Error>(nchar::char('<'))?;
        let mut variant_index = None;
        for (i, mangled_name) in variants.iter().enumerate().map(|(i, _)| {
            (
                i,
                super::ser::mangle_variant_name(name, i.try_into().unwrap()),
            )
        }) {
            match self.nom::<_, Self::Error>(verify(parse_symbol, |s| s.0 == mangled_name)) {
                Ok(_) => {
                    variant_index = Some(i);
                    break;
                }
                Err(_) => continue,
            }
        }
        let res = visitor.visit_enum(DeEnumerator::new(
            &mut *self,
            name,
            variant_index.unwrap().try_into().unwrap(),
        ))?;
        self.nom::<_, Self::Error>(nchar::char('>'))?;
        Ok(res)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }
}

struct DeSequencer<'de, 'i, const TERM: u8> {
    de: &'de mut Deserializer<'i>,
}

impl<'de, 'i, const TERM: u8> DeSequencer<'de, 'i, TERM> {
    fn new(de: &'de mut Deserializer<'i>) -> Self {
        Self { de }
    }
}

impl<'de, 'i, const TERM: u8> SeqAccess<'i> for DeSequencer<'de, 'i, TERM> {
    type Error = Error<'i>;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'i>,
    {
        if self.de.input.get(0).cloned() == Some(TERM) {
            return Ok(None);
        }
        seed.deserialize(&mut *self.de).map(Some)
    }
}

type DeSetter<'de, 'i, const TERM: u8> = DeSequencer<'de, 'i, TERM>;

struct DeMapper<'de, 'i> {
    de: &'de mut Deserializer<'i>,
}

impl<'de, 'i> DeMapper<'de, 'i> {
    fn new(de: &'de mut Deserializer<'i>) -> Self {
        Self { de }
    }
}

impl<'de, 'i> MapAccess<'i> for DeMapper<'de, 'i> {
    type Error = Error<'i>;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'i>,
    {
        if let Some(b'}') = self.de.input.get(0) {
            return Ok(None);
        }
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'i>,
    {
        seed.deserialize(&mut *self.de)
    }
}

struct DeVariant<'de, 'i> {
    de: &'de mut Deserializer<'i>,
}

impl<'de, 'i> DeVariant<'de, 'i> {
    fn new(de: &'de mut Deserializer<'i>) -> Self {
        Self { de }
    }
}

impl<'de, 'i> VariantAccess<'i> for DeVariant<'de, 'i> {
    type Error = Error<'i>;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'i>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'i>,
    {
        serde::de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'i>,
    {
        serde::de::Deserializer::deserialize_seq(self.de, visitor)
    }
}

struct DeEnumerator<'de, 'i> {
    de: &'de mut Deserializer<'i>,
    enum_name: &'i str,
    variant_index: u32,
}

impl<'de, 'i> DeEnumerator<'de, 'i> {
    fn new(de: &'de mut Deserializer<'i>, enum_name: &'i str, variant_index: u32) -> Self {
        Self {
            de,
            enum_name,
            variant_index,
        }
    }
}

impl<'de, 'i> EnumAccess<'i> for DeEnumerator<'de, 'i> {
    type Error = Error<'i>;

    type Variant = DeVariant<'de, 'i>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: serde::de::DeserializeSeed<'i>,
    {
        Ok((todo!(), Self::Variant::new(self.de)))
    }
}
