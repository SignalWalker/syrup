use std::{borrow::Cow, marker::PhantomData, num::ParseIntError};

use borrow_or_share::{BorrowOrShare, Bos};
use nom::{
    IResult, Mode, Needed, OutputMode, Parser,
    bytes::streaming as bytes,
    character::streaming::one_of,
    combinator::recognize,
    error::{FromExternalError, ParseError},
    multi::many1,
    sequence::preceded,
};

mod int;
pub use int::*;

use crate::de::SyrupKind;

#[cfg(test)]
mod test;

struct ParseByte<E> {
    byte: u8,
    _e: PhantomData<E>,
}

impl<'input, E: ParseError<&'input [u8]>> Parser<&'input [u8]> for ParseByte<E> {
    type Output = ();

    type Error = E;

    fn process<OM: nom::OutputMode>(
        &mut self,
        input: &'input [u8],
    ) -> nom::PResult<OM, &'input [u8], Self::Output, Self::Error> {
        let Some(&c) = input.first() else {
            return Err(nom::Err::Incomplete(Needed::new(1)));
        };
        if c == self.byte {
            Ok((&input[1..], OM::Output::bind(|| ())))
        } else {
            Err(nom::Err::Error(OM::Error::bind(|| {
                E::from_char(input, char::from(self.byte))
            })))
        }
    }
}

struct TakeConstBytes<const AMT: usize, E> {
    _e: PhantomData<E>,
}

impl<'i, const AMT: usize, E: ParseError<&'i [u8]>> Parser<&'i [u8]> for TakeConstBytes<AMT, E> {
    type Output = &'i [u8; AMT];
    type Error = E;

    fn process<OM: OutputMode>(
        &mut self,
        input: &'i [u8],
    ) -> nom::PResult<OM, &'i [u8], Self::Output, Self::Error> {
        match input.first_chunk::<AMT>() {
            Some(res) => Ok((&input[AMT..], OM::Output::bind(|| res))),
            None => Err(nom::Err::Incomplete(Needed::new(AMT - input.len()))),
        }
    }
}

pub fn take_c<'i, const AMT: usize, E: ParseError<&'i [u8]>>()
-> impl Parser<&'i [u8], Output = &'i [u8; AMT], Error = E> {
    TakeConstBytes::<AMT, E> { _e: PhantomData }
}

pub fn byte<'input, E: ParseError<&'input [u8]>>(
    byte: u8,
) -> impl Parser<&'input [u8], Output = (), Error = E> {
    ParseByte {
        byte,
        _e: PhantomData,
    }
}

pub fn bool_literal<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], bool, E> {
    let Some(&c) = i.first() else {
        return Err(nom::Err::Incomplete(Needed::new(1)));
    };
    match c {
        b't' => Ok((&i[1..], true)),
        b'f' => Ok((&i[1..], false)),
        _ => Err(nom::Err::Error(E::from_error_kind(
            i,
            nom::error::ErrorKind::Char,
        ))),
    }
}

pub fn f32_literal<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], f32, E> {
    preceded(byte(b'F'), take_c::<4, E>().map(|&b| f32::from_be_bytes(b))).parse(i)
}

pub fn f64_literal<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], f64, E> {
    preceded(byte(b'D'), take_c::<8, E>().map(|&b| f64::from_be_bytes(b))).parse(i)
}

fn digits_dec<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], &'i [u8], E> {
    recognize(many1(one_of("0123456789"))).parse(i)
}

pub fn int_literal<'i, Digits, E: ParseError<&'i [u8]>>(
    i: &'i [u8],
) -> IResult<&'i [u8], Int<Digits>, E>
where
    &'i [u8]: Into<Digits>,
{
    #[expect(unsafe_code)]
    digits_dec
        .and(one_of("+-").map(|v| v == '+'))
        .map(|(digits, positive)| unsafe { Int::new(positive, digits.into()) })
        .parse(i)
}

pub trait ParseLiteralError<'i>:
    ParseError<&'i [u8]> + FromExternalError<&'i [u8], ParseIntError>
{
}

impl<'i, E: ParseError<&'i [u8]> + FromExternalError<&'i [u8], ParseIntError>> ParseLiteralError<'i>
    for E
{
}

fn literal_len<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], u64, E> {
    // TODO :: switch to u64::from_ascii when that's stable
    digits_dec
        .map_res(|digits| {
            #[expect(
                unsafe_code,
                reason = "we just checked that it's a string of ascii digits"
            )]
            <u64 as std::str::FromStr>::from_str(unsafe { std::str::from_utf8_unchecked(digits) })
        })
        .parse(i)
}

