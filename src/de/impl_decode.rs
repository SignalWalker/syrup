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

use crate::de::{Decode, DecodeError, Dictionary, List, Literal, Record, Set, TokenTree};

#[cfg(test)]
mod test;

mod _impl_tuple {
    use crate as syrup;

    syrup_proc::impl_decode_for_tuple!(32);
}

impl<'i> Decode<'i> for () {
    fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
        match input {
            TokenTree::List(List { elements }) if elements.is_empty() => Ok(()),
            _ => Err(DecodeError::unexpected("empty list".into(), input.clone())),
        }
    }
}

// TODO :: this seems to be convention, but maybe it should be a separate function instead of the
// canonical impl for Option
impl<'i, T: Decode<'i>> Decode<'i> for Option<T> {
    fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
        match input {
            TokenTree::Literal(Literal::Bool(false)) => Ok(None),
            val => val.decode().map(Self::Some),
        }
    }
}

macro_rules! impl_decode_for_wrapper {
    ($($Wrapper:ident),+) => {
        $(
        impl<'i, T: Decode<'i>> Decode<'i> for $Wrapper<T> {
            fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
                T::decode(input).map($Wrapper::new)
            }
        }
        )+
    };
}

impl_decode_for_wrapper!(Box, Rc, Arc, Cell, RefCell, Mutex);

impl<'i, T> Decode<'i> for Box<[T]>
where
    Vec<T>: Decode<'i>,
{
    fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
        Vec::<T>::decode(input).map(From::from)
    }
}

macro_rules! impl_decode_to_tokens {
    ($Id:ident, $expected:literal => $Ty:ty) => {
        impl<'i> Decode<'i> for $Ty {
            fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
                match input {
                    TokenTree::$Id(val) => Ok(<$Ty>::clone(val)),
                    _ => Err(DecodeError::unexpected(
                        Cow::Borrowed($expected),
                        input.clone(),
                    )),
                }
            }
        }

        impl<'i> Decode<'i> for &'i $Ty {
            fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
                match input {
                    TokenTree::$Id(val) => Ok(val),
                    _ => Err(DecodeError::unexpected(
                        Cow::Borrowed($expected),
                        input.clone(),
                    )),
                }
            }
        }
    };
}
impl_decode_to_tokens! {List, "List" => List}
impl_decode_to_tokens! {Dictionary, "Dictionary" => Dictionary}
impl_decode_to_tokens! {Set, "Set" => Set}
impl_decode_to_tokens! {Record, "Record" => Record}
impl_decode_to_tokens! {Literal, "Literal" => Literal}

#[allow(edition_2024_expr_fragment_specifier)]
macro_rules! impl_parse_simple {
    ($Ty:ty, $expected:expr, $Lit:ident) => {
        impl<'i> Decode<'i> for $Ty {
            fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
                match input {
                    TokenTree::Literal(Literal::$Lit(val)) => Ok(*val),
                    _ => Err(DecodeError::unexpected(
                        Cow::Borrowed($expected),
                        input.clone(),
                    )),
                }
            }
        }
    };
}

impl_parse_simple! {bool, "bool", Bool}
impl_parse_simple! {f32, "f32", F32}
impl_parse_simple! {f64, "f64", F64}

#[allow(edition_2024_expr_fragment_specifier)]
macro_rules! impl_parse_stringlike {
    ($String:ty, $expected:expr, $Lit:ident) => {
        impl<'i> Decode<'i> for $String {
            fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
                match input {
                    TokenTree::Literal(Literal::$Lit(val)) => match std::str::from_utf8(val) {
                        Ok(res) => Ok(res.into()),
                        Err(e) => Err(DecodeError::utf8(Cow::Owned(val.clone()), e)),
                    },
                    _ => Err(DecodeError::unexpected(
                        Cow::Borrowed($expected),
                        input.clone(),
                    )),
                }
            }
        }
    };
}
impl_parse_stringlike! {String, "String", String}
// impl_parse_stringlike! {Symbol<'i>, "Symbol", Symbol}

// impl<'i, const LEN: usize> Decode<'i> for ByteArray<'i, LEN> {
//     fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
//         // TODO :: surely there's a way to make the expected str const
//         match input {
//             TokenTree::Literal(Literal {
//                 repr: LiteralValue::Bytes(ref bytes),
//                 ..
//             }) => match bytes {
//                 Cow::Borrowed(b) => ByteArray::try_from(*b),
//                 // TODO :: avoid `b.clone()` here
//                 Cow::Owned(b) => ByteArray::try_from(b.clone()),
//             }
//             .map_err(|_error| input.to_unexpected(format!("{LEN} Bytes").into())),
//             tree => Err(tree.to_unexpected(format!("{LEN} Bytes").into())),
//         }
//     }
// }
//
// impl<'i> Decode<'i> for String {
//     fn decode(input: TokenTree<'i>) -> Result<Self, DecodeError<'i>> {
//         Cow::<'i, str>::decode(input).map(Cow::into_owned)
//     }
// }
//
macro_rules! impl_decode_for_int {
    ($($Int:ty),+$(,)?) => {
        $(
        impl<'i> Decode<'i> for $Int {
            fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
                match input {
                    TokenTree::Literal(Literal::Int(int)) => int.try_into().map_err(|source| DecodeError::int(int.clone().into_static(), source)),
                    _ => Err(DecodeError::unexpected(
                        Cow::Borrowed(::std::stringify!($Int)),
                        input.clone()
                    )),
                }
            }
        }
        )+
    };
}

