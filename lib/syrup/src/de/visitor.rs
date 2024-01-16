use super::{DeserializeError, DictAccess, RecordAccess, SeqAccess, SetAccess};

pub trait Visitor<'input>: Sized {
    type Value;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;

    fn visit_bool<E: DeserializeError>(self, v: bool) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_i8<E: DeserializeError>(self, v: i8) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_i16<E: DeserializeError>(self, v: i16) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_i32<E: DeserializeError>(self, v: i32) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_i64<E: DeserializeError>(self, v: i64) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_i128<E: DeserializeError>(self, v: i128) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_isize<E: DeserializeError>(self, v: isize) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_u8<E: DeserializeError>(self, v: u8) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_u16<E: DeserializeError>(self, v: u16) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_u32<E: DeserializeError>(self, v: u32) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_u64<E: DeserializeError>(self, v: u64) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_u128<E: DeserializeError>(self, v: u128) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_usize<E: DeserializeError>(self, v: usize) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_f32<E: DeserializeError>(self, v: f32) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_f64<E: DeserializeError>(self, v: f64) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_str<E: DeserializeError>(self, v: &'input str) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_string<E: DeserializeError>(self, v: String) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_sym<E: DeserializeError>(self, v: &'input str) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_symbol<E: DeserializeError>(self, v: String) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_bytes<E: DeserializeError>(self, v: &'input [u8]) -> Result<Self::Value, E> {
        todo!()
    }
    fn visit_byte_buf<E: DeserializeError>(self, v: Vec<u8>) -> Result<Self::Value, E> {
        todo!()
    }

    fn visit_dictionary<D: DictAccess<'input>>(self, dict: D) -> Result<Self::Value, D::Error> {
        todo!()
    }
    fn visit_set<S: SetAccess<'input>>(self, set: S) -> Result<Self::Value, S::Error> {
        todo!()
    }
    fn visit_sequence<S: SeqAccess<'input>>(self, seq: S) -> Result<Self::Value, S::Error> {
        todo!()
    }
    fn visit_record<R: RecordAccess<'input>>(self, rec: R) -> Result<Self::Value, R::Error> {
        todo!()
    }
}
