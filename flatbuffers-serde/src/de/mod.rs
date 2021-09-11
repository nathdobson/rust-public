use crate::de::error::Error;
use serde::Deserialize;
use crate::de::wrapper::Deserializer;
use crate::de::identity::IdentityDeserializer;
use flatbuffers::Follow;

pub mod error;
pub mod identity;
pub mod field;
pub mod none;
pub mod table;
pub mod wrapper;
pub mod vector;
pub mod map;

pub type Result<T> = std::result::Result<T, Error>;

pub fn deserialize_raw<'a, T: Deserialize<'a>>(buf: &'a [u8], loc: usize) -> crate::de::Result<T> {
    T::deserialize(Deserializer::<IdentityDeserializer>::follow(buf, loc))
}
