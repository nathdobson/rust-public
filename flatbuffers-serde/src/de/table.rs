use flatbuffers::{Follow, ForwardsUOffset, Table};
use serde::de::value::U16Deserializer;
use serde::de::{
    DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor,
};

use crate::de::error::Error;
use crate::de::field::FieldDeserializer;
use crate::de::identity::IdentityDeserializer;
use crate::de::wrapper::{Deserializer, FlatDeserializer};
use crate::flat_util::VariantT;

#[derive(Debug)]
pub struct TableDeserializer<'de> {
    table: Table<'de>,
    index: usize,
}

impl<'de> Follow<'de> for TableDeserializer<'de> {
    type Inner = Self;
    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        TableDeserializer {
            table: Table::follow(buf, loc),
            index: 0,
        }
    }
}

impl<'a, 'de> EnumAccess<'de> for &'a mut TableDeserializer<'de> {
    type Error = Error;
    type Variant = &'a mut TableDeserializer<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = self.deserialize_fixed::<VariantT>().unwrap_or(0);
        let de: U16Deserializer<Error> = variant.into_deserializer();
        let variant = seed.deserialize(de)?;
        Ok((variant, self))
    }
}

impl<'a, 'de> VariantAccess<'de> for &'a mut TableDeserializer<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        self.index += 1;
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let deserializer = self.deserialize_fixed::<FieldDeserializer>();
        let mut deserializer = deserializer.ok_or(Error::MissingEnumValue)?;
        seed.deserialize(Deserializer::new(deserializer))
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let deserializer = self.deserialize_fixed::<ForwardsUOffset<TableDeserializer>>();
        let mut deserializer = deserializer.ok_or(Error::MissingEnumValue)?;
        visitor.visit_seq(&mut deserializer)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.tuple_variant(fields.len(), visitor)
    }
}

impl<'de> SeqAccess<'de> for TableDeserializer<'de> {
    type Error = Error;
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        println!("next_element_seed {:?}", self.index);
        Ok(Some(seed.deserialize(Deserializer::new(self))?))
    }
}

impl<'a, 'de> FlatDeserializer<'de> for &'a mut TableDeserializer<'de> {
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if let Some(deserializer) =
            self.deserialize_fixed::<ForwardsUOffset<IdentityDeserializer>>()
        {
            visitor.visit_some(Deserializer::new(deserializer))
        } else {
            visitor.visit_none()
        }
    }

    fn deserialize_fixed<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        println!("deserialize_value({:?})", self);
        let result = self.table.get::<T>((self.index * 2 + 4) as u16, None);
        self.index += 1;
        result
    }

    fn deserialize_variable<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        println!("deserialize_value({:?})", self);
        let result = self
            .table
            .get::<ForwardsUOffset<T>>((self.index * 2 + 4) as u16, None);
        self.index += 1;
        result
    }
    fn deserialize_enum<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_enum(self)
    }
}
