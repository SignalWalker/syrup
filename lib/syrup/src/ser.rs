mod impl_serialize;

mod byte_serializer;
pub use byte_serializer::*;

pub trait SerializeError {}

pub trait SerializeDict {
    type Ok;
    type Error;

    fn serialize_entry<Ke: Serialize + ?Sized, Va: Serialize + ?Sized>(
        &mut self,
        key: &Ke,
        value: &Va,
    ) -> Result<(), Self::Error>;
    fn end(self) -> Result<Self::Ok, Self::Error>;
}

pub trait SerializeSeq {
    type Ok;
    type Error;

    fn serialize_element<El: Serialize + ?Sized>(&mut self, el: &El) -> Result<(), Self::Error>;
    fn end(self) -> Result<Self::Ok, Self::Error>;
}

pub trait SerializeRecord {
    type Ok;
    type Error;

    fn serialize_field<Fi: Serialize + ?Sized>(&mut self, fi: &Fi) -> Result<(), Self::Error>;
    fn end(self) -> Result<Self::Ok, Self::Error>;
}

pub trait SerializeSet {
    type Ok;
    type Error;

    fn serialize_element<El: Serialize + ?Sized>(&mut self, el: &El) -> Result<(), Self::Error>;
    fn end(self) -> Result<Self::Ok, Self::Error>;
}

macro_rules! serialize_simple {
    ($fn:ident, $T:ty) => {
        fn $fn(self, v: $T) -> Result<Self::Ok, Self::Error>;
    };
}

pub trait Serializer {
    type Ok;
    type Error;

    type SerializeDict: SerializeDict<Ok = Self::Ok, Error = Self::Error>;
    type SerializeSeq: SerializeSeq<Ok = Self::Ok, Error = Self::Error>;
    type SerializeRecord: SerializeRecord<Ok = Self::Ok, Error = Self::Error>;
    type SerializeSet: SerializeSet<Ok = Self::Ok, Error = Self::Error>;

    serialize_simple!(serialize_bool, bool);

    serialize_simple!(serialize_f32, f32);
    serialize_simple!(serialize_f64, f64);

    serialize_simple!(serialize_i8, i8);
    serialize_simple!(serialize_i16, i16);
    serialize_simple!(serialize_i32, i32);
    serialize_simple!(serialize_i64, i64);
    serialize_simple!(serialize_i128, i128);
    serialize_simple!(serialize_isize, isize);

    serialize_simple!(serialize_u8, u8);
    serialize_simple!(serialize_u16, u16);
    serialize_simple!(serialize_u32, u32);
    serialize_simple!(serialize_u64, u64);
    serialize_simple!(serialize_u128, u128);
    serialize_simple!(serialize_usize, usize);

    serialize_simple!(serialize_str, &str);
    serialize_simple!(serialize_sym, &str);
    serialize_simple!(serialize_bytes, &[u8]);

    fn serialize_dictionary(self, len: Option<usize>) -> Result<Self::SerializeDict, Self::Error>;
    fn serialize_sequence(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error>;
    fn serialize_record(
        self,
        name: &'static str,
        len: Option<usize>,
    ) -> Result<Self::SerializeRecord, Self::Error>;
    fn serialize_set(self, len: Option<usize>) -> Result<Self::SerializeSet, Self::Error>;
}

pub trait Serialize {
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error>;
}

pub fn to_bytes<T: Serialize + ?Sized>(
    val: &T,
) -> Result<Vec<u8>, <&mut ByteSerializer as Serializer>::Error> {
    let mut ser = ByteSerializer { bytes: vec![] };
    val.serialize(&mut ser)?;
    Ok(ser.bytes)
}