fn sized_literal_known<'i, const SEPARATOR: u8, Data, E: ParseLiteralError<'i>>(
    length: u64,
) -> impl Parser<&'i [u8], Output = Data, Error = E>
where
    &'i [u8]: Into<Data>,
{
    preceded(byte(SEPARATOR), bytes::take(length)).map(Into::into)
}

pub fn bytes_literal<'i, Data, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Data, E>
where
    &'i [u8]: Into<Data>,
{
    literal_len
        .flat_map(sized_literal_known::<b':', Data, E>)
        .parse(i)
}

pub fn string_literal<'i, Data, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Data, E>
where
    &'i [u8]: Into<Data>,
{
    literal_len
        .flat_map(sized_literal_known::<b'"', Data, E>)
        .parse(i)
}

pub fn symbol_literal<'i, Data, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Data, E>
where
    &'i [u8]: Into<Data>,
{
    literal_len
        .flat_map(sized_literal_known::<b'\'', Data, E>)
        .parse(i)
}

#[derive(Clone)]
pub enum Literal<Data> {
    Bool(bool),
    F32(f32),
    F64(f64),
    Int(Int<Data>),
    /// A string of raw bytes.
    Bytes(Data),
    /// A UTF-8 string.
    /// Not yet known to be valid UTF-8.
    String(Data),
    /// A UTF-8 string, marked as a symbol.
    /// Not yet known to be valid UTF-8.
    Symbol(Data),
}

impl<Data> std::fmt::Debug for Literal<Data>
where
    Data: Bos<[u8]>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Bool(b) => f.write_str(if *b { "true" } else { "false" }),
            Literal::F32(fl) => fl.fmt(f),
            Literal::F64(fl) => fl.fmt(f),
            Literal::Int(int) => int.fmt(f),
            Literal::Bytes(data) => write!(f, "{}:<...>", data.borrow_or_share().len()),
            Literal::String(data) => {
                write!(f, "\"{}\"", String::from_utf8_lossy(data.borrow_or_share()))
            }
            Literal::Symbol(data) => {
                write!(f, "'{}", String::from_utf8_lossy(data.borrow_or_share()))
            }
        }
    }
}

impl<LData, RData> PartialEq<Literal<RData>> for Literal<LData>
where
    LData: PartialEq<RData>,
{
    fn eq(&self, other: &Literal<RData>) -> bool {
        match (self, other) {
            (Self::Bool(l0), Literal::Bool(r0)) => l0 == r0,
            (Self::F32(l0), Literal::F32(r0)) => l0.to_be_bytes() == r0.to_be_bytes(),
            (Self::F64(l0), Literal::F64(r0)) => l0.to_be_bytes() == r0.to_be_bytes(),
            (Self::Int(l0), Literal::Int(r0)) => l0.eq(r0),
            (Self::Bytes(l0), Literal::Bytes(r0))
            | (Self::String(l0), Literal::String(r0))
            | (Self::Symbol(l0), Literal::Symbol(r0)) => l0.eq(r0),
            _ => false,
        }
    }
}

impl<Data> Eq for Literal<Data> where Data: PartialEq + Eq {}

