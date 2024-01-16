use syn::{Path, Type};

#[derive(Clone)]
pub enum Conversion {
    Infallible(Type),
    Fallible(Type),
}

#[derive(Clone)]
pub enum With {
    Conversion(Conversion),
    Custom(Path),
}

impl With {
    #[inline]
    pub fn infallible(ty: Type) -> Self {
        Self::Conversion(Conversion::Infallible(ty))
    }
    #[inline]
    pub fn fallible(ty: Type) -> Self {
        Self::Conversion(Conversion::Fallible(ty))
    }
}
