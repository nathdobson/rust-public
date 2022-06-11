use std::mem::size_of;

use flatbuffers::{Follow, ForwardsUOffset, UOffsetT};
use serde::de::{DeserializeSeed, SeqAccess, Visitor};

use crate::de::error::Error;
use crate::de::identity::IdentityDeserializer;
use crate::de::wrapper::{Deserializer, FlatDeserializer};
use crate::flat_util::FollowOrNull;

#[derive(Debug)]
pub struct VectorDeserializer<'de> {
    buf: &'de [u8],
    loc: usize,
    len: usize,
}

impl<'de> Follow<'de> for VectorDeserializer<'de> {
    type Inner = VectorDeserializer<'de>;
    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        let len = UOffsetT::follow(buf, loc);
        let loc = loc + size_of::<UOffsetT>();
        VectorDeserializer {
            buf,
            loc,
            len: len as usize,
        }
    }
}

impl<'de> VectorDeserializer<'de> {
    pub fn next<T: Follow<'de>>(&mut self) -> Option<T::Inner> {
        if self.len == 0 {
            return None;
        }
        let result = T::follow(self.buf, self.loc);
        self.loc += size_of::<T>();
        self.len -= 1;
        Some(result)
    }
}

impl<'a, 'de> FlatDeserializer<'de> for &'a mut VectorDeserializer<'de> {
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        println!("deserialize_option({:?})", self);
        let deserializer = self
            .deserialize_fixed::<FollowOrNull<ForwardsUOffset<IdentityDeserializer>>>()
            .unwrap();
        if let Some(deserializer) = deserializer {
            visitor.visit_some(Deserializer::new(deserializer))
        } else {
            visitor.visit_none()
        }
    }

    fn deserialize_fixed<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        let result = self.next::<T>().unwrap();
        Some(result)
    }
    fn deserialize_variable<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        self.deserialize_fixed::<ForwardsUOffset<T>>()
    }
    fn deserialize_enum<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_fixed::<ForwardsUOffset<IdentityDeserializer>>()
            .unwrap()
            .deserialize_enum(visitor)
    }
}

impl<'de> SeqAccess<'de> for VectorDeserializer<'de> {
    type Error = Error;
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.len == 0 {
            Ok(None)
        } else {
            Ok(Some(seed.deserialize(Deserializer::new(self))?))
        }
    }
}
