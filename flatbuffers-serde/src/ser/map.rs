use serde::ser::{SerializeMap, SerializeSeq};
use serde::Serialize;

use crate::ser;
use crate::ser::error;
use crate::ser::error::Error;
use crate::ser::value::{OneValue, Value};
use crate::ser::vector::VectorBuilder;
use crate::ser::wrapper::Serializer;
use crate::ser::Result;

pub struct MapBuilder<'a, 'b> {
    vector: VectorBuilder<'a, 'b>,
    key: Option<Value>,
}

impl<'a, 'b> MapBuilder<'a, 'b> {
    pub fn new(serializer: Serializer<'a, 'b>) -> Self {
        MapBuilder { vector: VectorBuilder::new(serializer), key: None }
    }
}

impl<'a, 'b> SerializeMap for MapBuilder<'a, 'b> {
    type Ok = Value;
    type Error = Error;
    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()> where T: Serialize {
        self.key = Some(key.serialize(self.vector.reborrow())?);
        Ok(())
    }
    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        let key = self.key.take().unwrap();
        let value = value.serialize(self.vector.reborrow())?;
        let mut table = self.vector.reborrow().start_table();
        table.push(key);
        table.push(value);
        let table = table.end_table();
        self.vector.push(OneValue::Ref(table));
        Ok(())
    }
    fn end(self) -> Result<Value> {
        self.vector.end()
    }
}
