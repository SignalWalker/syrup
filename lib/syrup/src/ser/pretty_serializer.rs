use crate::{
    de::from_bytes,
    ser::{SerializeDict, SerializeRecord, SerializeSeq, SerializeSet, Serializer},
    Error, Serialize,
};
use ibig::UBig;

pub struct PrettySerializer {
    pub res: String,
}

pub struct PrettyDictSerializer<'ser> {
    ser: &'ser mut PrettySerializer,
    entries: Vec<(String, String)>,
}

impl<'ser> PrettyDictSerializer<'ser> {
    fn new(ser: &'ser mut PrettySerializer, len: Option<usize>) -> Self {
        Self {
            ser,
            entries: Vec::with_capacity(len.unwrap_or(0)),
        }
    }
}

impl<'ser> SerializeDict for PrettyDictSerializer<'ser> {
    type Ok = <&'ser mut PrettySerializer as Serializer>::Ok;
    type Error = <&'ser mut PrettySerializer as Serializer>::Error;

    /// Serialize entries, ensuring that they're correctly sorted.
    fn serialize_entry<Ke: Serialize + ?Sized, Va: Serialize + ?Sized>(
        &mut self,
        key: &Ke,
        value: &Va,
    ) -> Result<(), Self::Error> {
        let key = to_pretty(key)?;
        let value = to_pretty(value)?;
        self.entries.insert(
            self.entries.partition_point(|(k, _)| k < &key),
            (key, value),
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        for (i, (k, v)) in self.entries.iter().enumerate() {
            self.ser.res.push_str(k);
            self.ser.res.push_str(": ");
            self.ser.res.push_str(v);
            if i < self.entries.len() - 1 {
                self.ser.res.push_str(", ");
            } else {
                self.ser.res.push(' ');
            }
        }
        self.ser.res.push('}');
        Ok(())
    }
}

pub struct PrettySetSerializer<'ser> {
    ser: &'ser mut PrettySerializer,
    entries: Vec<String>,
}

impl<'ser> PrettySetSerializer<'ser> {
    fn new(ser: &'ser mut PrettySerializer, len: Option<usize>) -> Self {
        Self {
            ser,
            entries: Vec::with_capacity(len.unwrap_or(0)),
        }
    }
}

impl<'ser> SerializeSet for PrettySetSerializer<'ser> {
    type Ok = <&'ser mut PrettySerializer as Serializer>::Ok;
    type Error = <&'ser mut PrettySerializer as Serializer>::Error;

    fn serialize_element<El: Serialize + ?Sized>(&mut self, el: &El) -> Result<(), Self::Error> {
        let el = to_pretty(el)?;
        self.entries
            .insert(self.entries.partition_point(|v| v < &el), el);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        for (i, e) in self.entries.iter().enumerate() {
            self.ser.res.push_str(e);
            if i < self.entries.len() - 1 {
                self.ser.res.push_str(", ");
            } else {
                self.ser.res.push(' ');
            }
        }
        self.ser.res.push(')');
        Ok(())
    }
}

impl<'ser> SerializeSeq for &'ser mut PrettySerializer {
    type Ok = <Self as Serializer>::Ok;
    type Error = <Self as Serializer>::Error;

    #[inline]
    fn serialize_element<El: Serialize + ?Sized>(&mut self, el: &El) -> Result<(), Self::Error> {
        self.res.push(' ');
        el.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.res.push_str(" ]");
        Ok(())
    }
}

impl<'ser> SerializeRecord for &'ser mut PrettySerializer {
    type Ok = <Self as Serializer>::Ok;
    type Error = <Self as Serializer>::Error;

    #[inline]
    fn serialize_field<Fi: Serialize + ?Sized>(&mut self, fi: &Fi) -> Result<(), Self::Error> {
        self.res.push(' ');
        fi.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.res.push('>');
        Ok(())
    }
}

macro_rules! serialize_int {
    ($serialize_fn:ident, $Int:ty, $UInt:ty) => {
        fn $serialize_fn(self, v: $Int) -> Result<Self::Ok, Self::Error> {
            self.res.push_str(&v.to_string());
            Ok(())
        }
    };
}

macro_rules! serialize_uint {
    ($serialize_fn:ident, $Int:ty) => {
        fn $serialize_fn(self, v: $Int) -> Result<Self::Ok, Self::Error> {
            self.res.push_str(&v.to_string());
            Ok(())
        }
    };
}

impl<'ser> Serializer for &'ser mut PrettySerializer {
    type Ok = ();
    type Error = Error<'static>;

    type SerializeDict = PrettyDictSerializer<'ser>;

    type SerializeSeq = Self;

    type SerializeRecord = Self;

    type SerializeSet = PrettySetSerializer<'ser>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.res.push_str(match v {
            true => "true",
            false => "false",
        });
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.res.push_str(&v.to_string());
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.res.push_str(&v.to_string());
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
        self.res.push('"');
        self.res.push_str(v);
        self.res.push('"');
        Ok(())
    }

    fn serialize_sym(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.res.push_str(v);
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.res.push('#');
        self.res.push_str(&String::from_utf8_lossy(v));
        self.res.push('#');
        Ok(())
    }

    fn serialize_dictionary(self, len: Option<usize>) -> Result<Self::SerializeDict, Self::Error> {
        self.res.push_str("{ ");
        Ok(PrettyDictSerializer::new(self, len))
    }

    fn serialize_sequence(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.res.push_str("[");
        Ok(self)
    }

    fn serialize_record(
        self,
        name: &str,
        _len: Option<usize>,
    ) -> Result<Self::SerializeRecord, Self::Error> {
        self.res.push('<');
        self.serialize_sym(name)?;
        Ok(self)
    }

    fn serialize_set(self, len: Option<usize>) -> Result<Self::SerializeSet, Self::Error> {
        self.res.push_str("( ");
        Ok(PrettySetSerializer::new(self, len))
    }

    unsafe fn serialize_raw(self, data: &[u8]) -> Result<Self::Ok, Self::Error> {
        match from_bytes::<crate::Item>(data) {
            Ok(item) => self.res.push_str(&to_pretty(&item)?),
            Err(_) => self.res.push_str(&String::from_utf8_lossy(data)),
        }
        Ok(())
    }
}

pub fn to_pretty<T: Serialize + ?Sized>(
    val: &T,
) -> Result<String, <&mut PrettySerializer as Serializer>::Error> {
    let mut ser = PrettySerializer { res: String::new() };
    val.serialize(&mut ser)?;
    Ok(ser.res)
}
