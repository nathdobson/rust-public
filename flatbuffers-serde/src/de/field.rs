use flatbuffers::{Follow, ForwardsUOffset};
use serde::de::Visitor;

use crate::de::error::Error;
use crate::de::identity::IdentityDeserializer;
use crate::de::wrapper::FlatDeserializer;

#[derive(Debug)]
pub struct FieldDeserializer<'de> {
    buf: &'de [u8],
    loc: usize,
}

impl<'de> FieldDeserializer<'de> {
    fn follow<T: Follow<'de>>(&self) -> T::Inner { T::follow(self.buf, self.loc) }
}

impl<'de> Follow<'de> for FieldDeserializer<'de> {
    type Inner = Self;
    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner { FieldDeserializer { buf, loc } }
}

impl<'a, 'de> FlatDeserializer<'de> for FieldDeserializer<'de> {
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.follow::<ForwardsUOffset<IdentityDeserializer>>()
            .deserialize_option(visitor)
    }

    fn deserialize_fixed<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        Some(self.follow::<T>())
    }

    fn deserialize_variable<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        Some(self.follow::<ForwardsUOffset<T>>())
    }
    fn deserialize_enum<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.follow::<ForwardsUOffset<IdentityDeserializer>>()
            .deserialize_enum(visitor)
    }
}
