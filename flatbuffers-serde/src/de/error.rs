use core::fmt::{Display, Formatter};

use flatbuffers::InvalidFlatbuffer;

use crate::any_generated::{TypeMismatch, TypeTagHash};
use crate::tag::TypeTag;

#[derive(Debug)]
pub enum Error {
    Custom(String),
    BadChar,
    Unsupported,
    MissingEnumValue,
    MissingTypeTagHash,
    MissingData,
    TypeMismatch(TypeMismatch),
}

impl From<TypeMismatch> for Error {
    fn from(x: TypeMismatch) -> Self { Error::TypeMismatch(x) }
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Custom(x) => write!(f, "custom deserializer error: {}", x),
            Error::BadChar => write!(f, "bad char value"),
            Error::Unsupported => write!(f, "unsupported operation"),
            Error::MissingEnumValue => write!(f, "enum tag was present, but value was not"),
            Error::TypeMismatch(x) => write!(f, "{}", x),
            Error::MissingTypeTagHash => write!(f, "Missing AnyFlat::type_tag_hash field"),
            Error::MissingData => write!(f, "Missing AnyFlat::data field"),
        }
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::Custom(msg.to_string())
    }
}
