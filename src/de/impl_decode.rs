use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize, Saturating, Wrapping,
    },
    rc::Rc,
    sync::{Arc, Mutex},
};

use crate::{
    de::{
        ByteArray, Bytes, Decode, DecodeError, Dictionary, Literal, LiteralValue, Sequence, Set,
        Symbol, TokenTree,
    },
    Record,
};

mod _impl_tuple {
    use crate as syrup;

    syrup_proc::impl_decode_for_tuple!(32);
}

macro_rules! impl_parse_simple {
    ($Ty:ty, $expected:expr, $Lit:ident) => {
        impl<'i> Decode<'i> for $Ty {
            //const TYPE_STR: &'static str = $expected;
            fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
                match input {
                    TokenTree::Literal(Literal {
                        repr: LiteralValue::$Lit(val),
                        ..
                    }) => Ok(val),
                    tree => Err(DecodeError::unexpected($expected.into(), tree)),
                }
            }
        }
    };
}

impl<'i> Decode<'i> for () {
    fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
        match input {
            TokenTree::Sequence(Sequence { mut stream, .. }) => match stream.pop() {
                Some(t) => Err(t.to_unexpected("Nothing".into())),
                None => Ok(()),
            },
            tree => Err(tree.to_unexpected("Empty Sequence".into())),
        }
    }
}

impl<'i, T: Decode<'i>> Decode<'i> for Option<T> {
    fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
        match input {
            TokenTree::Literal(Literal {
                repr: LiteralValue::Bool(false),
                ..
            }) => Ok(None),
            val => val.decode().map(Self::Some),
        }
    }
}

macro_rules! impl_decode_to_tokens {
    ($Id:ident, $expected:literal => $Ty:ty) => {
        impl<'i> Decode<'i> for $Ty {
            fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
                match input {
                    TokenTree::$Id(val) => Ok(val),
                    tree => Err(tree.to_unexpected(Cow::Borrowed($expected))),
                }
            }
        }
    };
}

impl_decode_to_tokens! {Sequence, "Sequence" => Sequence<'i>}
impl_decode_to_tokens! {Dictionary, "Dictionary" => Dictionary<'i>}
impl_decode_to_tokens! {Set, "Set" => Set<'i>}
impl_decode_to_tokens! {Record, "Record" => Record<'i>}
impl_decode_to_tokens! {Literal, "Literal" => Literal<'i>}

impl_parse_simple! {bool, "bool", Bool}
impl_parse_simple! {f32, "f32", F32}
impl_parse_simple! {f64, "f64", F64}

impl<'i> Decode<'i> for Bytes<'i> {
    fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
        match input {
            TokenTree::Literal(Literal {
                repr: LiteralValue::Bytes(b),
                ..
            }) => Ok(Bytes(b)),
            tree => Err(tree.to_unexpected("Bytes".into())),
        }
    }
}

impl<'i> Decode<'i> for Cow<'i, [u8]> {
    fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
        match input {
            TokenTree::Literal(Literal {
                repr: LiteralValue::Bytes(b),
                ..
            }) => Ok(b),
            tree => Err(tree.to_unexpected("Bytes".into())),
        }
    }
}

macro_rules! impl_parse_stringlike {
    ($String:ty, $expected:expr, $Lit:ident) => {
        impl<'i> Decode<'i> for $String {
            fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
                match input {
                    TokenTree::Literal(Literal {
                        repr: LiteralValue::$Lit(val),
                        ..
                    }) => match val {
                        Cow::Borrowed(b) => match std::str::from_utf8(b) {
                            Ok(b) => Ok(b.into()),
                            Err(e) => Err(DecodeError::utf8(val, e)),
                        },
                        Cow::Owned(b) => match std::str::from_utf8(&b) {
                            #[allow(unsafe_code)]
                            // reason = we already know it's valid utf8 because of the match expr
                            Ok(_) => Ok(unsafe { String::from_utf8_unchecked(b) }.into()),
                            Err(e) => Err(DecodeError::utf8(b.into(), e)),
                        },
                    },
                    tree => Err(tree.to_unexpected($expected.into())),
                }
            }
        }
    };
}

impl_parse_stringlike! {Cow<'i, str>, "String", String}
impl_parse_stringlike! {Symbol<'i>, "Symbol", Symbol}

impl<'i, const LEN: usize> Decode<'i> for ByteArray<'i, LEN> {
    fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
        // TODO :: surely there's a way to make the expected str const
        match input {
            TokenTree::Literal(Literal {
                repr: LiteralValue::Bytes(ref bytes),
                ..
            }) => match bytes {
                Cow::Borrowed(b) => ByteArray::try_from(*b),
                // TODO :: avoid `b.clone()` here
                Cow::Owned(b) => ByteArray::try_from(b.clone()),
            }
            .map_err(|_error| input.to_unexpected(format!("{LEN} Bytes").into())),
            tree => Err(tree.to_unexpected(format!("{LEN} Bytes").into())),
        }
    }
}

impl<'i> Decode<'i> for String {
    fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
        Cow::<'i, str>::decode(input).map(Cow::into_owned)
    }
}

macro_rules! impl_parse_for_int {
    ($($Int:ty),+$(,)?) => {
        $(
        impl<'i> Decode<'i> for $Int {
            fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
                match input {
                    TokenTree::Literal(Literal {
                        repr: LiteralValue::Int(int),
                        ..
                    }) => (&int).try_into().map_err(|source| DecodeError::int(int, source)),
                    tree => Err(tree.to_unexpected(
                        Cow::Borrowed(::std::stringify!($Int))
                    )),
                }
            }
        }
        )+
    };
}

