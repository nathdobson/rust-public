use std::fmt::Debug;

use flatbuffers::Follow;
use serde::de::Visitor;

use crate::de::error::Error;
use crate::de::map::MapDeserializer;
use crate::de::none::{NoneDeserializer, RepeatNoneDeserializer};
use crate::de::table::TableDeserializer;
use crate::de::vector::VectorDeserializer;
use crate::flat_util::{Flat128, FlatUnit};

pub trait FlatDeserializer<'de>: Debug {
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error>;
    fn deserialize_fixed<T: Follow<'de> + 'de>(self) -> Option<T::Inner>;
    fn deserialize_variable<T: Follow<'de> + 'de>(self) -> Option<T::Inner>;
    fn deserialize_enum<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error>;
}

#[derive(Copy, Clone, Debug)]
pub struct Deserializer<I> {
    imp: I,
}

impl<I> Deserializer<I> {
    pub fn new(imp: I) -> Self { Deserializer { imp } }
}

impl<'de, I: Follow<'de>> Follow<'de> for Deserializer<I> {
    type Inner = Deserializer<I::Inner>;
    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        Deserializer {
            imp: I::follow(buf, loc),
        }
    }
}

impl<'de, I> serde::Deserializer<'de> for Deserializer<I>
where
    I: FlatDeserializer<'de>,
{
    type Error = Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::Unsupported)
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.imp.deserialize_fixed::<bool>().unwrap_or_default())
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.imp.deserialize_fixed::<i8>().unwrap_or_default())
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.imp.deserialize_fixed::<i16>().unwrap_or_default())
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.imp.deserialize_fixed::<i32>().unwrap_or_default())
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.imp.deserialize_fixed::<i64>().unwrap_or_default())
    }
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i128(self.imp.deserialize_fixed::<Flat128>().unwrap_or_default() as i128)
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.imp.deserialize_fixed::<u8>().unwrap_or_default())
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.imp.deserialize_fixed::<u16>().unwrap_or_default())
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.imp.deserialize_fixed::<u32>().unwrap_or_default())
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.imp.deserialize_fixed::<u64>().unwrap_or_default())
    }
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u128(self.imp.deserialize_fixed::<Flat128>().unwrap_or_default())
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.imp.deserialize_fixed::<f32>().unwrap_or_default())
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.imp.deserialize_fixed::<f64>().unwrap_or_default())
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_char(
            std::char::from_u32(self.imp.deserialize_fixed::<u32>().unwrap_or_default())
                .ok_or(Error::BadChar)?,
        )
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.imp.deserialize_variable::<&str>().unwrap_or_default())
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self.imp.deserialize_variable::<&[u8]>().unwrap_or_default())
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.imp.deserialize_option(visitor)
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.imp.deserialize_fixed::<FlatUnit>();
        visitor.visit_unit()
    }
    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.imp.deserialize_fixed::<FlatUnit>();
        visitor.visit_unit()
    }
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut deserializer = self.imp.deserialize_variable::<VectorDeserializer>();
        if let Some(mut deserializer) = deserializer {
            visitor.visit_seq(deserializer)
        } else {
            visitor.visit_seq(NoneDeserializer)
        }
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut deserializer = self.imp.deserialize_variable::<TableDeserializer>();
        if let Some(mut deserializer) = deserializer {
            visitor.visit_seq(deserializer)
        } else {
            visitor.visit_seq(RepeatNoneDeserializer)
        }
    }
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut deserializer = self.imp.deserialize_variable::<MapDeserializer>();
        if let Some(mut deserializer) = deserializer {
            visitor.visit_map(&mut deserializer)
        } else {
            visitor.visit_map(NoneDeserializer)
        }
    }
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(fields.len(), visitor)
    }
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.imp.deserialize_enum(visitor)
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::Unsupported)
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::Unsupported)
    }
}