impl_decode_for_int!(
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

impl<'i, T: Decode<'i> + Into<Wrapping<T>>> Decode<'i> for Wrapping<T> {
    fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
        T::decode(input).map(T::into)
    }
}

impl<'i, T: Decode<'i> + Into<Saturating<T>>> Decode<'i> for Saturating<T> {
    fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
        T::decode(input).map(T::into)
    }
}

#[cfg(feature = "decode-array")]
impl<'i, T: Decode<'i>, const LEN: usize> Decode<'i> for [T; LEN] {
    fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
        use std::mem::MaybeUninit;
        fn initialize_array<'i, 'e, T: Decode<'i>, const LEN: usize>(
            init_amt: &mut usize,
            array: &mut [MaybeUninit<T>; LEN],
            tokens: &'i [TokenTree],
        ) -> Result<(), DecodeError<'e>> {
            for (i, slot) in array.iter_mut().enumerate() {
                let element = match tokens.get(i) {
                    Some(el) => T::decode(el)?,
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
            TokenTree::List(List { elements }) => {
                #[allow(unsafe_code)]
                unsafe {
                    let mut init_amt = 0;
                    let mut res = [const { MaybeUninit::<T>::uninit() }; LEN];
                    if let Err(error) =
                        initialize_array::<T, LEN>(&mut init_amt, &mut res, elements)
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
            _ => Err(DecodeError::unexpected(
                Cow::Owned(format!("list with {LEN} elements")),
                input.clone(),
            )),
        }
    }
}

impl<'i, T: Decode<'i>> Decode<'i> for Vec<T> {
    fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
        match input {
            TokenTree::List(List { elements }) => {
                let mut res = Vec::with_capacity(elements.len());
                for token in elements {
                    res.push(T::decode(token)?);
                }
                Ok(res)
            }
            _ => Err(DecodeError::unexpected(
                Cow::Borrowed("list"),
                input.clone(),
            )),
        }
    }
}

impl<'i, T, S> Decode<'i> for HashSet<T, S>
where
    T: Decode<'i> + std::hash::Hash + Eq,
    S: Default + std::hash::BuildHasher,
{
    fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
        match input {
            TokenTree::Set(set) => {
                let mut res = HashSet::<T, S>::default();
                for element in set {
                    res.insert(element.decode()?);
                }
                Ok(res)
            }
            _ => Err(DecodeError::unexpected(Cow::Borrowed("set"), input.clone())),
        }
    }
}

impl<'i, T> Decode<'i> for BTreeSet<T>
where
    T: Decode<'i> + std::cmp::Ord,
{
    fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
        match input {
            TokenTree::Set(set) => {
                let mut res = Self::default();
                for element in set {
                    res.insert(element.decode()?);
                }
                Ok(res)
            }
            _ => Err(DecodeError::unexpected(Cow::Borrowed("set"), input.clone())),
        }
    }
}

impl<'i, K, V, S> Decode<'i> for HashMap<K, V, S>
where
    K: Decode<'i> + std::hash::Hash + Eq,
    V: Decode<'i>,
    S: Default + std::hash::BuildHasher,
{
    fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
        match input {
            TokenTree::Dictionary(dict) => {
                let mut res = Self::default();
                for (k, v) in dict {
                    res.insert(K::decode(k)?, V::decode(v)?);
                }
                Ok(res)
            }
            _ => Err(DecodeError::unexpected(
                Cow::Borrowed("dictionary"),
                input.clone(),
            )),
        }
    }
}

impl<'i, K, V> Decode<'i> for BTreeMap<K, V>
where
    K: Decode<'i> + std::cmp::Ord,
    V: Decode<'i>,
{
    fn decode<'e>(input: &'i TokenTree) -> Result<Self, DecodeError<'e>> {
        match input {
            TokenTree::Dictionary(dict) => {
                let mut res = Self::default();
                for (k, v) in dict {
                    res.insert(K::decode(k)?, V::decode(v)?);
                }
                Ok(res)
            }
            _ => Err(DecodeError::unexpected(
                Cow::Borrowed("dictionary"),
                input.clone(),
            )),
        }
    }
}
