use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    num::{
        NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize, NonZeroU8,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize, Saturating, Wrapping,
    },
    rc::Rc,
    sync::{Arc, Mutex},
};

use borrow_or_share::{BorrowOrShare, Bos};

use crate::de::{
    Decode, DecodeError, Dictionary, List, Literal, Record, Set, SyrupKind, TokenTree,
};

#[cfg(test)]
mod test;

mod _impl_tuple {
    use crate as syrup;

    syrup_proc::impl_decode_for_tuple!(32);
}

impl<'i, IData> Decode<'i, IData> for ()
where
    IData: Bos<[u8]>,
{
    fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
        match input {
            TokenTree::List(List { elements }) if elements.is_empty() => Ok(()),
            _ => Err(DecodeError::unexpected(
                SyrupKind::List { length: Some(0) },
                input,
            )),
        }
    }
}

// TODO :: this seems to be convention, but maybe it should be a separate function instead of the
// canonical impl for Option
impl<'i, IData, T: Decode<'i, IData>> Decode<'i, IData> for Option<T> {
    fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
        match input {
            TokenTree::Literal(Literal::Bool(false)) => Ok(None),
            val => val.decode().map(Self::Some),
        }
    }
}

macro_rules! impl_decode_for_wrapper {
    ($($Wrapper:ident),+) => {
        $(
        impl<'i, IData, T: Decode<'i, IData>> Decode<'i, IData> for $Wrapper<T> {
            fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
                T::decode(input).map($Wrapper::new)
            }
        }
        )+
    };
}

impl_decode_for_wrapper!(Box, Rc, Arc, Cell, RefCell, Mutex);

impl<'i, IData, T> Decode<'i, IData> for Box<[T]>
where
    Vec<T>: Decode<'i, IData>,
{
    fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
        Vec::<T>::decode(input).map(From::from)
    }
}

macro_rules! impl_decode_to_tokens {
    ($Id:ident, $expected:expr => $OData:ident, $Ty:ty, $val:ident => $into:expr) => {
        impl<'i, 'o, IData, $OData> Decode<'i, IData> for $Ty
        where
            IData: BorrowOrShare<'i, 'o, [u8]>,
            &'o [u8]: Into<$OData>,
        {
            fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
                match input {
                    TokenTree::$Id($val) => Ok($into),
                    _ => Err(DecodeError::unexpected($expected, input)),
                }
            }
        }

        // impl<'i> Decode<'i> for &'i $Ty {
        //     fn decode(input: &'i TokenTree) -> Result<Self, DecodeError> {
        //         match input {
        //             TokenTree::$Id(val) => Ok(val),
        //             _ => Err(DecodeError::unexpected(
        //                 Cow::Borrowed($expected),
        //                 input.clone(),
        //             )),
        //         }
        //     }
        // }
    };
}

impl_decode_to_tokens! {List, SyrupKind::List { length: None } => OData, List<OData>, val => val.into()}
impl_decode_to_tokens! {Dictionary, SyrupKind::Dictionary => OData, Dictionary<OData>, val => val.into()}
impl_decode_to_tokens! {Set, SyrupKind::Set => OData, Set<OData>, val => val.into()}
impl_decode_to_tokens! {Record, SyrupKind::Record { label: None } => OData, Record<OData>, val => (&**val).into()}
impl_decode_to_tokens! {Literal, SyrupKind::Unknown("Literal") => OData, Literal<OData>, val => val.into()}

macro_rules! impl_parse_simple {
    ($Ty:ty, $Lit:ident) => {
        impl<'i, IData> Decode<'i, IData> for $Ty
        where
            IData: Bos<[u8]>,
        {
            fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
                match input {
                    TokenTree::Literal(Literal::$Lit(val)) => Ok(*val),
                    _ => Err(DecodeError::unexpected(SyrupKind::$Lit, input)),
                }
            }
        }
    };
}
impl_parse_simple! {bool, Bool}
impl_parse_simple! {f32, F32}
impl_parse_simple! {f64, F64}

macro_rules! impl_parse_stringlike {
    ($i_lt:lifetime, $o_lt:lifetime, $String:ty, $Lit:ident) => {
        impl<$i_lt, $o_lt, IData> Decode<$i_lt, IData> for $String
        where
            IData: BorrowOrShare<$i_lt, $o_lt, [u8]>,
            &'o str: Into<$String>,
        {
            fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
                match input {
                    TokenTree::Literal(Literal::$Lit(val)) => {
                        Ok(std::str::from_utf8(val.borrow_or_share())?.into())
                    }
                    _ => Err(DecodeError::unexpected(SyrupKind::String, input)),
                }
            }
        }
    };
    ($String:ty, $Lit:ident) => {
        impl_parse_stringlike! {'i, 'o, $String, $Lit}
    };
}
impl_parse_stringlike! {String, String}
impl_parse_stringlike! {Box<str>, String}
impl_parse_stringlike! {Rc<str>, String}
impl_parse_stringlike! {Arc<str>, String}
impl_parse_stringlike! {'i, 'o, Cow<'o, str>, String}
// impl_parse_stringlike! {'i, 'o, &'o str, String}

impl<'i, 'o, IData> Decode<'i, IData> for &'o str
where
    IData: BorrowOrShare<'i, 'o, [u8]>,
{
    fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
        match input {
            TokenTree::Literal(Literal::String(val)) => {
                Ok(std::str::from_utf8(val.borrow_or_share())?)
            }
            _ => Err(DecodeError::unexpected(SyrupKind::String, input)),
        }
    }
}

