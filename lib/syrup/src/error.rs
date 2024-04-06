#[derive(Debug, PartialEq)]
pub enum ParseErrorKind {
    Nom(nom::error::ErrorKind),
    OutOfBounds,
}

impl From<nom::error::ErrorKind> for ParseErrorKind {
    fn from(value: nom::error::ErrorKind) -> Self {
        Self::Nom(value)
    }
}

#[derive(Debug, PartialEq)]
pub enum ErrorKind {
    Parse(ParseErrorKind),
    Incomplete(nom::Needed),
}

impl From<nom::error::ErrorKind> for ErrorKind {
    fn from(value: nom::error::ErrorKind) -> Self {
        Self::Parse(value.into())
    }
}

#[derive(thiserror::Error, PartialEq)]
pub struct Error<'input> {
    pub input: Option<&'input [u8]>,
    pub kind: ErrorKind,
    // #[backtrace]
    // backtrace: std::backtrace::Backtrace,
}

impl<'input> std::fmt::Display for Error<'input> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "syrup error: {:?}, input: {:?}",
            &self.kind,
            self.input.map(String::from_utf8_lossy)
        )
    }
}

impl<'input> std::fmt::Debug for Error<'input> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = f.debug_struct("Error");
        match self.input.map(std::str::from_utf8) {
            Some(Ok(s)) => res.field("input", &s),
            _ => res.field("input", &self.input),
        }
        .field("kind", &self.kind)
        .finish()
    }
}

impl<'input> nom::error::ParseError<&'input [u8]> for Error<'input> {
    fn from_error_kind(input: &'input [u8], kind: nom::error::ErrorKind) -> Self {
        Self {
            input: Some(input),
            kind: kind.into(),
        }
    }

    fn append(_input: &'input [u8], _kind: nom::error::ErrorKind, _other: Self) -> Self {
        todo!()
    }
}

impl<'input> From<nom::Err<Error<'input>>> for Error<'input> {
    fn from(value: nom::Err<Error<'input>>) -> Self {
        match value {
            nom::Err::Incomplete(n) => Self {
                input: None,
                kind: ErrorKind::Incomplete(n),
            },
            nom::Err::Error(e) | nom::Err::Failure(e) => e,
        }
    }
}

impl<'input> nom::error::FromExternalError<&'input [u8], ibig::error::OutOfBoundsError>
    for Error<'input>
{
    fn from_external_error(
        input: &'input [u8],
        kind: nom::error::ErrorKind,
        _e: ibig::error::OutOfBoundsError,
    ) -> Self {
        Self {
            input: Some(input),
            kind: kind.into(),
        }
    }
}

impl<'input> crate::de::DeserializeError for Error<'input> {
    fn needed(&self) -> Option<nom::Needed> {
        match self.kind {
            ErrorKind::Incomplete(n) => Some(n),
            ErrorKind::Parse(_) => None,
        }
    }
}
