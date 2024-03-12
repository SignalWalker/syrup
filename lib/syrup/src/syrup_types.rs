use crate::{
    de::{RecordFieldAccess, Visitor},
    ser::{ByteSerializer, SerializeDict, SerializeRecord, SerializeSeq, SerializeSet, Serializer},
    Deserialize, Serialize,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol<T>(pub T);

impl From<Symbol<String>> for String {
    #[inline]
    fn from(value: Symbol<String>) -> Self {
        value.0
    }
}

impl From<String> for Symbol<String> {
    #[inline]
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl<'s> From<&'s str> for Symbol<&'s str> {
    #[inline]
    fn from(value: &'s str) -> Self {
        Self(value)
    }
}

impl<'s> From<&'s String> for Symbol<&'s str> {
    #[inline]
    fn from(value: &'s String) -> Self {
        Self(value.as_str())
    }
}

impl<'s> From<Symbol<&'s str>> for &'s str {
    #[inline]
    fn from(value: Symbol<&'s str>) -> Self {
        value.0
    }
}

#[derive(Clone)]
pub struct RawSyrup {
    data: Vec<u8>,
}

impl RawSyrup {
    pub fn try_from_serialize(
        s: &(impl Serialize + ?Sized),
    ) -> Result<Self, <&mut ByteSerializer as Serializer>::Error> {
        Ok(Self {
            data: crate::ser::to_bytes(s)?,
        })
    }

    pub fn from_serialize(s: &(impl Serialize + ?Sized)) -> Self {
        Self::try_from_serialize(s).unwrap()
    }

    /// # Safety
    /// - Input data must be valid Syrup.
    #[allow(unsafe_code)]
    pub unsafe fn from_raw(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Serialize for RawSyrup {
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        #[allow(unsafe_code)]
        unsafe {
            s.serialize_raw(&self.data)
        }
    }
}

#[macro_export]
macro_rules! raw_syrup {
    [$($item:expr),* $(,)?] => {
        vec![$($crate::RawSyrup::try_from_serialize($item)?),*]
    };
    [$($item:expr),* $(,)?; $iter:expr] => {
        {
            let mut __res = $crate::raw_syrup![$($item),*];
            __res.extend($iter.into_iter().map($crate::RawSyrup::from_serialize));
            __res
        }
    };
}

#[macro_export]
macro_rules! raw_syrup_unwrap {
    [$($item:expr),* $(,)?] => {
        vec![$($crate::RawSyrup::try_from_serialize($item).unwrap()),*]
    };
    [$($item:expr),* $(,)?; $iter:expr] => {
        {
            let mut __res = $crate::raw_syrup_unwrap![$($item),*];
            __res.extend($iter.into_iter().map($crate::RawSyrup::from_serialize));
            __res
        }
    };
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Bytes<Base>(pub Base);

impl<'b> Serialize for Bytes<&'b [u8]> {
    #[inline]
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        crate::bytes::slice::serialize(self.0, s)
    }
}

impl Serialize for Bytes<Vec<u8>> {
    #[inline]
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        crate::bytes::vec::serialize(&self.0, s)
    }
}

impl<const LEN: usize> Serialize for Bytes<[u8; LEN]> {
    #[inline]
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        crate::bytes::array::serialize(&self.0, s)
    }
}

impl<'input> Deserialize<'input> for Bytes<&'input [u8]> {
    fn deserialize<D: crate::de::Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        crate::bytes::slice::deserialize(de).map(Self)
    }
}

impl<'input> Deserialize<'input> for Bytes<Vec<u8>> {
    fn deserialize<D: crate::de::Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        crate::bytes::vec::deserialize(de).map(Self)
    }
}

impl<'input, const LEN: usize> Deserialize<'input> for Bytes<[u8; LEN]> {
    fn deserialize<D: crate::de::Deserializer<'input>>(de: D) -> Result<Self, D::Error> {
        crate::bytes::array::deserialize(de).map(Self)
    }
}

impl<'b> From<&'b [u8]> for Bytes<&'b [u8]> {
    #[inline]
    fn from(value: &'b [u8]) -> Self {
        Self(value)
    }
}