macro_rules! impl_decode_for_int {
    ($($Int:ty),+$(,)?) => {
        $(
        impl<'i, IData> Decode<'i, IData> for $Int
        where
            IData: Bos<[u8]>
        {
            fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
                match input {
                    TokenTree::Literal(Literal::Int(int)) => <&'i $crate::de::Int<IData> as TryInto<$Int>>::try_into(int).map_err(|source| DecodeError::int::<$Int>(source.kind)),
                    _ => Err(DecodeError::unexpected(
                        SyrupKind::int::<$Int>(),
                        input
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

impl<'i, IData, T: Decode<'i, IData> + Into<Wrapping<T>>> Decode<'i, IData> for Wrapping<T> {
    fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
        T::decode(input).map(T::into)
    }
}

impl<'i, IData, T: Decode<'i, IData> + Into<Saturating<T>>> Decode<'i, IData> for Saturating<T> {
    fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
        T::decode(input).map(T::into)
    }
}

// #[cfg(feature = "decode-array")]
// impl<'i, T: Decode<'i>, const LEN: usize> Decode<'i> for [T; LEN] {
//     fn decode(input: &'i TokenTree) -> Result<Self, DecodeError> {
//         use std::mem::MaybeUninit;
//         fn initialize_array<'i, 'e, T: Decode<'i>, const LEN: usize>(
//             init_amt: &mut usize,
//             array: &mut [MaybeUninit<T>; LEN],
//             tokens: &'i [TokenTree],
//         ) -> Result<(), DecodeError> {
//             for (i, slot) in array.iter_mut().enumerate() {
//                 let element = match tokens.get(i) {
//                     Some(el) => T::decode(el)?,
//                     None => {
//                         return Err(DecodeError::missing(Cow::Owned(format!(
//                             "{i}th array element"
//                         ))));
//                     }
//                 };
//                 slot.write(element);
//                 *init_amt += 1;
//             }
//             Ok(())
//         }
//         match input {
//             TokenTree::List(List { elements }) => {
//                 #[expect(unsafe_code)]
//                 unsafe {
//                     let mut init_amt = 0;
//                     let mut res = [const { MaybeUninit::<T>::uninit() }; LEN];
//                     if let Err(error) =
//                         initialize_array::<T, LEN>(&mut init_amt, &mut res, elements)
//                     {
//                         for val in &mut res[..init_amt] {
//                             val.assume_init_drop();
//                         }
//                         return Err(error);
//                     }
//                     #[expect(unsafe_code)]
//                     Ok(MaybeUninit::array_assume_init(res))
//                 }
//             }
//             _ => Err(DecodeError::unexpected(
//                 Cow::Owned(format!("list with {LEN} elements")),
//                 input.clone(),
//             )),
//         }
//     }
// }

impl<'i, IData, T: Decode<'i, IData>> Decode<'i, IData> for Vec<T>
where
    IData: Bos<[u8]>,
{
    fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
        match input {
            TokenTree::List(List { elements }) => {
                let mut res = Vec::with_capacity(elements.len());
                for token in elements {
                    res.push(T::decode(token)?);
                }
                Ok(res)
            }
            _ => Err(DecodeError::unexpected(
                SyrupKind::List { length: None },
                input,
            )),
        }
    }
}

impl<'i, IData, T, S> Decode<'i, IData> for HashSet<T, S>
where
    T: Decode<'i, IData> + std::hash::Hash + Eq,
    S: Default + std::hash::BuildHasher,
    IData: Bos<[u8]>,
{
    fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
        match input {
            TokenTree::Set(set) => {
                let mut res = HashSet::<T, S>::default();
                for element in set {
                    res.insert(element.decode()?);
                }
                Ok(res)
            }
            _ => Err(DecodeError::unexpected(SyrupKind::Set, input)),
        }
    }
}

impl<'i, IData, T> Decode<'i, IData> for BTreeSet<T>
where
    T: Decode<'i, IData> + std::cmp::Ord,
    IData: Bos<[u8]>,
{
    fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
        match input {
            TokenTree::Set(set) => {
                let mut res = Self::default();
                for element in set {
                    res.insert(element.decode()?);
                }
                Ok(res)
            }
            _ => Err(DecodeError::unexpected(SyrupKind::Set, input)),
        }
    }
}

impl<'i, IData, K, V, S> Decode<'i, IData> for HashMap<K, V, S>
where
    K: Decode<'i, IData> + std::hash::Hash + Eq,
    V: Decode<'i, IData>,
    S: Default + std::hash::BuildHasher,
    IData: Bos<[u8]>,
{
    fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
        match input {
            TokenTree::Dictionary(dict) => {
                let mut res = Self::default();
                for (k, v) in dict {
                    res.insert(K::decode(k)?, V::decode(v)?);
                }
                Ok(res)
            }
            _ => Err(DecodeError::unexpected(SyrupKind::Dictionary, input)),
        }
    }
}

impl<'i, IData, K, V> Decode<'i, IData> for BTreeMap<K, V>
where
    K: Decode<'i, IData> + std::cmp::Ord,
    V: Decode<'i, IData>,
    IData: Bos<[u8]>,
{
    fn decode(input: &'i TokenTree<IData>) -> Result<Self, DecodeError> {
        match input {
            TokenTree::Dictionary(dict) => {
                let mut res = Self::default();
                for (k, v) in dict {
                    res.insert(K::decode(k)?, V::decode(v)?);
                }
                Ok(res)
            }
            _ => Err(DecodeError::unexpected(SyrupKind::Dictionary, input)),
        }
    }
}
