use crate::de::vector::VectorDeserializer;
use crate::de::table::TableDeserializer;
use serde::de::{MapAccess, DeserializeSeed};
use crate::de::error::Error;
use crate::de::wrapper::Deserializer;
use flatbuffers::{Follow, ForwardsUOffset};

#[derive(Debug)]
pub struct MapDeserializer<'de> {
    vector: VectorDeserializer<'de>,
    entry: Option<TableDeserializer<'de>>,
}

impl<'de> Follow<'de> for MapDeserializer<'de> {
    type Inner = Self;
    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        println!("Deserializing map {:?}", loc);
        MapDeserializer { vector: VectorDeserializer::follow(buf, loc), entry: None }
    }
}

impl<'a, 'de> MapAccess<'de> for &'a mut MapDeserializer<'de> {
    type Error = Error;
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> where K: DeserializeSeed<'de> {
        let mut entry = self.vector.next::<ForwardsUOffset<TableDeserializer>>();
        if let Some(mut entry) = entry {
            let key = seed.deserialize(Deserializer::new(&mut entry))?;
            self.entry = Some(entry);
            Ok(Some(key))
        } else {
            println!("No entry ");
            Ok(None)
        }
    }
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error> where V: DeserializeSeed<'de> {
        seed.deserialize(Deserializer::new(&mut self.entry.take().unwrap()))
    }
}