impl From<Vec<u8>> for Bytes<Vec<u8>> {
    #[inline]
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl<const LEN: usize> From<[u8; LEN]> for Bytes<[u8; LEN]> {
    #[inline]
    fn from(value: [u8; LEN]) -> Self {
        Self(value)
    }
}

impl<'b> From<Bytes<&'b [u8]>> for &'b [u8] {
    #[inline]
    fn from(value: Bytes<&'b [u8]>) -> Self {
        value.0
    }
}

impl From<Bytes<Vec<u8>>> for Vec<u8> {
    #[inline]
    fn from(value: Bytes<Vec<u8>>) -> Self {
        value.0
    }
}

impl<const LEN: usize> From<Bytes<[u8; LEN]>> for [u8; LEN] {
    #[inline]
    fn from(value: Bytes<[u8; LEN]>) -> Self {
        value.0
    }
}

#[derive(Clone, PartialEq)]
pub enum Item {
    Bool(bool),
    F32(f32),
    F64(f64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    ISize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    USize(usize),
    String(String),
    Symbol(String),
    Bytes(Vec<u8>),
    Option(Option<Box<Item>>),
    Dictionary(Vec<(Item, Item)>),
    Sequence(Vec<Item>),
    Record(Symbol<String>, Vec<Item>),
    Set(Vec<Item>),
}

impl std::fmt::Debug for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&crate::ser::to_pretty(self).unwrap())
    }
}

pub trait FromSyrupItem: Sized {
    fn from_syrup_item(item: &Item) -> Result<Self, &Item>;
}

pub trait AsSyrupItem {
    fn as_syrup_item(&self) -> Option<Item>;
}

impl<T> FromSyrupItem for T
where
    for<'de> T: Deserialize<'de>,
{
    fn from_syrup_item(item: &Item) -> Result<Self, &Item> {
        let serialized = crate::ser::to_bytes(item).unwrap();
        crate::de::from_bytes(&serialized).map_err(|_| item)
    }
}

impl<T: Serialize> AsSyrupItem for T {
    fn as_syrup_item(&self) -> Option<Item> {
        let serialized = crate::ser::to_bytes(self).unwrap();
        crate::de::from_bytes(&serialized).ok()
    }
}

impl Serialize for Item {
    fn serialize<Ser: crate::ser::Serializer>(&self, ser: Ser) -> Result<Ser::Ok, Ser::Error> {
        match self {
            Item::Bool(b) => b.serialize(ser),
            Item::F32(f) => f.serialize(ser),
            Item::F64(d) => d.serialize(ser),
            Item::I8(i) => i.serialize(ser),
            Item::I16(i) => i.serialize(ser),
            Item::I32(i) => i.serialize(ser),
            Item::I64(i) => i.serialize(ser),
            Item::I128(i) => i.serialize(ser),
            Item::ISize(i) => i.serialize(ser),
            Item::U8(i) => i.serialize(ser),
            Item::U16(i) => i.serialize(ser),
            Item::U32(i) => i.serialize(ser),
            Item::U64(i) => i.serialize(ser),
            Item::U128(i) => i.serialize(ser),
            Item::USize(i) => i.serialize(ser),
            Item::String(s) => s.serialize(ser),
            Item::Symbol(s) => Symbol(s.as_str()).serialize(ser),
            Item::Bytes(b) => Bytes(b.as_slice()).serialize(ser),
            Item::Option(o) => o.serialize(ser),
            Item::Dictionary(d) => {
                let mut dict = ser.serialize_dictionary(Some(d.len()))?;
                for (k, v) in d {
                    dict.serialize_entry(k, v)?;
                }
                dict.end()
            }
            Item::Sequence(s) => {
                let mut seq = ser.serialize_sequence(Some(s.len()))?;
                for i in s {
                    seq.serialize_element(i)?;
                }
                seq.end()
            }
            Item::Record(label, fields) => {
                let mut rec = ser.serialize_record(&label.0, Some(fields.len()))?;
                for field in fields {
                    rec.serialize_field(field)?;
                }
                rec.end()
            }
            Item::Set(s) => {
                let mut set = ser.serialize_set(Some(s.len()))?;
                for i in s {
                    set.serialize_element(i)?;
                }
                set.end()
            }
        }
    }
}

