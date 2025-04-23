use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, LinkedList, VecDeque},
};

use borrow_or_share::{BorrowOrShare, Bos};

use crate::{
    de::{
        Dictionary, Int, List, Literal, Set, TokenTree, dict_encoded_entries, encode_into_as_dict,
        encode_into_as_set, set_encoded_entries,
    },
    ser::{Encode, EncodeInto, EncodeIntoExt},
};

#[cfg(test)]
mod test;

mod _impl_tuple {
    use crate as syrup;

    syrup_proc::impl_encode_for_tuple!(32);
}

impl<'i, 'o, IData, OData> Encode<'i, OData> for TokenTree<IData>
where
    IData: BorrowOrShare<'i, 'o, [u8]>,
    &'o [u8]: Into<OData>,
{
    #[inline]
    fn encode(&'i self) -> TokenTree<OData> {
        self.into()
    }
}

impl<'i, IData> EncodeInto<'i> for TokenTree<IData>
where
    IData: Bos<[u8]>,
{
    fn encode_into(&'i self, w: &mut impl std::io::Write) -> std::io::Result<usize> {
        self.write_bytes(w)
    }
}

macro_rules! impl_encode_copy {
    ($Ty:ty, $Id:ident) => {
        impl<'i, OData> Encode<'i, OData> for $Ty {
            #[inline]
            fn encode(&'i self) -> TokenTree<OData> {
                TokenTree::Literal(Literal::$Id(*self))
            }
        }

        impl<'i> EncodeInto<'i> for $Ty {
            fn encode_into(&'i self, w: &mut impl std::io::Write) -> std::io::Result<usize> {
                <Self as Encode<'i, &'i [u8]>>::encode(self).write_bytes(w)
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
        impl<'i, OData> Encode<'i, OData> for $Int where $Int: Into<Int<OData>> {
            #[inline]
            fn encode(&'i self) -> TokenTree<OData> {
                TokenTree::Literal(Literal::Int((*self).into()))
            }
        }

        impl<'i> EncodeInto<'i> for $Int {
            fn encode_into(&'i self, w: &mut impl std::io::Write) -> std::io::Result<usize> {
                Int::<Vec<u8>>::from(*self).encode_into(w)
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
        impl<'i, OData> Encode<'i, OData> for $S
        where
            &'i [u8]: Into<OData>,
        {
            fn encode(&'i self) -> TokenTree<OData> {
                TokenTree::Literal(Literal::$Lit(self.as_bytes().into()))
            }
        }

        impl<'i> EncodeInto<'i> for $S {
            fn encode_into(&'i self, w: &mut impl std::io::Write) -> std::io::Result<usize> {
                <Self as Encode<'i, &'i [u8]>>::encode(self).write_bytes(w)
            }
        }
    };
}

impl_encode_str! {str => String}
impl_encode_str! {String => String}
impl_encode_str! {Cow<'_, str> => String}

macro_rules! impl_encode_list {
    ($T:ident; $List:ty) => {
        impl<'i, OData, $T> Encode<'i, OData> for $List
        where
            $T: Encode<'i, OData>,
        {
            fn encode(&'i self) -> TokenTree<OData> {
                TokenTree::List(List::new(self.iter().map(<$T>::encode).collect()))
            }
        }

        impl<'i, $T> EncodeInto<'i> for $List
        where
            $T: EncodeInto<'i>,
        {
            fn encode_into(&'i self, w: &mut impl std::io::Write) -> std::io::Result<usize> {
                let mut amt = 2; // starting at 2 for the []
                w.write_all(b"[")?;
                for element in self {
                    amt += element.encode_into(w)?;
                }
                w.write_all(b"]")?;
                Ok(amt)
            }
        }
    };
}

impl_encode_list! {T; [T]}
impl_encode_list! {T; Vec<T>}
impl_encode_list! {T; VecDeque<T>}
impl_encode_list! {T; LinkedList<T>}

impl<'i, OData, T: Encode<'i, OData>> Encode<'i, OData> for Cow<'i, [T]>
where
    [T]: std::borrow::ToOwned,
{
    fn encode(&'i self) -> TokenTree<OData> {
        TokenTree::List(List::new(self.iter().map(<T>::encode).collect()))
    }
}

impl<'i, T> EncodeInto<'i> for Cow<'i, [T]>
where
    [T]: std::borrow::ToOwned,
    T: EncodeInto<'i>,
{
    fn encode_into(&'i self, w: &mut impl std::io::Write) -> std::io::Result<usize> {
        let mut amt = 2; // starting at 2 for the []
        w.write_all(b"[")?;
        for element in self.iter() {
            amt += element.encode_into(w)?;
        }
        w.write_all(b"]")?;
        Ok(amt)
    }
}

// TODO :: is there a more efficient way to encode to dictionaries & sets?

impl<'i, OData, K, V, S> Encode<'i, OData> for HashMap<K, V, S>
where
    K: Encode<'i, OData>,
    V: Encode<'i, OData>,
{
    fn encode(&'i self) -> TokenTree<OData> {
        TokenTree::Dictionary(Dictionary::new(
            self.iter()
                .map(|(key, value)| (key.encode(), value.encode()))
                .collect(),
        ))
    }
}

impl<'i, K, V, S> EncodeInto<'i> for HashMap<K, V, S>
where
    K: for<'c> EncodeIntoExt<'c>,
    V: for<'c> EncodeIntoExt<'c>,
{
    fn encode_into(&'i self, w: &mut impl std::io::Write) -> std::io::Result<usize> {
        let (_, pairs) = dict_encoded_entries(
            self.iter()
                .map(|(k, v)| (k.encode_bytes().into(), v.encode_bytes().into())),
        );
        encode_into_as_dict(&pairs, w)
    }
}

impl<'i, OData, T, S> Encode<'i, OData> for HashSet<T, S>
where
    T: Encode<'i, OData>,
{
    fn encode(&'i self) -> TokenTree<OData> {
        TokenTree::Set(Set::new(self.iter().map(<T>::encode).collect()))
    }
}

impl<'i, T, S> EncodeInto<'i> for HashSet<T, S>
where
    T: for<'c> EncodeIntoExt<'c>,
{
    fn encode_into(&'i self, w: &mut impl std::io::Write) -> std::io::Result<usize> {
        let (_, sorted) = set_encoded_entries(self.iter().map(|e| e.encode_bytes().into()));
        encode_into_as_set(&sorted, w)
    }
}

impl<'i, OData, K, V> Encode<'i, OData> for BTreeMap<K, V>
where
    K: Encode<'i, OData>,
    V: Encode<'i, OData>,
{
    fn encode(&'i self) -> TokenTree<OData> {
        TokenTree::Dictionary(Dictionary::new(
            self.iter()
                .map(|(key, value)| (key.encode(), value.encode()))
                .collect(),
        ))
    }
}

impl<'i, K, V> EncodeInto<'i> for BTreeMap<K, V>
where
    K: for<'c> EncodeIntoExt<'c>,
    V: for<'c> EncodeIntoExt<'c>,
{
    fn encode_into(&'i self, w: &mut impl std::io::Write) -> std::io::Result<usize> {
        let (_, pairs) = dict_encoded_entries(
            self.iter()
                .map(|(k, v)| (k.encode_bytes().into(), v.encode_bytes().into())),
        );
        encode_into_as_dict(&pairs, w)
    }
}

impl<'i, OData, T> Encode<'i, OData> for BTreeSet<T>
where
    T: Encode<'i, OData>,
{
    fn encode(&'i self) -> TokenTree<OData> {
        TokenTree::Set(Set::new(self.iter().map(<T>::encode).collect()))
    }
}

impl<'i, T> EncodeInto<'i> for BTreeSet<T>
where
    T: for<'c> EncodeIntoExt<'c>,
{
    fn encode_into(&'i self, w: &mut impl std::io::Write) -> std::io::Result<usize> {
        let (_, sorted) = set_encoded_entries(self.iter().map(|e| e.encode_bytes().into()));
        encode_into_as_set(&sorted, w)
    }
}
