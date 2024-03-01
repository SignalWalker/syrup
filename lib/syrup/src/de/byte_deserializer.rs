use super::{
    parse_bool, parse_f32, parse_f64, parse_i128, parse_i16, parse_i32, parse_i64, parse_i8,
    parse_isize, parse_u128, parse_u16, parse_u32, parse_u64, parse_u8, parse_usize,
    DeserializeSeed, Deserializer, DictAccess, RecordAccess, RecordFieldAccess, SeqAccess,
    SetAccess, Visitor,
};
use crate::{
    de::{parse_byte_obj, parse_int, parse_str, parse_symbol},
    Error,
};
use nom::{character::streaming as nchar, Parser};

pub struct ByteDeserializer<'input> {
    pub(super) input: &'input [u8],
}

impl<'i> ByteDeserializer<'i> {
    pub fn from_bytes(input: &'i [u8]) -> Self {
        Self { input }
    }

    fn nom<'s, O>(
        &'s mut self,
        mut p: impl nom::Parser<&'i [u8], O, <&'s mut Self as Deserializer<'i>>::Error>,
    ) -> Result<O, <&'s mut Self as Deserializer<'i>>::Error> {
        let (rem, res) = p.parse(self.input)?;
        self.input = rem;
        Ok(res)
    }

    #[inline]
    fn peek(&self) -> Option<u8> {
        self.input.get(0).cloned()
    }
}

macro_rules! deserialize_simple {
    ($deserialize_fn:ident, $visit_fn:ident, $parse_expr:expr) => {
        fn $deserialize_fn<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            visitor.$visit_fn(self.nom($parse_expr)?)
        }
    };
}

macro_rules! deserialize_any {
    ($de:ident, $visitor:ident => $($parse_fn:expr, $visit_fn:path);+$(;)?) => {
        $(if let Ok(res) = $de.nom($parse_fn) {
            return $visit_fn($visitor, res);
        })+
    };
}

macro_rules! try_ibig_to_std {
    ($visitor:ident, $ibig:expr => $($Int:ty, $visit_fn:path);+$(;)?) => {
        $(
        if let Ok(r) = <$Int>::try_from($ibig) {
            return $visit_fn($visitor, r);
        }
        )+
    }
}

impl<'input> Deserializer<'input> for &mut ByteDeserializer<'input> {
    type Error = Error<'input>;

    fn deserialize_any<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let peek = match self.peek() {
            Some(p) => p,
            None => {
                return Err(Error {
                    input: Some(self.input),
                    kind: crate::ErrorKind::Incomplete(nom::Needed::Unknown),
                })
            }
        };
        match peek {
            b't' | b'f' => self.deserialize_bool(visitor),
            b'F' => self.deserialize_f32(visitor),
            b'D' => self.deserialize_f64(visitor),
            b'{' => self.deserialize_dictionary(visitor),
            b'[' => self.deserialize_sequence(visitor),
            b'#' => self.deserialize_set(visitor),
            b'<' => self.deserialize_record(visitor),
            b'0'..=b'9' => {
                if let Ok(i) = self.nom(parse_int) {
                    if i >= 0.into() {
                        try_ibig_to_std!(visitor, &i =>
                            u8, V::visit_u8;
                            u16, V::visit_u16;
                            u32, V::visit_u32;
                            u64, V::visit_u64;
                            u128, V::visit_u128;
                        );
                    } else {
                        try_ibig_to_std!(visitor, &i =>
                            i8, V::visit_i8;
                            i16, V::visit_i16;
                            i32, V::visit_i32;
                            i64, V::visit_i64;
                            i128, V::visit_i128;
                        );
                    }
                    return Err(todo!());
                }
                deserialize_any!(self, visitor =>
                    parse_str, V::visit_str;
                    parse_symbol.map(|s| s.0), V::visit_sym;
                    parse_byte_obj, V::visit_bytes;
                );
                Err(todo!())
            }
            _ => todo!(),
        }
    }

    deserialize_simple!(deserialize_bool, visit_bool, parse_bool);

    deserialize_simple!(deserialize_i8, visit_i8, parse_i8);
    deserialize_simple!(deserialize_i16, visit_i16, parse_i16);
    deserialize_simple!(deserialize_i32, visit_i32, parse_i32);
    deserialize_simple!(deserialize_i64, visit_i64, parse_i64);
    deserialize_simple!(deserialize_i128, visit_i128, parse_i128);
    deserialize_simple!(deserialize_isize, visit_isize, parse_isize);

    deserialize_simple!(deserialize_u8, visit_u8, parse_u8);
    deserialize_simple!(deserialize_u16, visit_u16, parse_u16);
    deserialize_simple!(deserialize_u32, visit_u32, parse_u32);
    deserialize_simple!(deserialize_u64, visit_u64, parse_u64);
    deserialize_simple!(deserialize_u128, visit_u128, parse_u128);
    deserialize_simple!(deserialize_usize, visit_usize, parse_usize);

    deserialize_simple!(deserialize_f32, visit_f32, parse_f32);
    deserialize_simple!(deserialize_f64, visit_f64, parse_f64);

    deserialize_simple!(deserialize_str, visit_str, parse_str);
    deserialize_simple!(
        deserialize_string,
        visit_string,
        parse_str.map(ToOwned::to_owned)
    );

    deserialize_simple!(deserialize_sym, visit_sym, parse_symbol.map(|s| s.0));
    deserialize_simple!(
        deserialize_symbol,
        visit_symbol,
        parse_symbol.map(|s| s.0.to_owned())
    );

    deserialize_simple!(deserialize_bytes, visit_bytes, parse_byte_obj);
    deserialize_simple!(
        deserialize_byte_buf,
        visit_byte_buf,
        parse_byte_obj.map(ToOwned::to_owned)
    );

    fn deserialize_option<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.nom(nchar::char('f')) {
            Ok(_) => visitor.visit_none(),
            Err(e) => match e.kind {
                crate::ErrorKind::Parse(crate::ParseErrorKind::Nom(
                    nom::error::ErrorKind::Char,
                )) => visitor.visit_some(self),
                _ => Err(e),
            },
        }
    }

    fn deserialize_dictionary<V: Visitor<'input>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.nom(nchar::char('{'))?;
        let res = visitor.visit_dictionary(&mut *self)?;
        self.nom(nchar::char('}'))?;
        Ok(res)
    }

    fn deserialize_sequence<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.nom(nchar::char('['))?;
        let res = visitor.visit_sequence(&mut *self)?;
        self.nom(nchar::char(']'))?;
        Ok(res)
    }

    fn deserialize_record<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.nom(nchar::char('<'))?;
        let res = visitor.visit_record(&mut *self)?;
        self.nom(nchar::char('>'))?;
        Ok(res)
    }

    fn deserialize_set<V: Visitor<'input>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.nom(nchar::char('#'))?;
        let res = visitor.visit_set(&mut *self)?;
        self.nom(nchar::char('$'))?;
        Ok(res)
    }
}

