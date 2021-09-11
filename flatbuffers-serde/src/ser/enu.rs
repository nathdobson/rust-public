use serde::ser::{SerializeStructVariant, SerializeTupleVariant};
use serde::Serialize;

use crate::ser;
use crate::ser::error;
use crate::ser::error::Error;
use crate::ser::table::TableBuilder;
use crate::ser::value::{OneValue, Value};
use crate::ser::wrapper::Serializer;
use crate::ser::Result;

pub struct EnumBuilder<'a, 'b> {
    table: TableBuilder<'a, 'b>,
    variant: u16,
}

impl<'a, 'b> EnumBuilder<'a, 'b> {
    pub fn new(serializer: Serializer<'a, 'b>, variant: u16) -> Self {
        EnumBuilder { table: TableBuilder::new(serializer), variant }
    }
}

impl<'a, 'b> SerializeTupleVariant for EnumBuilder<'a, 'b> {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        let value = value.serialize(self.table.reborrow())?;
        self.table.push(value);
        Ok(())
    }
    fn end(self) -> Result<Value> {
        Ok(Value::Enum {
            variant: self.variant,
            value: OneValue::Ref(self.table.end_table()),
        })
    }
}

impl<'a, 'b> SerializeStructVariant for EnumBuilder<'a, 'b> {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()> where T: Serialize {
        SerializeTupleVariant::serialize_field(self, value)
    }
    fn end(self) -> Result<Value> {
        SerializeTupleVariant::end(self)
    }
}
