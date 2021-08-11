mod de;
pub mod ser;
pub mod any;

use serde::{Serializer, Serialize, Deserializer, Deserialize};
use std::fmt::{Display, Debug, Formatter};
use serde::ser::{SerializeSeq, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant, SerializeMap, SerializeStruct, SerializeStructVariant};
use std::ops::Range;
use std::io::{Cursor, Read};
use serde::de::{Visitor, SeqAccess, MapAccess, EnumAccess, IntoDeserializer, VariantAccess};
use std::io;
use std::string::FromUtf8Error;
use serde::de::DeserializeSeed;
use crate::binary::Error::Unsupported;
use crate::tag::{TypeTag, TypeTagHash};
use crate::binary::ser::BinarySerializer;
use crate::binary::de::BinaryDeserializer;
use std::any::TypeId;

#[derive(Debug)]
pub enum Error {
    Custom(String),
    Io(io::Error),
    FromUtf8(FromUtf8Error),
    BadChar,
    Unsupported,
    MissingSerialize(TypeId),
    BadType,
}

type Result<T> = std::result::Result<T, Error>;

/// A struct created by [`AnySerde`](crate::AnySerde) when deserializing a binary value with
/// an unrecognized tag. Ensures that such values can safely be re-serialized without losing data.
#[derive(Clone)]
pub struct UnknownBinary {
    pub(in crate::binary) tag: TypeTagHash,
    pub(in crate::binary) content: Vec<u8>,
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Custom(e) => write!(f, "{}", e),
            Error::Io(e) => write!(f, "{}", e),
            Error::BadChar => write!(f, "Char not in unicode range."),
            Error::FromUtf8(e) => write!(f, "{}", e),
            Error::Unsupported => write!(f, "Unsupported operation"),
            Error::MissingSerialize(id) => write!(f, "Missing `impl_any_binary!` for type with {:?}", id),
            Error::BadType => write!(f, "Bad AnySerialize"),
        }
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self where T: Display {
        Error::Custom(format!("{}", msg))
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self where T: Display {
        Error::Custom(format!("{}", msg))
    }
}


impl From<io::Error> for Error {
    fn from(ioe: io::Error) -> Self { Error::Io(ioe) }
}

impl From<FromUtf8Error> for Error {
    fn from(fue: FromUtf8Error) -> Self { Error::FromUtf8(fue) }
}

pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut vec = vec![];
    value.serialize(BinarySerializer::new(&mut vec))?;
    Ok(vec)
}

pub fn deserialize<'de, T: Deserialize<'de>>(slice: &'de [u8]) -> Result<T> {
    T::deserialize(&mut BinaryDeserializer::new(slice))
}