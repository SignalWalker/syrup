use serde::ser::{
    SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};

use crate::syrup::Error;

pub struct Serializer {
    output: Vec<u8>,
}

impl Serializer {
    pub fn serialize_symbol(
        &mut self,
        symbol: impl AsRef<str>,
    ) -> Result<<&mut Self as serde::Serializer>::Ok, <&mut Self as serde::Serializer>::Error> {
        let symbol = symbol.as_ref();
        self.output
            .extend_from_slice(symbol.len().to_string().as_bytes());
        self.output.push(b'\'');
        self.output.extend_from_slice(symbol.as_bytes());
        Ok(())
    }
}

#[inline]
pub fn mangle_variant_name(name: &str, index: u32) -> String {
    format!("{name}__{index}__")
}

impl<'s> serde::Serializer for &'s mut Serializer {
    type Ok = ();

    type Error = Error<'static>;

    type SerializeSeq = Self;

    type SerializeTuple = Self;

    type SerializeTupleStruct = Self;

    type SerializeTupleVariant = Self;

    type SerializeMap = Self;

    type SerializeStruct = Self;

    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.output.push(if v { b't' } else { b'f' });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.output.push(v.to_be_bytes()[0]);
        self.output.push(if v < 0 { b'-' } else { b'+' });
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.output.extend_from_slice(&v.to_be_bytes());
        self.output.push(if v < 0 { b'-' } else { b'+' });
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.output.extend_from_slice(&v.to_be_bytes());
        self.output.push(if v < 0 { b'-' } else { b'+' });
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.output.extend_from_slice(&v.to_be_bytes());
        self.output.push(if v < 0 { b'-' } else { b'+' });
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.output.push(v.to_be_bytes()[0]);
        self.output.push(b'+');
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.output.extend_from_slice(&v.to_be_bytes());
        self.output.push(b'+');
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.output.extend_from_slice(&v.to_be_bytes());
        self.output.push(b'+');
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.output.extend_from_slice(&v.to_be_bytes());
        self.output.push(b'+');
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.output.push(b'F');
        self.output.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.output.push(b'D');
        self.output.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.output
            .extend_from_slice(format!("{}\"{v}", v.len_utf8()).as_bytes());
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.output
            .extend_from_slice(format!("{}\"{v}", v.len()).as_bytes());
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.output
            .extend_from_slice(v.len().to_string().as_bytes());
        self.output.push(b':');
        self.output.extend_from_slice(v);
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        todo!()
        // self.serialize_bool(false)
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
        // value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        todo!()
        // self.output.extend_from_slice(b"[]");
        // Ok(())
    }

    /// Serialize as a symbol.
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_symbol(name)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_symbol(mangle_variant_name(name, variant_index))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        // self.output.push(b'<');
        // self.serialize_symbol(name)?;
        value.serialize(&mut *self)?;
        // self.output.push(b'>');
        Ok(())
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        self.output.push(b'<');
        self.serialize_symbol(mangle_variant_name(name, variant_index))?;
        value.serialize(&mut *self)?;
        self.output.push(b'>');
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.output.push(b'[');
        Ok(self)
    }

    /// Serialize tuples as Syrup sets, which are ordered
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.output.push(b'#');
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.output.push(b'<');
        self.serialize_symbol(name)?;
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.output.push(b'<');
        self.serialize_symbol(mangle_variant_name(name, variant_index))?;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.output.push(b'{');
        Ok(self)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.output.push(b'<');
        self.serialize_symbol(name)?;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.output.push(b'<');
        self.serialize_symbol(mangle_variant_name(name, variant_index))?;
        Ok(self)
    }
}

impl<'s> SerializeSeq for &'s mut Serializer {
    type Ok = <Self as serde::Serializer>::Ok;

    type Error = <Self as serde::Serializer>::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(b']');
        Ok(())
    }
}

impl<'s> SerializeTuple for &'s mut Serializer {
    type Ok = <Self as serde::Serializer>::Ok;

    type Error = <Self as serde::Serializer>::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(b'$');
        Ok(())
    }
}

impl<'s> SerializeTupleStruct for &'s mut Serializer {
    type Ok = <Self as serde::Serializer>::Ok;

    type Error = <Self as serde::Serializer>::Error;

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(b'>');
        Ok(())
    }

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }
}

impl<'s> SerializeTupleVariant for &'s mut Serializer {
    type Ok = <Self as serde::Serializer>::Ok;

    type Error = <Self as serde::Serializer>::Error;

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(b'>');
        Ok(())
    }

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }
}

impl<'s> SerializeMap for &'s mut Serializer {
    type Ok = <Self as serde::Serializer>::Ok;

    type Error = <Self as serde::Serializer>::Error;

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(b'}');
        Ok(())
    }

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        key.serialize(&mut **self)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: serde::Serialize,
        V: serde::Serialize,
    {
        key.serialize(&mut **self)?;
        value.serialize(&mut **self)
    }
}

impl<'s> SerializeStruct for &'s mut Serializer {
    type Ok = <Self as serde::Serializer>::Ok;

    type Error = <Self as serde::Serializer>::Error;

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(b'>');
        Ok(())
    }

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }
}

impl<'s> SerializeStructVariant for &'s mut Serializer {
    type Ok = <Self as serde::Serializer>::Ok;

    type Error = <Self as serde::Serializer>::Error;

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.output.push(b'>');
        Ok(())
    }

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }
}

pub fn to_bytes<T: serde::Serialize>(
    value: &T,
) -> Result<Vec<u8>, <&mut Serializer as serde::Serializer>::Error> {
    let mut serializer = Serializer { output: vec![] };
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}
