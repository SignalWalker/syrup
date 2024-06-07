use std::fmt::Display;

use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
    Serializer,
};

use crate::{
    de::{Bytes, Group, Span, Symbol, TokenTree},
    Encode,
};

#[derive(Debug, thiserror::Error)]
#[error("{}", msg)]
pub struct ByteSerializerError {
    msg: String,
}

impl serde::ser::Error for ByteSerializerError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self {
            msg: msg.to_string(),
        }
    }
}

pub struct ByteSerializer {
    output: Vec<u8>,
}

macro_rules! serialize_trivial {
    ($name:ident, $Ty:ty) => {
        fn $name(self, v: $Ty) -> Result<Self::Ok, Self::Error> {
            self.output.extend_from_slice(&v.to_tokens().encode());
            Ok(())
        }
    };
}

impl<'ser> SerializeSeq for &'ser mut ByteSerializer {
    type Ok = <Self as Serializer>::Ok;
    type Error = <Self as Serializer>::Error;

    fn serialize_element<T>(&mut self, el: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
        T: ?Sized,
    {
        el.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(b']');
        Ok(())
    }
}

impl<'ser> SerializeTuple for &'ser mut ByteSerializer {
    type Ok = <Self as Serializer>::Ok;
    type Error = <Self as Serializer>::Error;

    fn serialize_element<T>(&mut self, el: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
        T: ?Sized,
    {
        <Self as SerializeSeq>::serialize_element(self, el)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as SerializeSeq>::end(self)
    }
}

impl<'ser> SerializeTupleStruct for &'ser mut ByteSerializer {
    type Ok = <Self as Serializer>::Ok;
    type Error = <Self as Serializer>::Error;

    fn serialize_field<T>(&mut self, field: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
        T: ?Sized,
    {
        field.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(b'>');
        Ok(())
    }
}

impl<'ser> SerializeTupleVariant for &'ser mut ByteSerializer {
    type Ok = <Self as Serializer>::Ok;
    type Error = <Self as Serializer>::Error;

    fn serialize_field<T>(&mut self, field: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
        T: ?Sized,
    {
        <Self as SerializeTupleStruct>::serialize_field(self, field)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as SerializeTupleStruct>::end(self)
    }
}

pub struct MapByteSerializer<'ser> {
    ser: &'ser mut ByteSerializer,
}

impl<'ser> SerializeMap for MapByteSerializer<'ser> {
    type Ok = <&'ser mut ByteSerializer as Serializer>::Ok;
    type Error = <&'ser mut ByteSerializer as Serializer>::Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
        T: ?Sized,
    {
        todo!()
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'ser> SerializeStruct for &'ser mut ByteSerializer {
    type Ok = <Self as Serializer>::Ok;
    type Error = <Self as Serializer>::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
        T: ?Sized,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'ser> SerializeStructVariant for &'ser mut ByteSerializer {
    type Ok = <Self as Serializer>::Ok;
    type Error = <Self as Serializer>::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
        T: ?Sized,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'ser> Serializer for &'ser mut ByteSerializer {
    type Ok = ();
    type Error = ByteSerializerError;

    type SerializeSeq = Self;

    type SerializeTuple = Self;

    type SerializeTupleStruct = Self;

    type SerializeTupleVariant = Self;

    type SerializeMap = MapByteSerializer<'ser>;

    type SerializeStruct = Self;

    type SerializeStructVariant = Self;

    serialize_trivial!(serialize_bool, bool);

    serialize_trivial!(serialize_u8, u8);
    serialize_trivial!(serialize_u16, u16);
    serialize_trivial!(serialize_u32, u32);
    serialize_trivial!(serialize_u64, u64);
    // serialize_trivial!(serialize_usize, usize);
    serialize_trivial!(serialize_u128, u128);

    serialize_trivial!(serialize_i8, i8);
    serialize_trivial!(serialize_i16, i16);
    serialize_trivial!(serialize_i32, i32);
    serialize_trivial!(serialize_i64, i64);
    // serialize_trivial!(serialize_isize, isize);
    serialize_trivial!(serialize_i128, i128);

    serialize_trivial!(serialize_f32, f32);
    serialize_trivial!(serialize_f64, f64);

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&v.to_string())
    }

    serialize_trivial!(serialize_str, &str);

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.output
            .extend_from_slice(&Bytes::from(v).to_tokens().encode());
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
        T: ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self.serialize_seq(Some(0))?)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.output.extend_from_slice(
            &TokenTree::Group(
                Group::record_builder(Span::default(), Symbol::from(name).to_tokens()).build(),
            )
            .encode(),
        );
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit_struct(variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
        T: ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
        T: ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.output.push(b'[');
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        todo!("serde serialize symbol")
        // self.output.push(b'<');
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!("serde serialize symbol")
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.output.push(b'{');
        Ok(Self::SerializeMap::new(self, len))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        todo!()
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!()
    }
}
