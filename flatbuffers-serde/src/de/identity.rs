use flatbuffers::{Follow, ForwardsUOffset};
use serde::de::Visitor;

use crate::de::error::Error;
use crate::de::table::TableDeserializer;
use crate::de::wrapper::{Deserializer, FlatDeserializer};
use crate::flat_util::FollowOrNull;

#[derive(Debug, Copy, Clone)]
pub struct IdentityDeserializer<'de> {
    buf: &'de [u8],
    loc: usize,
}

impl<'de> IdentityDeserializer<'de> {
    fn follow<T: Follow<'de>>(&self) -> T::Inner {
        T::follow(self.buf, self.loc)
    }
}

impl<'de> Follow<'de> for IdentityDeserializer<'de> {
    type Inner = Self;
    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        IdentityDeserializer {
            buf,
            loc,
        }
    }
}

impl<'de> FlatDeserializer<'de> for IdentityDeserializer<'de> {
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if let Some(deserializer)
        = self.follow::<FollowOrNull<ForwardsUOffset<Deserializer<IdentityDeserializer>>>>() {
            visitor.visit_some(deserializer)
        } else {
            visitor.visit_none()
        }
    }
    fn deserialize_fixed<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        Some(self.follow::<T>())
    }
    fn deserialize_variable<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        Some(self.follow::<T>())
    }
    fn deserialize_enum<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        println!("deserialize_enum({:?})", self);
        self.follow::<TableDeserializer>().deserialize_enum(visitor)
    }
}
