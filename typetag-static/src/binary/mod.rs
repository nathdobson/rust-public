use std::any::TypeId;
use std::f32::consts::E;
use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::io::{Cursor, ErrorKind, Read};
use std::ops::Range;
use std::string::FromUtf8Error;

pub use any::IMPLS;
use serde::de::{
    DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use serde::ser::{
    SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::binary::de::BinaryDeserializer;
use crate::binary::ser::BinarySerializer;
use crate::tag::{TypeTag, TypeTagHash};

pub mod any;
mod de;
pub mod ser;

#[derive(Debug, Serialize, Deserialize)]
pub enum Error {
    Custom(String),
    Io(#[serde(skip)] Option<io::Error>),
    FromUtf8(#[serde(skip)] Option<FromUtf8Error>),
    BadChar,
    Unsupported,
    MissingSerialize(String),
    BadType,
    BadLength,
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Error::Custom(x) => Error::Custom(x.clone()),
            Error::Io(_) => Error::Io(None),
            Error::FromUtf8(x) => Error::FromUtf8(x.clone()),
            Error::BadChar => Error::BadChar,
            Error::Unsupported => Error::Unsupported,
            Error::MissingSerialize(x) => Error::MissingSerialize(x.clone()),
            Error::BadType => Error::BadType,
            Error::BadLength => Error::BadLength,
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

/// A struct created by [`AnySerde`](crate::AnySerde) when deserializing a binary value with
/// an unrecognized tag. Ensures that such values can safely be re-serialized without losing data.
#[derive(Clone, Debug)]
pub struct UnknownBinary {
    pub(in crate::binary) tag: TypeTagHash,
    pub(in crate::binary) content: Vec<u8>,
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Custom(e) => write!(f, "{}", e),
            Error::Io(None) => write!(f, "io error"),
            Error::Io(Some(e)) => write!(f, "io error: {}", e),
            Error::BadChar => write!(f, "Char not in unicode range."),
            Error::FromUtf8(None) => write!(f, "UTF8 error"),
            Error::FromUtf8(Some(e)) => write!(f, "UTF8 error: {}", e),
            Error::Unsupported => write!(f, "Unsupported operation"),
            Error::MissingSerialize(id) => {
                write!(f, "Missing `impl_any_binary!` for type with {:?}", id)
            }
            Error::BadType => write!(f, "Bad AnySerialize"),
            Error::BadLength => write!(f, "Bad length"),
        }
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::Custom(format!("{}", msg))
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::Custom(format!("{}", msg))
    }
}

impl From<io::Error> for Error {
    fn from(ioe: io::Error) -> Self { Error::Io(Some(ioe)) }
}

impl From<FromUtf8Error> for Error {
    fn from(fue: FromUtf8Error) -> Self { Error::FromUtf8(Some(fue)) }
}

impl From<Error> for io::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::Custom(e) => io::Error::new(ErrorKind::Other, e),
            Error::Io(Some(e)) => e,
            Error::Io(None) => io::Error::new(ErrorKind::Other, "io error"),
            Error::FromUtf8(Some(e)) => io::Error::new(ErrorKind::Other, e),
            Error::FromUtf8(None) => io::Error::new(ErrorKind::Other, "utf8 error"),
            Error::MissingSerialize(_)
            | Error::BadChar
            | Error::Unsupported
            | Error::BadType
            | Error::BadLength => io::Error::new(ErrorKind::Other, format!("{}", e)),
        }
    }
}

pub fn serialize<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>> {
    let mut vec = vec![];
    value.serialize(BinarySerializer::new(&mut vec))?;
    Ok(vec)
}

pub fn serialize_into<T: Serialize>(output: &mut Vec<u8>, value: &T) -> Result<()> {
    value.serialize(BinarySerializer::new(output))?;
    Ok(())
}

pub fn deserialize<'de, T: Deserialize<'de>>(slice: &'de [u8]) -> Result<T> {
    T::deserialize(&mut BinaryDeserializer::new(slice))
}
