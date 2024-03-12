use syn::{Expr, Path, Type};

#[derive(Clone)]
pub(crate) enum Conversion {
    Infallible(Type),
    Fallible(Type),
}

#[derive(Clone)]
pub(crate) enum With {
    Conversion(Conversion),
    Custom(Path),
    Verbatim(Expr),
    Optional,
}

impl With {
    #[inline]
    pub(crate) fn infallible(ty: Type) -> Self {
        Self::Conversion(Conversion::Infallible(ty))
    }
    #[inline]
    pub(crate) fn fallible(ty: Type) -> Self {
        Self::Conversion(Conversion::Fallible(ty))
    }
}
