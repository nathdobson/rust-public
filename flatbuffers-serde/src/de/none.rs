use flatbuffers::Follow;
use serde::de::value::U32Deserializer;
use serde::de::{
    DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor,
};

use crate::de::error::Error;
use crate::de::wrapper::{Deserializer, FlatDeserializer};

#[derive(Debug)]
pub struct NoneDeserializer;

#[derive(Debug)]
pub struct RepeatNoneDeserializer;

impl<'de> FlatDeserializer<'de> for NoneDeserializer {
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_none()
    }

    fn deserialize_fixed<T: Follow<'de> + 'de>(self) -> Option<T::Inner> { None }
    fn deserialize_variable<T: Follow<'de> + 'de>(self) -> Option<T::Inner> { None }
    fn deserialize_enum<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        struct NoneEnumAccess;
        struct NoneVariantAccess;
        impl<'de> EnumAccess<'de> for NoneEnumAccess {
            type Error = Error;
            type Variant = NoneVariantAccess;

            fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
            where
                V: DeserializeSeed<'de>,
            {
                let de: U32Deserializer<Error> = 0u32.into_deserializer();
                Ok((seed.deserialize(de)?, NoneVariantAccess))
            }
        }
        impl<'de> VariantAccess<'de> for NoneVariantAccess {
            type Error = Error;

            fn unit_variant(self) -> Result<(), Self::Error> { Ok(()) }

            fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
            where
                T: DeserializeSeed<'de>,
            {
                seed.deserialize(Deserializer::new(NoneDeserializer))
            }

            fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                visitor.visit_seq(RepeatNoneDeserializer)
            }

            fn struct_variant<V>(
                self,
                fields: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                visitor.visit_seq(RepeatNoneDeserializer)
            }
        }
        visitor.visit_enum(NoneEnumAccess)
    }
}

impl<'de> SeqAccess<'de> for NoneDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        Ok(None)
    }
}

impl<'de> SeqAccess<'de> for RepeatNoneDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        Ok(Some(seed.deserialize(Deserializer::new(NoneDeserializer))?))
    }
}

impl<'de> MapAccess<'de> for NoneDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        Ok(None)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        Err(Error::Unsupported)
    }
}
