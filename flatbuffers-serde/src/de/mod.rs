use flatbuffers::Follow;
use serde::Deserialize;

use crate::de::error::Error;
use crate::de::identity::IdentityDeserializer;
use crate::de::wrapper::Deserializer;

pub mod error;
pub mod field;
pub mod identity;
pub mod map;
pub mod none;
pub mod table;
pub mod vector;
pub mod wrapper;

pub type Result<T> = std::result::Result<T, Error>;

pub fn deserialize_raw<'a, T: Deserialize<'a>>(buf: &'a [u8], loc: usize) -> crate::de::Result<T> {
    T::deserialize(Deserializer::<IdentityDeserializer>::follow(buf, loc))
}