macro_rules! simple_visit {
    ($name:ident, $Value:ty, $Item:ident) => {
        fn $name<E: crate::de::DeserializeError>(self, v: $Value) -> Result<Self::Value, E> {
            Ok(Item::$Item(v))
        }
    };
}

impl<'de> Deserialize<'de> for Item {
    fn deserialize<D: crate::de::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct __Visitor;
        impl<'de> Visitor<'de> for __Visitor {
            type Value = Item;

            fn expecting(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                todo!()
            }

            simple_visit! {visit_bool, bool, Bool}

            simple_visit! {visit_i8,    i8,    I8}
            simple_visit! {visit_i16,   i16,   I16}
            simple_visit! {visit_i32,   i32,   I32}
            simple_visit! {visit_i64,   i64,   I64}
            simple_visit! {visit_i128,  i128,  I128}
            simple_visit! {visit_isize, isize, ISize}

            simple_visit! {visit_u8,    u8,    U8}
            simple_visit! {visit_u16,   u16,   U16}
            simple_visit! {visit_u32,   u32,   U32}
            simple_visit! {visit_u64,   u64,   U64}
            simple_visit! {visit_u128,  u128,  U128}
            simple_visit! {visit_usize, usize, USize}

            simple_visit! {visit_f32, f32, F32}
            simple_visit! {visit_f64, f64, F64}

            fn visit_str<E: crate::de::DeserializeError>(
                self,
                v: &'de str,
            ) -> Result<Self::Value, E> {
                Ok(Item::String(v.to_owned()))
            }

            simple_visit! { visit_string, String, String }

            fn visit_sym<E: crate::de::DeserializeError>(
                self,
                v: &'de str,
            ) -> Result<Self::Value, E> {
                Ok(Item::Symbol(v.to_owned()))
            }

            simple_visit! { visit_symbol, String, Symbol }

            fn visit_bytes<E: crate::de::DeserializeError>(
                self,
                v: &'de [u8],
            ) -> Result<Self::Value, E> {
                Ok(Item::Bytes(v.to_owned()))
            }

            simple_visit! { visit_byte_buf, Vec<u8>, Bytes }

            fn visit_none<E: crate::de::DeserializeError>(self) -> Result<Self::Value, E> {
                Ok(Item::Option(None))
            }

            fn visit_some<D: crate::de::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                deserializer
                    .deserialize_any(self)
                    .map(|i| Item::Option(Some(Box::new(i))))
            }

            fn visit_dictionary<D: crate::de::DictAccess<'de>>(
                self,
                mut dict: D,
            ) -> Result<Self::Value, D::Error> {
                let mut res = Vec::new();
                while let Some(entry) = dict.next_entry()? {
                    res.push(entry);
                }
                Ok(Item::Dictionary(res))
            }

            fn visit_set<S: crate::de::SetAccess<'de>>(
                self,
                mut set: S,
            ) -> Result<Self::Value, S::Error> {
                let mut res = Vec::new();
                while let Some(element) = set.next_key()? {
                    res.push(element);
                }
                Ok(Item::Set(res))
            }

            fn visit_sequence<S: crate::de::SeqAccess<'de>>(
                self,
                mut seq: S,
            ) -> Result<Self::Value, S::Error> {
                let mut res = Vec::new();
                while let Some(element) = seq.next_value()? {
                    res.push(element);
                }
                Ok(Item::Sequence(res))
            }

            fn visit_record<R: crate::de::RecordAccess<'de>>(
                self,
                rec: R,
            ) -> Result<Self::Value, R::Error> {
                let mut fields = Vec::new();
                let (mut rec, label) = rec.label()?;
                while let Some(field) = rec.next_field()? {
                    fields.push(field);
                }
                Ok(Item::Record(label, fields))
            }
        }
        de.deserialize_any(__Visitor)
    }
}