impl<'i> RecordFieldAccess<'i> for &mut ByteDeserializer<'i> {
    type Error = <Self as Deserializer<'i>>::Error;

    fn next_field_seed<F: DeserializeSeed<'i>>(
        &mut self,
        seed: F,
    ) -> Result<Option<F::Value>, Self::Error> {
        if self.input.get(0).cloned() == Some(b'>') {
            Ok(None)
        } else {
            seed.deserialize(&mut **self).map(Some)
        }
    }
}

impl<'i> RecordAccess<'i> for &mut ByteDeserializer<'i> {
    type Error = <Self as Deserializer<'i>>::Error;

    type FieldAccess = Self;

    fn label_seed<L: DeserializeSeed<'i>>(
        self,
        seed: L,
    ) -> Result<(Self::FieldAccess, L::Value), Self::Error> {
        let res = seed.deserialize(&mut *self)?;
        Ok((self, res))
    }
}

impl<'i> SeqAccess<'i> for &mut ByteDeserializer<'i> {
    type Error = <Self as Deserializer<'i>>::Error;

    fn next_value_seed<V: DeserializeSeed<'i>>(
        &mut self,
        seed: V,
    ) -> Result<Option<V::Value>, Self::Error> {
        if self.input.get(0).cloned() == Some(b']') {
            Ok(None)
        } else {
            seed.deserialize(&mut **self).map(Some)
        }
    }
}

impl<'i> SetAccess<'i> for &mut ByteDeserializer<'i> {
    type Error = <Self as Deserializer<'i>>::Error;

    fn next_key_seed<V: DeserializeSeed<'i>>(
        &mut self,
        seed: V,
    ) -> Result<Option<V::Value>, Self::Error> {
        if self.input.get(0).cloned() == Some(b'$') {
            Ok(None)
        } else {
            seed.deserialize(&mut **self).map(Some)
        }
    }
}

impl<'i> DictAccess<'i> for &mut ByteDeserializer<'i> {
    type Error = <Self as Deserializer<'i>>::Error;

    fn next_key_seed<V: DeserializeSeed<'i>>(
        &mut self,
        seed: V,
    ) -> Result<Option<V::Value>, Self::Error> {
        if self.input.get(0).cloned() == Some(b'}') {
            Ok(None)
        } else {
            seed.deserialize(&mut **self).map(Some)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'i>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        seed.deserialize(&mut **self)
    }
}