impl<Data> Literal<Data> {
    pub fn encode<'i, 'o>(&'i self) -> Cow<'o, [u8]>
    where
        Data: BorrowOrShare<'i, 'o, [u8]>,
    {
        fn sized_literal<const SEP: u8>(bytes: &[u8]) -> Vec<u8> {
            let digits = bytes.len().to_string().into_bytes();
            let mut res = Vec::with_capacity(digits.len() + 1 + bytes.len());
            res.extend(digits);
            res.push(SEP);
            res.extend(bytes);
            res
        }
        match self {
            Literal::Bool(b) => match *b {
                true => Cow::Borrowed(b"t"),
                false => Cow::Borrowed(b"f"),
            },
            Literal::F32(f) => {
                let mut res = Vec::with_capacity(1 + std::mem::size_of::<f32>());
                res.push(b'F');
                res.extend(f.to_be_bytes());
                Cow::Owned(res)
            }
            Literal::F64(d) => {
                let mut res = Vec::with_capacity(1 + std::mem::size_of::<f64>());
                res.push(b'D');
                res.extend(d.to_be_bytes());
                Cow::Owned(res)
            }
            Literal::Int(int) => int.encode(),
            Literal::Bytes(bytes) => Cow::Owned(sized_literal::<b':'>(bytes.borrow_or_share())),
            Literal::String(bytes) => Cow::Owned(sized_literal::<b'"'>(bytes.borrow_or_share())),
            Literal::Symbol(bytes) => Cow::Owned(sized_literal::<b'\''>(bytes.borrow_or_share())),
        }
    }

    pub fn encode_into(&self, w: &mut impl std::io::Write) -> std::io::Result<usize>
    where
        Data: Bos<[u8]>,
    {
        fn sized_literal<const SEP: u8>(
            bytes: &[u8],
            w: &mut impl std::io::Write,
        ) -> std::io::Result<usize> {
            let digits = bytes.len().to_string().into_bytes();
            w.write_all(&digits)?;
            w.write_all(&[SEP])?;
            w.write_all(bytes)?;
            Ok(digits.len() + 1 + bytes.len())
        }
        match self {
            Literal::Bool(b) => match *b {
                true => w.write_all(b"t").map(|_| 1),
                false => w.write_all(b"f").map(|_| 1),
            },
            Literal::F32(f) => {
                w.write_all(b"F")?;
                w.write_all(&f.to_be_bytes())?;
                Ok(1 + std::mem::size_of::<f32>())
            }
            Literal::F64(d) => {
                w.write_all(b"D")?;
                w.write_all(&d.to_be_bytes())?;
                Ok(1 + std::mem::size_of::<f64>())
            }
            Literal::Int(int) => int.encode_into(w),
            Literal::Bytes(bytes) => sized_literal::<b':'>(bytes.borrow_or_share(), w),
            Literal::String(bytes) => sized_literal::<b'"'>(bytes.borrow_or_share(), w),
            Literal::Symbol(bytes) => sized_literal::<b'\''>(bytes.borrow_or_share(), w),
        }
    }

    pub fn parse<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Self, E>
    where
        &'i [u8]: Into<Data>,
    {
        bool_literal
            .map(Literal::Bool)
            .or(f32_literal.map(Literal::F32))
            .or(f64_literal.map(Literal::F64))
            .or(int_literal.map(Literal::Int))
            .or(bytes_literal.map(Literal::Bytes))
            .or(string_literal.map(Literal::String))
            .or(symbol_literal.map(Literal::Symbol))
            .parse(i)
    }

    pub fn data_into<IData>(self) -> Literal<IData>
    where
        Data: Into<IData>,
    {
        match self {
            Literal::Bool(b) => Literal::Bool(b),
            Literal::F32(f) => Literal::F32(f),
            Literal::F64(f) => Literal::F64(f),
            Literal::Int(int) => Literal::Int(int.digits_into()),
            Literal::Bytes(b) => Literal::Bytes(b.into()),
            Literal::String(s) => Literal::String(s.into()),
            Literal::Symbol(s) => Literal::Symbol(s.into()),
        }
    }

    pub fn kind(&self) -> SyrupKind
    where
        Data: Bos<[u8]>,
    {
        match self {
            Literal::Bool(_) => SyrupKind::Bool,
            Literal::F32(_) => SyrupKind::F32,
            Literal::F64(_) => SyrupKind::F64,
            Literal::Int(_) => SyrupKind::Int { desc: None },
            Literal::Bytes(b) => SyrupKind::Bytes {
                length: Some(b.borrow_or_share().len()),
            },
            Literal::String(_) => SyrupKind::String,
            // TODO :: convert symbol text...?
            Literal::Symbol(_) => SyrupKind::Symbol(None),
        }
    }
}

impl<'i, 'o, IData, OData> From<&'i Literal<IData>> for Literal<OData>
where
    IData: BorrowOrShare<'i, 'o, [u8]>,
    &'o [u8]: Into<OData>,
{
    fn from(value: &'i Literal<IData>) -> Self {
        match value {
            Literal::Bool(b) => Literal::Bool(*b),
            Literal::F32(f) => Literal::F32(*f),
            Literal::F64(f) => Literal::F64(*f),
            Literal::Int(int) => Literal::Int(int.borrow_or_share().into()),
            Literal::Bytes(b) => Literal::Bytes(b.borrow_or_share().into()),
            Literal::String(s) => Literal::String(s.borrow_or_share().into()),
            Literal::Symbol(s) => Literal::Symbol(s.borrow_or_share().into()),
        }
    }
}
