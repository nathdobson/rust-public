use crate::ser::error::Error;
use serde::Serialize;
use flatbuffers::{FlatBufferBuilder, WIPOffset, UnionWIPOffset};
use crate::ser::wrapper::{Stack, Serializer};

pub mod wrapper;
pub mod value;
pub mod enu;
pub mod error;
pub mod vector;
pub mod map;
pub mod table;

pub type Result<T> = std::result::Result<T, Error>;

pub fn serialize_raw<'a, 'b, T: Serialize>(
    fbb: &'b mut FlatBufferBuilder<'a>,
    value: &T,
) -> crate::ser::Result<WIPOffset<UnionWIPOffset>> {
    let mut stack = Stack::new();
    let mut serializer = Serializer::new(fbb, &mut stack);
    let value = serializer.serialize_to_offset(value)?;
    Ok(WIPOffset::new(value.value()))
}
