use std::io::{Cursor, Read};

use serde::de::{
    DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use serde::Deserializer;

use crate::binary::{Error, Result};

pub struct BinaryDeserializer<'de> {
    pub cursor: Cursor<&'de [u8]>,
}

impl<'de> BinaryDeserializer<'de> {
    pub fn new(slice: &'de [u8]) -> Self {
        BinaryDeserializer {
            cursor: Cursor::new(slice),
        }
    }
    pub fn read_fixed<const C: usize>(&mut self) -> Result<[u8; C]> {
        let mut buf = [0u8; C];
        self.cursor.read_exact(&mut buf)?;
        Ok(buf)
    }
    pub fn read_exact(&mut self, slice: &mut [u8]) -> Result<()> {
        Ok(self.cursor.read_exact(slice)?)
    }
}

impl<'a, 'de> EnumAccess<'de> for &'a mut BinaryDeserializer<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        let index: u32 = u32::from_le_bytes(self.read_fixed()?);
        let val = DeserializeSeed::deserialize(
            seed,
            IntoDeserializer::<Error>::into_deserializer(index),
        )?;
        Ok((val, self))
    }
}

impl<'a, 'de> VariantAccess<'de> for &'a mut BinaryDeserializer<'de> {
    type Error = Error;
    fn unit_variant(self) -> Result<()> { Ok(()) }
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        DeserializeSeed::deserialize(seed, self)
    }
    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }
    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(fields.len(), visitor)
    }
}

impl<'a, 'de> Deserializer<'de> for &'a mut BinaryDeserializer<'de> {
    type Error = Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Unsupported)
    }
    fn deserialize_bool<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(u8::from_le_bytes(self.read_fixed()?) != 0)
    }
    fn deserialize_i8<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(i8::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_i16<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(i16::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_i32<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(i32::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_i64<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(i64::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_i128<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i128(i128::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_u8<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(u8::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_u16<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(u16::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_u32<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(u32::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_u64<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(u64::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_u128<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u128(u128::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_f32<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(f32::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_f64<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(f64::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_char<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_char(
            char::from_u32(u32::from_le_bytes(self.read_fixed()?)).ok_or(Error::BadChar)?,
        )
    }
    fn deserialize_str<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }
    fn deserialize_string<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let length = u64::from_le_bytes(self.read_fixed()?);
        let mut vec = vec![0; length as usize];
        self.cursor.read_exact(&mut vec)?;
        visitor.visit_string(String::from_utf8(vec)?)
    }
    fn deserialize_bytes<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }
    fn deserialize_byte_buf<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let length = u64::from_le_bytes(self.read_fixed()?);
        let mut vec = vec![0; length as usize];
        self.cursor.read_exact(&mut vec)?;
        visitor.visit_byte_buf(vec)
    }
    fn deserialize_option<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if u8::from_le_bytes(self.read_fixed()?) != 0 {
            visitor.visit_some(self)
        } else {
            visitor.visit_none()
        }
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let length = u64::from_le_bytes(self.read_fixed()?);
        self.deserialize_tuple(length as usize, visitor)
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        struct Access<'a, 'de> {
            de: &'a mut BinaryDeserializer<'de>,
            len: usize,
        }
        impl<'a, 'de> SeqAccess<'de> for Access<'a, 'de> {
            type Error = Error;
            fn next_element_seed<T: DeserializeSeed<'de>>(
                &mut self,
                seed: T,
            ) -> Result<Option<T::Value>> {
                if self.len > 0 {
                    self.len -= 1;
                    let value = seed.deserialize(&mut *self.de)?;
                    Ok(Some(value))
                } else {
                    Ok(None)
                }
            }
            fn size_hint(&self) -> Option<usize> { Some(self.len) }
        }
        visitor.visit_seq(Access { de: self, len })
    }
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let len = u64::from_le_bytes(self.read_fixed()?) as usize;
        struct Access<'a, 'de> {
            de: &'a mut BinaryDeserializer<'de>,
            len: usize,
        }
        impl<'a, 'de> MapAccess<'de> for Access<'a, 'de> {
            type Error = Error;
            fn size_hint(&self) -> Option<usize> { Some(self.len) }
            fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
            where
                K: DeserializeSeed<'de>,
            {
                if self.len > 0 {
                    self.len -= 1;
                    let key = seed.deserialize(&mut *self.de)?;
                    Ok(Some(key))
                } else {
                    Ok(None)
                }
            }
            fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
            where
                V: DeserializeSeed<'de>,
            {
                seed.deserialize(&mut *self.de)
            }
        }
        visitor.visit_map(Access { de: self, len })
    }
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
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
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(self)
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Unsupported)
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Unsupported)
    }
}
