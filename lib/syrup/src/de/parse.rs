use crate::Symbol;
use ibig::{IBig, UBig};
use nom::{
    bytes::streaming as bytes,
    character::streaming as nchar,
    combinator::value,
    error::ParseError,
    number::streaming as number,
    number::Endianness,
    sequence::{preceded, terminated},
    IResult, Parser,
};

pub fn parse_bool<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], bool, E> {
    value(true, nchar::char('t'))
        .or(value(false, nchar::char('f')))
        .parse(i)
}

pub fn parse_f32<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], f32, E> {
    preceded(nchar::char('F'), number::f32(Endianness::Big)).parse(i)
}

pub fn parse_f64<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], f64, E> {
    preceded(nchar::char('D'), number::f64(Endianness::Big)).parse(i)
}

pub fn parse_ubig<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], UBig, E> {
    nchar::digit1
        .map(|b| {
            UBig::from_str_radix(
                unsafe {
                    // we already know this is a sequence of ascii digits
                    std::str::from_utf8_unchecked(b)
                },
                10,
            )
            .unwrap()
        })
        .parse(i)
}

pub fn parse_int<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], IBig, E> {
    parse_ubig
        .and(nchar::one_of("+-"))
        .map(|(i, sign)| match sign {
            '+' => IBig::from(i),
            '-' => IBig::from(i) * -1,
            _ => unreachable!(),
        })
        .parse(i)
}

macro_rules! parse_i_n {
    ($parse_fn:ident, $Int:ty) => {
        pub fn $parse_fn<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], $Int, E>
        where
            E: nom::error::FromExternalError<&'i [u8], <$Int as TryFrom<IBig>>::Error>,
        {
            nom::combinator::map_res(parse_int, <$Int>::try_from).parse(i)
        }
    };
}

parse_i_n!(parse_u8, u8);
parse_i_n!(parse_u16, u16);
parse_i_n!(parse_u32, u32);
parse_i_n!(parse_u64, u64);
parse_i_n!(parse_u128, u128);
parse_i_n!(parse_usize, usize);
parse_i_n!(parse_i8, i8);
parse_i_n!(parse_i16, i16);
parse_i_n!(parse_i32, i32);
parse_i_n!(parse_i64, i64);
parse_i_n!(parse_i128, i128);
parse_i_n!(parse_isize, isize);

pub fn parse_netstring<
    'i,
    E: ParseError<&'i [u8]>,
    Output,
    OutputParser: Parser<&'i [u8], Output, E>,
    ParseFn: FnMut(usize) -> OutputParser,
>(
    sep: char,
    parse_fn: ParseFn,
) -> impl Parser<&'i [u8], Output, E> {
    fn usize_terminated<'i, E: ParseError<&'i [u8]>>(
        term: char,
    ) -> impl Parser<&'i [u8], usize, E> {
        terminated::<&'i [u8], usize, char, E, _, _>(
            parse_ubig::<'i, E>.map::<_, usize>(|amt| usize::try_from(amt).unwrap()),
            nchar::char::<&'i [u8], E>(term),
        )
    }
    nom::combinator::flat_map(usize_terminated::<'i, E>(sep), parse_fn)
}

pub fn parse_byte_obj<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], &'i [u8], E> {
    parse_netstring(':', bytes::take).parse(i)
}

pub fn parse_str<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], &'i str, E> {
    parse_netstring('"', bytes::take)
        .map(|b| unsafe { std::str::from_utf8_unchecked(b) })
        .parse(i)
}

pub fn parse_char<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], char, E> {
    preceded(
        bytes::tag("1\""),
        bytes::take(1usize).map(|b: &[u8]| char::from(b[0])),
    )
    .parse(i)
}

pub fn parse_symbol<'i, E: ParseError<&'i [u8]>>(
    i: &'i [u8],
) -> IResult<&'i [u8], Symbol<&'i str>, E> {
    parse_netstring('\'', bytes::take)
        .map(|b| Symbol(unsafe { std::str::from_utf8_unchecked(b) }))
        .parse(i)
}

// pub fn parse_item<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], ItemRef<'i>, E> {
//     parse_bool
//         .map(ItemRef::Bool)
//         .or(parse_f32.map(ItemRef::F32))
//         .or(parse_f64.map(ItemRef::F64))
//         .or(parse_dictionary.map(ItemRef::Dictionary))
//         .or(parse_sequence.map(ItemRef::Sequence))
//         .or(parse_record_ref.map(ItemRef::Record))
//         .or(parse_set.map(ItemRef::Set))
//         .or(parse_int.map(ItemRef::Int))
//         .or(parse_byte_obj.map(ItemRef::Bytes))
//         .or(parse_str.map(ItemRef::String))
//         .or(parse_symbol.map(ItemRef::Symbol))
//         .parse(i)
// }

// pub fn parse_dictionary<'i, E: ParseError<&'i [u8]>>(
//     i: &'i [u8],
// ) -> IResult<&'i [u8], HashMap<ItemRef<'i>, ItemRef<'i>>, E> {
//     delimited(
//         nchar::char('{'),
//         multi::many0(parse_item.and(parse_item)).map(HashMap::from_iter),
//         nchar::char('}'),
//     )
//     .parse(i)
// }

// pub fn parse_sequence<'i, E: ParseError<&'i [u8]>>(
//     i: &'i [u8],
// ) -> IResult<&'i [u8], Sequence<ItemRef<'i>>, E> {
//     delimited(
//         nchar::char('['),
//         multi::many0(parse_item).map(Sequence),
//         nchar::char(']'),
//     )
//     .parse(i)
// }

pub fn parse_unit<'i, E: ParseError<&'i [u8]>>(i: &'i [u8]) -> IResult<&'i [u8], (), E> {
    nom::combinator::value((), bytes::tag("[]")).parse(i)
}

// pub fn parse_record<'i, Label, Entry, E: ParseError<&'i [u8]>>(
//     label_p: impl Parser<&'i [u8], Label, E>,
//     entry_p: impl Parser<&'i [u8], Entry, E>,
// ) -> impl Parser<&'i [u8], Record<Label, Entry>, E> {
//     delimited(
//         nchar::char('<'),
//         label_p
//             .and(multi::many0(entry_p))
//             .map(|(label, entries)| Record(label, entries)),
//         nchar::char('>'),
//     )
// }

// pub fn parse_unit_record<'i, Label, E: ParseError<&'i [u8]>>(
//     label_p: impl Parser<&'i [u8], Label, E>,
// ) -> impl Parser<&'i [u8], Record<Label, ()>, E> {
//     delimited(
//         nchar::char('<'),
//         label_p.map(|label| Record(label, vec![])),
//         nchar::char('>'),
//     )
// }

// pub fn parse_record_ref<'i, E: ParseError<&'i [u8]>>(
//     i: &'i [u8],
// ) -> IResult<&'i [u8], Record<Box<ItemRef<'i>>, ItemRef<'i>>, E> {
//     parse_record(parse_item.map(Box::new), parse_item).parse(i)
//     // delimited(
//     //     nchar::char('<'),
//     //     parse_item
//     //         .and(multi::many0(parse_item))
//     //         .map(|(label, items)| Record(Box::new(label), items)),
//     //     nchar::char('>'),
//     // )
//     // .parse(i)
// }

// pub fn parse_set<'i, E: ParseError<&'i [u8]>>(
//     i: &'i [u8],
// ) -> IResult<&'i [u8], Set<ItemRef<'i>>, E> {
//     delimited(
//         nchar::char('#'),
//         multi::many0(parse_item).map(Set),
//         nchar::char('$'),
//     )
//     .parse(i)
// }
