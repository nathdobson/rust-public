use core::marker::Sized;
use core::result::Result::Ok;

use flatbuffers::{UnionWIPOffset, WIPOffset};
use serde::ser::{Serialize, SerializeStruct, SerializeTuple, SerializeTupleStruct};

use crate::ser::error::Error;
use crate::ser::value::{OneValue, Value};
use crate::ser::wrapper::Serializer;
use crate::ser::Result;

pub struct TableBuilder<'a, 'b> {
    serializer: Serializer<'a, 'b>,
    element_start: usize,
}

impl<'a, 'b> TableBuilder<'a, 'b> {
    pub fn new(serializer: Serializer<'a, 'b>) -> Self {
        let len = serializer.stack.field_stack.len();
        TableBuilder {
            serializer,
            element_start: len,
        }
    }
    pub fn push(&mut self, value: Value) { self.serializer.stack.field_stack.push(value); }
    pub fn reborrow<'c>(&'c mut self) -> Serializer<'c, 'b> { self.serializer.reborrow() }
}

impl<'a, 'b> TableBuilder<'a, 'b> {
    pub fn end_table(self) -> WIPOffset<UnionWIPOffset> {
        let table = self.serializer.fbb.start_table();
        let mut off = 4;
        for element in self
            .serializer
            .stack
            .field_stack
            .drain(self.element_start..)
        {
            match element {
                Value::OneValue(element) => {
                    element.push_slot_always(self.serializer.fbb, off);
                    off += 2;
                }
                Value::Enum { variant, value } => {
                    self.serializer.fbb.push_slot_always(off, variant);
                    off += 2;
                    value.push_slot_always(self.serializer.fbb, off);
                    off += 2;
                }
            }
        }
        let table = self.serializer.fbb.end_table(table).as_union_value();
        table
    }
}

impl<'a, 'b> SerializeTuple for TableBuilder<'a, 'b> {
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        let value = value.serialize(self.serializer.reborrow())?;
        self.serializer.stack.field_stack.push(value);
        Ok(())
    }
    fn end(mut self) -> Result<Value> { Ok(Value::OneValue(OneValue::Ref(self.end_table()))) }
}

impl<'a, 'b> SerializeTupleStruct for TableBuilder<'a, 'b> {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        let value = value.serialize(self.serializer.reborrow())?;
        self.serializer.stack.field_stack.push(value);
        Ok(())
    }
    fn end(self) -> Result<Value> { Ok(Value::OneValue(OneValue::Ref(self.end_table()))) }
}

impl<'a, 'b> SerializeStruct for TableBuilder<'a, 'b> {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        SerializeTupleStruct::serialize_field(self, value)
    }
    fn end(self) -> Result<Value> { SerializeTupleStruct::end(self) }
}
