pub use syrup_derive::{Deserialize, Serialize};

pub mod de;
pub mod ser;

pub use de::Deserialize;
pub use ser::Serialize;

#[cfg(feature = "serde")]
pub mod serde;

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

#[derive(Debug, PartialEq)]
pub enum ParseErrorKind {
    Nom(nom::error::ErrorKind),
    OutOfBounds,
}

impl From<nom::error::ErrorKind> for ParseErrorKind {
    fn from(value: nom::error::ErrorKind) -> Self {
        Self::Nom(value)
    }
}

#[derive(Debug, PartialEq)]
pub enum ErrorKind {
    Parse(ParseErrorKind),
    Incomplete(nom::Needed),
}

impl From<nom::error::ErrorKind> for ErrorKind {
    fn from(value: nom::error::ErrorKind) -> Self {
        Self::Parse(value.into())
    }
}

#[derive(thiserror::Error, PartialEq)]
pub struct Error<'input> {
    input: Option<&'input [u8]>,
    kind: ErrorKind,
    // #[backtrace]
    // backtrace: std::backtrace::Backtrace,
}

impl<'input> std::fmt::Display for Error<'input> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<'input> std::fmt::Debug for Error<'input> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = f.debug_struct("Error");
        match self.input.map(std::str::from_utf8) {
            Some(Ok(s)) => res.field("input", &s),
            _ => res.field("input", &self.input),
        }
        .field("kind", &self.kind)
        .finish()
    }
}

impl<'input> nom::error::ParseError<&'input [u8]> for Error<'input> {
    fn from_error_kind(input: &'input [u8], kind: nom::error::ErrorKind) -> Self {
        Self {
            input: Some(input),
            kind: kind.into(),
        }
    }

    fn append(input: &'input [u8], kind: nom::error::ErrorKind, other: Self) -> Self {
        todo!()
    }
}

impl<'input> From<nom::Err<Error<'input>>> for Error<'input> {
    fn from(value: nom::Err<Error<'input>>) -> Self {
        match value {
            nom::Err::Incomplete(_) => todo!(),
            nom::Err::Error(e) => e,
            nom::Err::Failure(e) => e,
        }
    }
}

impl<'input> nom::error::FromExternalError<&'input [u8], ibig::error::OutOfBoundsError>
    for Error<'input>
{
    fn from_external_error(
        input: &'input [u8],
        kind: nom::error::ErrorKind,
        _e: ibig::error::OutOfBoundsError,
    ) -> Self {
        Self {
            input: Some(input),
            kind: kind.into(),
        }
    }
}

impl<'input> de::DeserializeError for Error<'input> {}

#[cfg(test)]
pub(crate) mod test {
    #[macro_export]
    macro_rules! test_deserialize {
        ($n:ident; $($arg:ident in $generator:expr),+ => $input:ident, $mk_input:expr => $f:expr => $exp_res:expr) => {
            proptest::proptest! {
                #[test]
                fn $n($($arg in $generator),+) {
                    let $input = $mk_input;
                    assert_eq!($f, Ok($exp_res))
                }
            }
        };
        ($n:ident, $f:expr; $($arg:ident in $generator:expr),+ => $mk_input:expr => $exp_res:expr) => {
            test_deserialize!{$n; $($arg in $generator),+ => __input, $mk_input => $f(&__input) => $exp_res}
        };
        ($f:ident; $($arg:ident in $generator:expr),+ => $mk_input:expr => $exp_left:expr; $exp_res:expr) => {
            test_deserialize!($f, $crate::de::parse::$f::<nom::error::VerboseError<_>>; $($arg in $generator),+ => $mk_input => $exp_left; $exp_res)
        };
    }
}

pub mod optional_map {
    use std::{collections::HashMap, marker::PhantomData};

    use super::{
        de::{Deserialize, DeserializeError, Deserializer, DictAccess, Visitor},
        ser::{Serialize, SerializeDict, Serializer},
    };

    pub fn serialize<S: Serializer, K: Serialize, V: Serialize, State>(
        m: &HashMap<K, V, State>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        if m.is_empty() {
            ser.serialize_bool(false)
        } else {
            let mut map = ser.serialize_dictionary(Some(m.len()))?;
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

        fn visit_bool<E: DeserializeError>(self, v: bool) -> Result<Self::Value, E> {
            match v {
                true => todo!(),
                // true => Err(serde::de::Error::invalid_type(
                //     serde::de::Unexpected::Bool(v),
                //     &self,
                // )),
                false => Ok(HashMap::<K, V, State>::default()),
            }
        }

        fn visit_dictionary<A>(self, mut access: A) -> Result<Self::Value, A::Error>
        where
            A: DictAccess<'de>,
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
        D: Deserializer<'de>,
        K: Deserialize<'de> + Eq + PartialEq + std::hash::Hash,
        V: Deserialize<'de>,
        S: Default + std::hash::BuildHasher,
    >(
        de: D,
    ) -> Result<HashMap<K, V, S>, D::Error> {
        de.deserialize_any(OptionalMapVisitor::new())
    }
}
