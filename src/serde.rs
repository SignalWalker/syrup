use super::Error;

pub mod de;
pub mod ser;

impl<'input> serde::ser::Error for Error<'input> {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        todo!()
    }
}

impl<'input> serde::de::Error for Error<'input> {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        todo!()
    }
}

pub mod symbol {
    pub fn serialize<S: serde::Serializer>(s: impl AsRef<str>, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(s.as_ref())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(de: D) -> Result<String, D::Error> {
        todo!()
    }
}

pub mod optional_map {
    use serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize};
    use std::{collections::HashMap, marker::PhantomData};

    pub fn serialize<S: serde::Serializer, K: Serialize, V: Serialize, State>(
        m: &HashMap<K, V, State>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        if m.is_empty() {
            ser.serialize_bool(false)
        } else {
            let mut map = ser.serialize_map(Some(m.len()))?;
            for (k, v) in m {
                map.serialize_entry(k, v)?;
            }
            map.end()
        }
    }

    #[derive(Clone, Copy)]
    struct OptionalMapVisitor<K, V, State: Default> {
        _k: PhantomData<K>,
        _v: PhantomData<V>,
        _state: PhantomData<State>,
    }

    impl<K, V, State: Default> OptionalMapVisitor<K, V, State> {
        fn new() -> Self {
            Self {
                _k: PhantomData,
                _v: PhantomData,
                _state: PhantomData,
            }
        }
    }

    impl<
            'de,
            K: Deserialize<'de> + Eq + PartialEq + std::hash::Hash,
            V: Deserialize<'de>,
            State: Default + std::hash::BuildHasher,
        > Visitor<'de> for OptionalMapVisitor<K, V, State>
    {
        type Value = HashMap<K, V, State>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("false or map")
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match v {
                true => Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Bool(v),
                    &self,
                )),
                false => Ok(HashMap::<K, V, State>::default()),
            }
        }

        fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut map = HashMap::<K, V, State>::with_capacity_and_hasher(
                access.size_hint().unwrap_or(0),
                State::default(),
            );
            while let Some((k, v)) = access.next_entry()? {
                map.insert(k, v);
            }
            Ok(map)
        }
    }

    pub fn deserialize<
        'de,
        D: serde::Deserializer<'de>,
        K: Deserialize<'de> + Eq + PartialEq + std::hash::Hash,
        V: Deserialize<'de>,
        S: Default + std::hash::BuildHasher,
    >(
        de: D,
    ) -> Result<HashMap<K, V, S>, D::Error> {
        de.deserialize_any(OptionalMapVisitor::new())
    }
}