impl_parse_for_int!(
    u8,
    u16,
    u32,
    u64,
    usize,
    u128,
    i8,
    i16,
    i32,
    i64,
    isize,
    i128,
    NonZeroU8,
    NonZeroU16,
    NonZeroU32,
    NonZeroU64,
    NonZeroUsize,
    NonZeroU128,
    NonZeroI8,
    NonZeroI16,
    NonZeroI32,
    NonZeroI64,
    NonZeroIsize,
    NonZeroI128
);

#[cfg(feature = "decode-array")]
impl<'i, T: Decode<'i>, const LEN: usize> Decode<'i> for [T; LEN] {
    fn needed() -> nom::Needed {
        use nom::Needed;
        let Some(len) = NonZeroUsize::new(LEN) else {
            #[allow(unsafe_code)] // reason = 2 != 0
            return Needed::Size(unsafe { NonZeroUsize::new_unchecked(2) });
        };
        let Needed::Size(t_amt) = T::needed() else {
            return Needed::Unknown;
        };

        Needed::Size(t_amt.saturating_mul(len).saturating_add(2))
    }
    fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
        use std::mem::MaybeUninit;
        fn initialize_array<'i, T: Decode<'i>, const LEN: usize>(
            init_amt: &mut usize,
            array: &mut [MaybeUninit<T>; LEN],
            mut stream: crate::TokenStream<'i>,
        ) -> Result<(), DecodeError<'i>> {
            for (i, slot) in array.iter_mut().enumerate() {
                let element = match stream.pop() {
                    Some(el) => el.decode()?,
                    None => {
                        return Err(DecodeError::missing(Cow::Owned(format!(
                            "{i}th array element"
                        ))))
                    }
                };
                slot.write(element);
                *init_amt += 1;
            }
            Ok(())
        }
        match input {
            TokenTree::Sequence(Sequence { stream, .. }) => {
                #[allow(unsafe_code)]
                unsafe {
                    let mut init_amt = 0;
                    let mut res = MaybeUninit::uninit_array::<LEN>();
                    if let Err(error) = initialize_array::<T, LEN>(&mut init_amt, &mut res, stream)
                    {
                        for val in &mut res[..init_amt] {
                            val.assume_init_drop();
                        }
                        return Err(error);
                    }
                    #[allow(unsafe_code)]
                    Ok(MaybeUninit::array_assume_init(res))
                }
            }
            tree => Err(tree.to_unexpected(Cow::Owned(format!("Sequence with {LEN} elements")))),
        }
    }
}

impl<'i, T: Decode<'i>> Decode<'i> for Vec<T> {
    fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
        match input {
            TokenTree::Sequence(Sequence { stream, .. }) => {
                let mut res = Vec::with_capacity(stream.len());
                for token in stream {
                    res.push(token.decode()?);
                }
                Ok(res)
            }
            tree => Err(tree.to_unexpected("Sequence".into())),
        }
    }
}

impl<'i, T, S> Decode<'i> for HashSet<T, S>
where
    T: Decode<'i> + std::hash::Hash + Eq,
    S: Default + std::hash::BuildHasher,
{
    fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
        match input {
            TokenTree::Set(Set { stream, .. }) => {
                let mut res = HashSet::<T, S>::default();
                for element in stream {
                    res.insert(element.decode()?);
                }
                Ok(res)
            }
            tree => Err(tree.to_unexpected("Set".into())),
        }
    }
}

impl<'i, T> Decode<'i> for BTreeSet<T>
where
    T: Decode<'i> + std::cmp::Ord,
{
    fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
        match input {
            TokenTree::Set(Set { stream, .. }) => {
                let mut res = Self::default();
                for element in stream {
                    res.insert(element.decode()?);
                }
                Ok(res)
            }
            tree => Err(tree.to_unexpected("Set".into())),
        }
    }
}

impl<'i, K, V, S> Decode<'i> for HashMap<K, V, S>
where
    K: Decode<'i> + std::hash::Hash + Eq,
    V: Decode<'i>,
    S: Default + std::hash::BuildHasher,
{
    fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
        match input {
            TokenTree::Dictionary(Dictionary { mut stream, .. }) => {
                let mut res = HashMap::<K, V, S>::default();
                loop {
                    let Some(key) = stream.pop() else { break };
                    let key = key.decode::<K>()?;
                    res.insert(key, stream.require(Cow::Borrowed("entry value"))?.decode()?);
                }
                Ok(res)
            }
            tree => Err(tree.to_unexpected(Cow::Borrowed("Dictionary"))),
        }
    }
}

impl<'i, K, V> Decode<'i> for BTreeMap<K, V>
where
    K: Decode<'i> + std::cmp::Ord,
    V: Decode<'i>,
{
    fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
        match input {
            TokenTree::Dictionary(Dictionary { mut stream, .. }) => {
                let mut res = Self::default();
                loop {
                    let Some(key) = stream.pop() else { break };
                    let key = key.decode::<K>()?;
                    res.insert(
                        key,
                        match stream.pop() {
                            Some(value) => value.decode()?,
                            None => return Err(DecodeError::missing(Cow::Borrowed("entry value"))),
                        },
                    );
                }
                Ok(res)
            }
            tree => Err(tree.to_unexpected(Cow::Borrowed("Dictionary"))),
        }
    }
}

macro_rules! impl_decode_for_wrapper {
    ($($Wrapper:ident),+) => {
        $(
        impl<'i, T: Decode<'i> + Into<$Wrapper<T>>> Decode<'i> for $Wrapper<T> {
            fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
                T::decode(input).map(Into::into)
            }
        }
        )+
    };
}

impl_decode_for_wrapper!(Box, Rc, Arc, Cell, RefCell, Wrapping, Saturating, Mutex);

impl<'i, T: Decode<'i>> Decode<'i> for Box<[T]> {
    fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
        Vec::<T>::decode(input).map(From::from)
    }
}
