use super::{SerializeDict, SerializeSeq, SerializeSet};
use crate::{
    ser::{Serialize, Serializer},
    Symbol,
};
use std::collections::{HashMap, HashSet};

mod for_tuple {
    use crate::ser::{Serialize, SerializeSeq, Serializer};

    syrup_proc::impl_serialize_for_tuple!(32);
}

macro_rules! impl_serialize_simple {
    ($self:ident, $self_expr:expr, $T:ty, $serialize_fn:ident) => {
        impl Serialize for $T {
            fn serialize<Ser: Serializer>($self: &Self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
                s.$serialize_fn($self_expr)
            }
        }
    };
    ($T:ty, $serialize_fn:ident) => {
        impl_serialize_simple! {self, *self, $T, $serialize_fn}
    };
}

impl<S: Serialize> Serialize for Box<S> {
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        (**self).serialize(s)
    }
}

impl_serialize_simple!(bool, serialize_bool);
impl_serialize_simple!(self, self, str, serialize_str);
impl_serialize_simple!(self, self.as_str(), String, serialize_str);
impl_serialize_simple!(self, self.0, Symbol<&str>, serialize_sym);
impl_serialize_simple!(self, self.0.as_str(), Symbol<String>, serialize_sym);

impl_serialize_simple!(i8, serialize_i8);
impl_serialize_simple!(i16, serialize_i16);
impl_serialize_simple!(i32, serialize_i32);
impl_serialize_simple!(i64, serialize_i64);
impl_serialize_simple!(i128, serialize_i128);
impl_serialize_simple!(isize, serialize_isize);

impl_serialize_simple!(u8, serialize_u8);
impl_serialize_simple!(u16, serialize_u16);
impl_serialize_simple!(u32, serialize_u32);
impl_serialize_simple!(u64, serialize_u64);
impl_serialize_simple!(u128, serialize_u128);
impl_serialize_simple!(usize, serialize_usize);

impl Serialize for ibig::IBig {
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        i128::try_from(self).unwrap().serialize(s)
    }
}

impl Serialize for ibig::UBig {
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        u128::try_from(self).unwrap().serialize(s)
    }
}

impl_serialize_simple!(f32, serialize_f32);
impl_serialize_simple!(f64, serialize_f64);

impl<T: Serialize> Serialize for Option<T> {
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        match self {
            None => false.serialize(s),
            Some(v) => v.serialize(s),
        }
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        let mut seq = s.serialize_sequence(Some(self.len()))?;
        for e in self {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

impl<T: Serialize> Serialize for [T] {
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        let mut seq = s.serialize_sequence(Some(self.len()))?;
        for e in self {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

impl<T: Serialize, const LEN: usize> Serialize for [T; LEN] {
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        let mut seq = s.serialize_sequence(Some(self.len()))?;
        for e in self {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

impl<T: Serialize, State> Serialize for HashSet<T, State> {
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        let mut set = s.serialize_set(Some(self.len()))?;
        for e in self {
            set.serialize_element(e)?;
        }
        set.end()
    }
}

impl<Ke: Serialize, Va: Serialize, State> Serialize for HashMap<Ke, Va, State> {
    fn serialize<Ser: Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        let mut dict = s.serialize_dictionary(Some(self.len()))?;
        for (k, v) in self {
            dict.serialize_entry(k, v)?;
        }
        dict.end()
    }
}
