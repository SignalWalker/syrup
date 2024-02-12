use ibig::IBig;

// use crate::Symbol;
// use ibig::IBig;
//
// pub enum Token {
//     /// t
//     True,
//     /// f
//     False,
//     /// F
//     F32Start,
//     /// D
//     F64Start,
//     /// +
//     Plus,
//     /// -
//     Minus,
//     /// {
//     DictionaryOpen,
//     /// }
//     DictionaryClose,
//     /// [
//     SequenceOpen,
//     /// ]
//     SequenceClose,
//     /// <
//     RecordOpen,
//     /// >
//     RecordClose,
// }
//
// macro_rules! Token {
//     ('t') => {
//         Token::True
//     };
//     ('f') => {
//         Token::False
//     };
//     ('F') => {
//         Token::F32Start
//     };
//     ('D') => {
//         Token::F64Start
//     };
//     ('+') => {
//         Token::Plus
//     };
//     ('-') => {
//         Token::Minus
//     };
//     ('{') => {
//         Token::DictionaryOpen
//     };
//     ('}') => {
//         Token::DictionaryClose
//     };
//     ('[') => {
//         Token::SequenceOpen
//     };
//     (']') => {
//         Token::SequenceClose
//     };
//     ('<') => {
//         Token::RecordOpen
//     };
//     ('>') => {
//         Token::RecordClose
//     };
// }
//
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralOwned {
    Bool(bool),
    F32(f32),
    F64(f64),
    Int(IBig),
    String(String),
    Symbol(String),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal<'data> {
    Bool(bool),
    F32(f32),
    F64(f64),
    Int(IBig),
    String(&'data str),
    Symbol(&'data str),
    Bytes(&'data [u8]),
}

impl From<Literal<'_>> for LiteralOwned {
    fn from(value: Literal<'_>) -> Self {
        match value {
            Literal::Bool(b) => Self::Bool(b),
            Literal::F32(f) => Self::F32(f),
            Literal::F64(d) => Self::F64(d),
            Literal::Int(i) => Self::Int(i),
            Literal::String(s) => Self::String(s.to_owned()),
            Literal::Symbol(s) => Self::Symbol(s.to_owned()),
            Literal::Bytes(b) => Self::Bytes(b.to_owned()),
        }
    }
}

/// An iterator over tokens in an input byte slice.
pub struct ItemStream<'input> {
    input: &'input [u8],
}

impl<'input> Iterator for ItemStream<'input> {
    type Item = Literal<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
