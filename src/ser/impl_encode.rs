use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, LinkedList, VecDeque},
};

use crate::{
    de::{Dictionary, Int, List, Literal, Set, TokenTree},
    ser::Encode,
};

#[cfg(test)]
mod test;

mod _impl_tuple {
    use crate as syrup;

    syrup_proc::impl_encode_for_tuple!(32);
}

impl Encode for TokenTree {
    #[inline]
    fn encode(&self) -> TokenTree {
        self.clone()
    }
}

macro_rules! impl_encode_copy {
    ($Ty:ty, $Id:ident) => {
        impl Encode for $Ty {
            #[inline]
            fn encode(&self) -> TokenTree {
                TokenTree::Literal(Literal::$Id(*self))
            }
        }
    };
}

impl_encode_copy! {bool, Bool}
impl_encode_copy! {f32, F32}
impl_encode_copy! {f64, F64}

macro_rules! impl_encode_int {
    ($($Int:ty),+$(,)?) => {
        $(
        impl Encode for $Int {
            #[inline]
            fn encode(&self) -> TokenTree {
                TokenTree::Literal(Literal::Int(Int::from(*self)))
            }
        }
        )+
    };
}

impl_encode_int!(
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
    std::num::NonZeroU8,
    std::num::NonZeroU16,
    std::num::NonZeroU32,
    std::num::NonZeroU64,
    std::num::NonZeroUsize,
    std::num::NonZeroU128,
    std::num::NonZeroI8,
    std::num::NonZeroI16,
    std::num::NonZeroI32,
    std::num::NonZeroI64,
    std::num::NonZeroIsize,
    std::num::NonZeroI128,
);

macro_rules! impl_encode_str {
    ($S:ty => $Lit:ident) => {
        impl Encode for $S {
            fn encode(&self) -> TokenTree {
                TokenTree::Literal(Literal::$Lit(self.as_bytes().to_vec()))
            }
        }
    };
}

impl_encode_str! {str => String}
impl_encode_str! {String => String}
impl_encode_str! {Cow<'_, str> => String}

macro_rules! impl_encode_list {
    ($T:ident; $List:ty) => {
        impl<$T> Encode for $List
        where
            $T: Encode,
        {
            fn encode(&self) -> TokenTree {
                TokenTree::List(List::new(self.iter().map(<$T>::encode).collect()))
            }
        }
    };
}

impl_encode_list! {T; [T]}
impl_encode_list! {T; Vec<T>}
impl_encode_list! {T; VecDeque<T>}
impl_encode_list! {T; LinkedList<T>}

impl<'t, T: Encode> Encode for Cow<'t, [T]>
where
    [T]: std::borrow::ToOwned,
{
    fn encode(&self) -> TokenTree {
        TokenTree::List(List::new(self.iter().map(<T>::encode).collect()))
    }
}

// TODO :: is there a more efficient way to encode to dictionaries & sets?

impl<K, V, S> Encode for HashMap<K, V, S>
where
    K: Encode,
    V: Encode,
{
    fn encode(&self) -> TokenTree {
        TokenTree::Dictionary(Dictionary::new(
            self.iter()
                .map(|(key, value)| (key.encode(), value.encode()))
                .collect(),
        ))
    }
}

impl<T, S> Encode for HashSet<T, S>
where
    T: Encode,
{
    fn encode(&self) -> TokenTree {
        TokenTree::Set(Set::new(self.iter().map(<T>::encode).collect()))
    }
}

impl<K, V> Encode for BTreeMap<K, V>
where
    K: Encode,
    V: Encode,
{
    fn encode(&self) -> TokenTree {
        TokenTree::Dictionary(Dictionary::new(
            self.iter()
                .map(|(key, value)| (key.encode(), value.encode()))
                .collect(),
        ))
    }
}

impl<T> Encode for BTreeSet<T>
where
    T: Encode,
{
    fn encode(&self) -> TokenTree {
        TokenTree::Set(Set::new(self.iter().map(<T>::encode).collect()))
    }
}
