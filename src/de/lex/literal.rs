use std::{borrow::Cow, marker::PhantomData, num::ParseIntError};

use nom::{
    bytes::streaming as bytes,
    character::streaming::one_of,
    combinator::recognize,
    error::{FromExternalError, ParseError},
    multi::many1,
    sequence::preceded,
    IResult, Mode, Needed, OutputMode, Parser,
};

mod int;
pub use int::*;

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

pub fn take_c<'i, const AMT: usize, E: ParseError<&'i [u8]>>(
) -> impl Parser<&'i [u8], Output = &'i [u8; AMT], Error = E> {
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

pub fn int_literal<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], Int<'i>, E> {
    #[allow(unsafe_code)]
    digits_dec
        .and(one_of("+-").map(|v| v == '+'))
        .map(|(digits, positive)| unsafe { Int::new(positive, Cow::Borrowed(digits)) })
        .parse(i)
}

pub trait ParseLiteralError<'i> = ParseError<&'i [u8]> + FromExternalError<&'i [u8], ParseIntError>;

fn literal_len<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], u64, E> {
    digits_dec.map_res(u64::from_ascii).parse(i)
}

fn sized_literal_known<'i, const SEPARATOR: u8, E: ParseLiteralError<'i>>(
    length: u64,
) -> impl Parser<&'i [u8], Output = Vec<u8>, Error = E> {
    preceded(byte(SEPARATOR), bytes::take(length)).map(<[u8]>::to_vec)
}

pub fn bytes_literal<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Vec<u8>, E> {
    literal_len
        .flat_map(sized_literal_known::<b':', E>)
        .parse(i)
}

pub fn string_literal<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Vec<u8>, E> {
    literal_len
        .flat_map(sized_literal_known::<b'"', E>)
        .parse(i)
}

pub fn symbol_literal<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Vec<u8>, E> {
    literal_len
        .flat_map(sized_literal_known::<b'\'', E>)
        .parse(i)
}

#[derive(Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Literal {
    Bool(bool),
    F32(f32),
    F64(f64),
    Int(Int<'static>),
    Bytes(Vec<u8>),
    /// Not known to be valid UTF-8.
    String(#[cfg_attr(test, proptest(regex = ".*"))] Vec<u8>),
    /// Not known to be valid UTF-8.
    Symbol(#[cfg_attr(test, proptest(regex = ".*"))] Vec<u8>),
}

impl std::fmt::Debug for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Bool(b) => f.write_str(if *b { "true" } else { "false" }),
            Literal::F32(fl) => fl.fmt(f),
            Literal::F64(fl) => fl.fmt(f),
            Literal::Int(int) => int.fmt(f),
            Literal::Bytes(data) => write!(f, "{}:<...>", data.len()),
            Literal::String(data) => write!(f, "\"{}\"", String::from_utf8_lossy(data)),
            Literal::Symbol(data) => write!(f, "'{}", String::from_utf8_lossy(data)),
        }
    }
}

impl PartialEq for Literal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::F32(l0), Self::F32(r0)) => l0.to_be_bytes() == r0.to_be_bytes(),
            (Self::F64(l0), Self::F64(r0)) => l0.to_be_bytes() == r0.to_be_bytes(),
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            (Self::Bytes(l0), Self::Bytes(r0))
            | (Self::String(l0), Self::String(r0))
            | (Self::Symbol(l0), Self::Symbol(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl Eq for Literal {}

impl Literal {
    pub fn encode(&self) -> Vec<u8> {
        #[inline]
        fn sized_literal<const SEP: u8>(bytes: &[u8]) -> Vec<u8> {
            let digits = bytes.len().to_string().into_bytes();
            let mut res = Vec::with_capacity(digits.len() + 1 + bytes.len());
            res.extend(digits);
            res.push(SEP);
            res.extend(bytes);
            res
        }
        match self {
            Literal::Bool(b) => {
                if *b {
                    vec![b't']
                } else {
                    vec![b'f']
                }
            }
            Literal::F32(f) => {
                let mut res = Vec::with_capacity(1 + std::mem::size_of::<f32>());
                res.push(b'F');
                res.extend(f.to_be_bytes());
                res
            }
            Literal::F64(d) => {
                let mut res = Vec::with_capacity(1 + std::mem::size_of::<f64>());
                res.push(b'D');
                res.extend(d.to_be_bytes());
                res
            }
            Literal::Int(int) => int.encode(),
            Literal::Bytes(bytes) => sized_literal::<b':'>(bytes),
            Literal::String(bytes) => sized_literal::<b'"'>(bytes),
            Literal::Symbol(bytes) => sized_literal::<b'\''>(bytes),
        }
    }

    pub fn parse<'i, E: ParseLiteralError<'i>>(i: &'i [u8]) -> IResult<&'i [u8], Self, E> {
        bool_literal
            .map(Literal::Bool)
            .or(f32_literal.map(Literal::F32))
            .or(f64_literal.map(Literal::F64))
            .or(int_literal.map(|i| Literal::Int(i.into_static())))
            .or(bytes_literal.map(Literal::Bytes))
            .or(string_literal.map(Literal::String))
            .or(symbol_literal.map(Literal::Symbol))
            .parse(i)
    }
}
