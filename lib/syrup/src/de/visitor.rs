use super::{DeserializeError, Deserializer, DictAccess, RecordAccess, SeqAccess, SetAccess};

pub trait Visitor<'input>: Sized {
    type Value;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;

    fn visit_bool<E: DeserializeError>(self, _: bool) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_i8<E: DeserializeError>(self, _: i8) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_i16<E: DeserializeError>(self, _: i16) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_i32<E: DeserializeError>(self, _: i32) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_i64<E: DeserializeError>(self, _: i64) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_i128<E: DeserializeError>(self, _: i128) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_isize<E: DeserializeError>(self, _: isize) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_u8<E: DeserializeError>(self, _: u8) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_u16<E: DeserializeError>(self, _: u16) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_u32<E: DeserializeError>(self, _: u32) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_u64<E: DeserializeError>(self, _: u64) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_u128<E: DeserializeError>(self, _: u128) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_usize<E: DeserializeError>(self, _: usize) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_f32<E: DeserializeError>(self, _: f32) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_f64<E: DeserializeError>(self, _: f64) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_str<E: DeserializeError>(self, _: &'input str) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_string<E: DeserializeError>(self, _: String) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_sym<E: DeserializeError>(self, _: &'input str) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_symbol<E: DeserializeError>(self, _: String) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_bytes<E: DeserializeError>(self, _: &'input [u8]) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_byte_buf<E: DeserializeError>(self, _: Vec<u8>) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_none<E: DeserializeError>(self) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_some<D: Deserializer<'input>>(self, _: D) -> Result<Self::Value, D::Error> {
        todo!()
    }

    fn visit_dictionary<D: DictAccess<'input>>(self, _: D) -> Result<Self::Value, D::Error> {
        todo!()
    }
    fn visit_set<S: SetAccess<'input>>(self, _: S) -> Result<Self::Value, S::Error> {
        todo!()
    }
    fn visit_sequence<S: SeqAccess<'input>>(self, _: S) -> Result<Self::Value, S::Error> {
        todo!()
    }
    fn visit_record<R: RecordAccess<'input>>(self, _: R) -> Result<Self::Value, R::Error> {
        todo!()
    }
}
