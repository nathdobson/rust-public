use flatbuffers::{FlatBufferBuilder, UnionWIPOffset, UOffsetT, WIPOffset};
use serde::Serialize;

use crate::{ser};
use crate::ser::enu::EnumBuilder;
use crate::ser::error;
use crate::ser::error::Error;
use crate::ser::map::MapBuilder;
use crate::ser::table::TableBuilder;
use crate::ser::value::{OneValue, Value};
use crate::ser::vector::VectorBuilder;
use crate::ser::Result;

pub struct Stack {
    pub field_stack: Vec<Value>,
    pub vector_stack: Vec<OneValue>,
}

impl Stack {
    pub fn new() -> Self {
        Stack { field_stack: vec![], vector_stack: vec![] }
    }
}

pub struct Serializer<'a, 'b> {
    pub fbb: &'a mut FlatBufferBuilder<'b>,
    pub stack: &'a mut Stack,
}

impl<'a, 'b> Serializer<'a, 'b> {
    pub fn new(fbb: &'a mut FlatBufferBuilder<'b>, stack: &'a mut Stack) -> Self {
        Serializer { fbb, stack }
    }
    pub fn reborrow<'c>(&'c mut self) -> Serializer<'c, 'b> {
        Serializer { fbb: self.fbb, stack: self.stack }
    }
    pub fn start_vector(mut self) -> VectorBuilder<'a, 'b> {
        VectorBuilder::new(self)
    }
    pub fn start_table(self) -> TableBuilder<'a, 'b> {
        TableBuilder::new(self)
    }
    pub fn serialize_to_offset<T: Serialize>(&mut self, v: &T) -> Result<WIPOffset<UnionWIPOffset>> {
        let value = v.serialize(self.reborrow())?;
        Ok(value.to_offset(self))
    }
}


impl<'a, 'b> serde::Serializer for Serializer<'a, 'b> {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = VectorBuilder<'a, 'b>;
    type SerializeTuple = TableBuilder<'a, 'b>;
    type SerializeTupleStruct = TableBuilder<'a, 'b>;
    type SerializeTupleVariant = EnumBuilder<'a, 'b>;
    type SerializeMap = MapBuilder<'a, 'b>;
    type SerializeStruct = TableBuilder<'a, 'b>;
    type SerializeStructVariant = EnumBuilder<'a, 'b>;
    fn serialize_bool(mut self, v: bool) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed8(if v { 1 } else { 0 })))
    }
    fn serialize_i8(mut self, v: i8) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed8(v as u8)))
    }
    fn serialize_i16(mut self, v: i16) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed16(v as u16)))
    }
    fn serialize_i32(mut self, v: i32) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed32(v as u32)))
    }
    fn serialize_i64(mut self, v: i64) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed64(v as u64)))
    }
    fn serialize_i128(mut self, v: i128) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed128(v as u128)))
    }
    fn serialize_u8(mut self, v: u8) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed8(v)))
    }
    fn serialize_u16(mut self, v: u16) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed16(v)))
    }
    fn serialize_u32(mut self, v: u32) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed32(v)))
    }
    fn serialize_u64(mut self, v: u64) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed64(v)))
    }
    fn serialize_u128(mut self, v: u128) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed128(v)))
    }
    fn serialize_f32(mut self, v: f32) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed32(v.to_bits())))
    }
    fn serialize_f64(mut self, v: f64) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed64(v.to_bits())))
    }
    fn serialize_char(mut self, v: char) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed32(v as u32)))
    }
    fn serialize_str(mut self, v: &str) -> Result<Self::Ok> {
        self.serialize_bytes(v.as_bytes())
    }
    fn serialize_bytes(mut self, v: &[u8]) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Ref(self.fbb.create_vector_direct(v).as_union_value())))
    }
    fn serialize_none(mut self) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::NoneRef))
    }
    fn serialize_some<T: ?Sized>(mut self, value: &T) -> Result<Self::Ok> where T: Serialize {
        let value = value.serialize(self.reborrow())?;
        let value = Value::OneValue(OneValue::SomeRef(value.to_offset(&mut self)));
        Ok(value)
    }
    fn serialize_unit(mut self) -> Result<Value> {
        Ok(Value::OneValue(OneValue::Fixed0))
    }
    fn serialize_unit_struct(mut self, name: &'static str) -> Result<Value> {
        Ok(Value::OneValue(OneValue::Fixed0))
    }
    fn serialize_unit_variant(mut self, name: &'static str, variant_index: u32, variant: &'static str) -> Result<Value> {
        Ok(Value::Enum { variant: variant_index as u16, value: OneValue::Fixed0 })
    }
    fn serialize_newtype_struct<T: ?Sized>(mut self, name: &'static str, value: &T) -> Result<Value> where T: Serialize {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized>(mut self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<Value> where T: Serialize {
        let value = value.serialize(self.reborrow())?;
        let value = value.to_one_value(&mut self);
        Ok(Value::Enum { variant: variant_index as u16, value })
    }
    fn serialize_seq(mut self, len: Option<usize>) -> Result<VectorBuilder<'a, 'b>> {
        Ok(self.start_vector())
    }
    fn serialize_tuple(mut self, len: usize) -> Result<TableBuilder<'a, 'b>> {
        Ok(self.start_table())
    }
    fn serialize_tuple_struct(mut self, name: &'static str, len: usize) -> Result<TableBuilder<'a, 'b>> {
        Ok(self.start_table())
    }
    fn serialize_tuple_variant(mut self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<EnumBuilder<'a, 'b>> {
        Ok(EnumBuilder::new(self, variant_index as u16))
    }
    fn serialize_map(mut self, len: Option<usize>) -> Result<MapBuilder<'a, 'b>> {
        Ok(MapBuilder::new(self))
    }
    fn serialize_struct(mut self, name: &'static str, len: usize) -> Result<TableBuilder<'a, 'b>> {
        Ok(self.start_table())
    }
    fn serialize_struct_variant(mut self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<EnumBuilder<'a, 'b>> {
        Ok(EnumBuilder::new(self, variant_index as u16))
    }
}
