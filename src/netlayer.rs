//! - [Draft Specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Netlayers.md)

#[cfg(feature = "netlayer-onion")]
pub mod onion;

pub trait Netlayer {
    fn connect();
    fn accept();
}
