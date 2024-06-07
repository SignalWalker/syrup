use std::ops::{Range, RangeFrom, RangeFull, RangeTo};

use nom::{InputIter, InputLength, InputTake, InputTakeAtPosition, Needed};

use crate::de::Cursor;

impl<'rem> InputLength for Cursor<'rem> {
    fn input_len(&self) -> usize {
        self.rem.len()
    }
}

impl<'rem> InputIter for Cursor<'rem> {
    type Item = u8;

    type Iter = <&'rem [u8] as InputIter>::Iter;

    type IterElem = <&'rem [u8] as InputIter>::IterElem;

    fn iter_indices(&self) -> Self::Iter {
        self.rem.iter_indices()
    }

    fn iter_elements(&self) -> Self::IterElem {
        self.rem.iter_elements()
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.rem.position(predicate)
    }

    fn slice_index(&self, count: usize) -> Result<usize, nom::Needed> {
        self.rem.slice_index(count)
    }
}

impl<'rem> InputTake for Cursor<'rem> {
    fn take(&self, count: usize) -> Self {
        Self {
            rem: &self.rem[0..count],
            off: self.off,
        }
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        (*self, self.advance(count))
    }
}

impl<'rem> InputTakeAtPosition for Cursor<'rem> {
    type Item = u8;

    fn split_at_position<P, E: nom::error::ParseError<Self>>(
        &self,
        predicate: P,
    ) -> nom::IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.rem.iter().position(|c| predicate(*c)) {
            Some(i) => Ok(self.take_split(i)),
            None => Err(nom::Err::Incomplete(Needed::new(1))),
        }
    }

    fn split_at_position1<P, E: nom::error::ParseError<Self>>(
        &self,
        predicate: P,
        e: nom::error::ErrorKind,
    ) -> nom::IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.rem.iter().position(|c| predicate(*c)) {
            Some(0) => Err(nom::Err::Error(E::from_error_kind(*self, e))),
            Some(i) => Ok(self.take_split(i)),
            None => Err(nom::Err::Incomplete(Needed::new(1))),
        }
    }

    fn split_at_position_complete<P, E: nom::error::ParseError<Self>>(
        &self,
        predicate: P,
    ) -> nom::IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.rem.iter().position(|c| predicate(*c)) {
            Some(i) => Ok(self.take_split(i)),
            None => Ok(self.take_split(self.input_len())),
        }
    }

    fn split_at_position1_complete<P, E: nom::error::ParseError<Self>>(
        &self,
        predicate: P,
        e: nom::error::ErrorKind,
    ) -> nom::IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.rem.iter().position(|c| predicate(*c)) {
            Some(0) => Err(nom::Err::Error(E::from_error_kind(*self, e))),
            Some(i) => Ok(self.take_split(i)),
            None => {
                if self.rem.is_empty() {
                    Err(nom::Err::Error(E::from_error_kind(*self, e)))
                } else {
                    Ok(self.take_split(self.input_len()))
                }
            }
        }
    }
}

impl<'left, 'right> nom::Compare<Cursor<'right>> for Cursor<'left> {
    fn compare(&self, t: Cursor<'right>) -> nom::CompareResult {
        self.rem.compare(t.rem)
    }

    fn compare_no_case(&self, t: Cursor<'right>) -> nom::CompareResult {
        self.rem.compare_no_case(t.rem)
    }
}

impl<'left, 'right> nom::Compare<&'right [u8]> for Cursor<'left> {
    fn compare(&self, t: &'right [u8]) -> nom::CompareResult {
        self.rem.compare(t)
    }

    fn compare_no_case(&self, t: &'right [u8]) -> nom::CompareResult {
        self.rem.compare_no_case(t)
    }
}

impl<'rem> nom::Slice<Range<usize>> for Cursor<'rem> {
    fn slice(&self, range: Range<usize>) -> Self {
        Self {
            off: self.off + range.start,
            rem: &self.rem[range],
        }
    }
}

impl<'rem> nom::Slice<RangeTo<usize>> for Cursor<'rem> {
    fn slice(&self, range: RangeTo<usize>) -> Self {
        Self {
            rem: &self.rem[range],
            off: self.off,
        }
    }
}

impl<'rem> nom::Slice<RangeFrom<usize>> for Cursor<'rem> {
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        Self {
            off: self.off + range.start,
            rem: &self.rem[range],
        }
    }
}

impl<'rem> nom::Slice<RangeFull> for Cursor<'rem> {
    fn slice(&self, _: RangeFull) -> Self {
        *self
    }
}
