use super::{
    to_bytes, Serialize, SerializeDict, SerializeRecord, SerializeSeq, SerializeSet, Serializer,
};
use crate::Error;
use ibig::UBig;

pub struct ByteSerializer {
    pub bytes: Vec<u8>,
}

pub struct ByteDictSerializer<'ser> {
    ser: &'ser mut ByteSerializer,
    entries: Vec<(Vec<u8>, Vec<u8>)>,
}

impl<'ser> ByteDictSerializer<'ser> {
    fn new(ser: &'ser mut ByteSerializer, len: Option<usize>) -> Self {
        Self {
            ser,
            entries: Vec::with_capacity(len.unwrap_or(0)),
        }
    }
}

impl<'ser> SerializeDict for ByteDictSerializer<'ser> {
    type Ok = <&'ser mut ByteSerializer as Serializer>::Ok;
    type Error = <&'ser mut ByteSerializer as Serializer>::Error;

    /// Serialize entries, ensuring that they're correctly sorted.
    fn serialize_entry<Ke: Serialize + ?Sized, Va: Serialize + ?Sized>(
        &mut self,
        key: &Ke,
        value: &Va,
    ) -> Result<(), Self::Error> {
        let key = to_bytes(key)?;
        let value = to_bytes(value)?;
        self.entries.insert(
            self.entries.partition_point(|(k, _)| k < &key),
            (key, value),
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        for (k, v) in self.entries.into_iter() {
            self.ser.bytes.extend_from_slice(&k);
            self.ser.bytes.extend_from_slice(&v);
        }
        self.ser.bytes.push(b'}');
        Ok(())
    }
}

pub struct ByteSetSerializer<'ser> {
    ser: &'ser mut ByteSerializer,
    entries: Vec<Vec<u8>>,
}

impl<'ser> ByteSetSerializer<'ser> {
    fn new(ser: &'ser mut ByteSerializer, len: Option<usize>) -> Self {
        Self {
            ser,
            entries: Vec::with_capacity(len.unwrap_or(0)),
        }
    }
}

impl<'ser> SerializeSet for ByteSetSerializer<'ser> {
    type Ok = <&'ser mut ByteSerializer as Serializer>::Ok;
    type Error = <&'ser mut ByteSerializer as Serializer>::Error;

    fn serialize_element<El: Serialize + ?Sized>(&mut self, el: &El) -> Result<(), Self::Error> {
        let el = to_bytes(el)?;
        self.entries
            .insert(self.entries.partition_point(|v| v < &el), el);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        for e in self.entries.into_iter() {
            self.ser.bytes.extend_from_slice(&e);
        }
        self.ser.bytes.push(b'$');
        Ok(())
    }
}

impl<'ser> SerializeSeq for &'ser mut ByteSerializer {
    type Ok = <Self as Serializer>::Ok;
    type Error = <Self as Serializer>::Error;

    #[inline]
    fn serialize_element<El: Serialize + ?Sized>(&mut self, el: &El) -> Result<(), Self::Error> {
        el.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.bytes.push(b']');
        Ok(())
    }
}

impl<'ser> SerializeRecord for &'ser mut ByteSerializer {
    type Ok = <Self as Serializer>::Ok;
    type Error = <Self as Serializer>::Error;

    #[inline]
    fn serialize_field<Fi: Serialize + ?Sized>(&mut self, fi: &Fi) -> Result<(), Self::Error> {
        fi.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.bytes.push(b'>');
        Ok(())
    }
}

macro_rules! serialize_int {
    ($serialize_fn:ident, $Int:ty, $UInt:ty) => {
        fn $serialize_fn(self, v: $Int) -> Result<Self::Ok, Self::Error> {
            self.bytes.extend_from_slice(
                v.checked_abs()
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| {
                        (<$UInt>::try_from(<$Int>::MAX).unwrap() + <$UInt>::from(1u8)).to_string()
                    })
                    .as_bytes(),
            );
            self.bytes.push(if v < 0 { b'-' } else { b'+' });
            Ok(())
        }
    };
}

macro_rules! serialize_uint {
    ($serialize_fn:ident, $Int:ty) => {
        fn $serialize_fn(self, v: $Int) -> Result<Self::Ok, Self::Error> {
            self.bytes.extend_from_slice(v.to_string().as_bytes());
            self.bytes.push(b'+');
            Ok(())
        }
    };
}

impl<'ser> Serializer for &'ser mut ByteSerializer {
    type Ok = ();
    type Error = Error<'static>;

    type SerializeDict = ByteDictSerializer<'ser>;

    type SerializeSeq = Self;

    type SerializeRecord = Self;

    type SerializeSet = ByteSetSerializer<'ser>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.bytes.push(match v {
            true => b't',
            false => b'f',
        });
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.bytes.push(b'F');
        self.bytes.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.bytes.push(b'D');
        self.bytes.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    serialize_int!(serialize_i8, i8, u16);
    serialize_int!(serialize_i16, i16, u32);
    serialize_int!(serialize_i32, i32, u64);
    serialize_int!(serialize_i64, i64, u128);
    serialize_int!(serialize_i128, i128, UBig);
    serialize_int!(serialize_isize, isize, u128);

    serialize_uint!(serialize_u8, u8);
    serialize_uint!(serialize_u16, u16);
    serialize_uint!(serialize_u32, u32);
    serialize_uint!(serialize_u64, u64);
    serialize_uint!(serialize_u128, u128);
    serialize_uint!(serialize_usize, usize);

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.bytes.extend_from_slice(v.len().to_string().as_bytes());
        self.bytes.push(b'"');
        self.bytes.extend_from_slice(v.as_bytes());
        Ok(())
    }

    fn serialize_sym(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.bytes.extend_from_slice(v.len().to_string().as_bytes());
        self.bytes.push(b'\'');
        self.bytes.extend_from_slice(v.as_bytes());
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.bytes.extend_from_slice(v.len().to_string().as_bytes());
        self.bytes.push(b':');
        self.bytes.extend_from_slice(v);
        Ok(())
    }

    fn serialize_dictionary(self, len: Option<usize>) -> Result<Self::SerializeDict, Self::Error> {
        self.bytes.push(b'{');
        Ok(ByteDictSerializer::new(self, len))
    }

    fn serialize_sequence(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.bytes.push(b'[');
        Ok(self)
    }

    fn serialize_record(
        self,
        name: &'static str,
        _len: Option<usize>,
    ) -> Result<Self::SerializeRecord, Self::Error> {
        self.bytes.push(b'<');
        self.serialize_sym(name)?;
        Ok(self)
    }

    fn serialize_set(self, len: Option<usize>) -> Result<Self::SerializeSet, Self::Error> {
        self.bytes.push(b'#');
        Ok(ByteSetSerializer::new(self, len))
    }
}
